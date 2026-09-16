//! ボタンはあったら押したくなる

use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::components::{BorderStyle, Text};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::platform::{Key, PointerButton};
use crate::theme::{
    Color, ControlAppearance, ControlVisualState, CornerRadius, ShadowStyle, Theme,
};
use crate::typography::{TextAlignment, TextRole};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor, ZStackAlignment};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonStyle {
    #[default]
    Standard,

    Primary,
    Accent,
    Ghost,
    Danger,

    Custom {
        background: Color,
        hovered_background: Color,
        border: Color,
        hovered_border: Color,
        foreground: Color,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonSize {
    Small,

    #[default]
    Medium,

    Large,
}

impl ButtonSize {
    const fn text_role(self) -> TextRole {
        match self {
            Self::Small => TextRole::Caption,
            Self::Medium | Self::Large => TextRole::Label,
        }
    }

    pub(crate) const fn height(self, theme: &Theme) -> f32 {
        match self {
            Self::Small => theme.layout.compact_control_height,
            Self::Medium => theme.layout.control_height,
            Self::Large => theme.layout.large_control_height,
        }
    }

    pub(crate) const fn icon_size(self, theme: &Theme) -> f32 {
        match self {
            Self::Small => theme.layout.compact_icon_size,
            Self::Medium => theme.layout.control_icon_size,
            Self::Large => theme.layout.stepper_icon_size,
        }
    }

    const fn horizontal_padding(self, theme: &Theme) -> f32 {
        match self {
            Self::Small => theme.spacing.small,
            Self::Medium => theme.button.horizontal_padding,
            Self::Large => theme.spacing.large,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ButtonColor {
    Surface,

    #[default]
    Accent,

    Destructive,

    Custom(Color),
}

impl ControlAppearance {
    fn with_opacity(self, opacity: f32) -> Self {
        Self {
            background: color_with_opacity(self.background, opacity),

            border: color_with_opacity(self.border, opacity),

            foreground: color_with_opacity(self.foreground, opacity),
        }
    }
}

impl ButtonStyle {
    fn resolve(self, theme: &Theme, state: ControlVisualState) -> ControlAppearance {
        match self {
            Self::Standard => theme.button.standard.resolve(state),
            Self::Primary => theme.button.primary.resolve(state),
            Self::Accent => theme.button.accent.resolve(state),
            Self::Ghost => theme.button.ghost.resolve(state),
            Self::Danger => theme.button.danger.resolve(state),

            Self::Custom {
                background,
                hovered_background,
                border,
                hovered_border,
                foreground,
            } => ControlAppearance {
                background: if matches!(
                    state,
                    ControlVisualState::Hovered | ControlVisualState::Pressed
                ) {
                    hovered_background
                } else {
                    background
                },

                border: if matches!(
                    state,
                    ControlVisualState::Hovered | ControlVisualState::Pressed
                ) {
                    hovered_border
                } else {
                    border
                },

                foreground,
            },
        }
    }

    pub fn foreground_color(self, theme: &Theme) -> Color {
        self.resolve(theme, ControlVisualState::Rest).foreground
    }
}

impl From<ButtonColor> for ButtonStyle {
    fn from(color: ButtonColor) -> Self {
        match color {
            ButtonColor::Surface => Self::Standard,
            ButtonColor::Accent => Self::Accent,
            ButtonColor::Destructive => Self::Danger,
            ButtonColor::Custom(color) => Self::Custom {
                background: color,
                hovered_background: color,
                border: color,
                hovered_border: color,
                foreground: Color::WHITE,
            },
        }
    }
}

fn color_with_opacity(color: Color, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };

    color.with_alpha((color.alpha as f32 * opacity).round() as u8)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct ButtonInteractionInner {
    hovered: bool,
    focused: bool,

    /*
     * このButton上でPrimaryボタンが
     * 押されたかを表します。
     */
    armed: bool,

    /*
     * armedかつ現在ポインターが
     * Button内にある場合にtrueです。
     */
    pressed: bool,

    clicked: bool,
    enabled: bool,
}

#[derive(Clone)]
pub struct ButtonInteractionState {
    inner: Rc<RefCell<ButtonInteractionInner>>,
}

impl Default for ButtonInteractionState {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ButtonInteractionInner {
                enabled: true,

                ..ButtonInteractionInner::default()
            })),
        }
    }
}

impl ButtonInteractionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_hovered(&self) -> bool {
        self.inner.borrow().hovered
    }

    pub fn is_pressed(&self) -> bool {
        self.inner.borrow().pressed
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.borrow().enabled
    }

    pub fn is_focused(&self) -> bool {
        self.inner.borrow().focused
    }

    pub fn take_clicked(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let clicked = inner.clicked;

        inner.clicked = false;

        clicked
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();

        inner.hovered = false;
        inner.focused = false;
        inner.armed = false;
        inner.pressed = false;
        inner.clicked = false;
    }

    fn set_enabled(&self, enabled: bool) -> bool {
        let mut inner = self.inner.borrow_mut();

        let changed = inner.enabled != enabled;

        inner.enabled = enabled;

        if !enabled {
            inner.hovered = false;
            inner.focused = false;
            inner.armed = false;
            inner.pressed = false;
        }

        changed
    }

    fn visual_state(&self) -> ControlVisualState {
        let inner = self.inner.borrow();

        if !inner.enabled {
            ControlVisualState::Disabled
        } else if inner.pressed {
            ControlVisualState::Pressed
        } else if inner.hovered {
            ControlVisualState::Hovered
        } else if inner.focused {
            ControlVisualState::Focused
        } else {
            ControlVisualState::Rest
        }
    }
}

#[allow(unused)]
pub struct Button {
    interaction: ButtonInteractionState,
    label: Option<String>,
    content: Option<StackChild>,
    intrinsic_label_size: RefCell<Option<Size>>,
    style: ButtonStyle,
    size: ButtonSize,
    radius: Option<CornerRadius>,
    shadow: ShadowStyle,
    alignment: ZStackAlignment,
    enabled: bool,
    accessibility_label: Option<String>,
    accessibility_value: Option<String>,
    accessibility_role: AccessibilityRole,
    accessibility_checked: Option<bool>,
    accessibility_selected: bool,
    on_click: Option<RefCell<Box<dyn FnMut()>>>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            interaction: ButtonInteractionState::new(),
            label: Some(label.into()),
            content: None,
            style: ButtonStyle::Standard,
            size: ButtonSize::Medium,
            radius: None,
            shadow: ShadowStyle::None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_role: AccessibilityRole::Button,
            accessibility_checked: None,
            accessibility_selected: false,
            on_click: None,
            intrinsic_label_size: RefCell::new(None),
        }
    }

    pub fn content<C>(mut self, content: C) -> Self
    where
        C: IntoStackChild,
    {
        self.label = None;
        self.content = Some(content.into_stack_child());

        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: ButtonColor) -> Self {
        self.style = ButtonStyle::from(color);

        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = Some(radius);
        self
    }

    pub fn shadow(mut self, shadow: ShadowStyle) -> Self {
        self.shadow = shadow;
        self
    }

    pub fn alignment(mut self, alignment: ZStackAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn accessibility_label_option(mut self, label: Option<String>) -> Self {
        self.accessibility_label = label;
        self
    }

    pub fn accessibility_value(mut self, value: impl Into<String>) -> Self {
        self.accessibility_value = Some(value.into());
        self
    }

    pub fn accessibility_value_option(mut self, value: Option<String>) -> Self {
        self.accessibility_value = value;
        self
    }

    pub fn accessibility_role(mut self, role: AccessibilityRole) -> Self {
        self.accessibility_role = role;
        self
    }

    pub fn accessibility_checked(mut self, checked: bool) -> Self {
        self.accessibility_checked = Some(checked);
        self
    }

    pub fn accessibility_selected(mut self, selected: bool) -> Self {
        self.accessibility_selected = selected;
        self
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    pub(crate) fn with_interaction(interaction: ButtonInteractionState) -> Self {
        Self {
            interaction,
            label: None,
            content: None,
            style: ButtonStyle::Standard,
            size: ButtonSize::Medium,
            radius: None,
            shadow: ShadowStyle::None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_role: AccessibilityRole::Button,
            accessibility_checked: None,
            accessibility_selected: false,
            on_click: None,
            intrinsic_label_size: RefCell::new(None),
        }
    }

    pub(crate) fn with_interaction_and_label(
        interaction: ButtonInteractionState,
        label: impl Into<String>,
    ) -> Self {
        let mut button = Self::with_interaction(interaction);
        button.label = Some(label.into());
        button
    }

    #[must_use]
    pub fn on_click(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_click = Some(RefCell::new(Box::new(callback)));
        self
    }

    fn label_text_alignment(&self) -> TextAlignment {
        match self.alignment {
            ZStackAlignment::TopLeading
            | ZStackAlignment::Leading
            | ZStackAlignment::BottomLeading => TextAlignment::Start,

            ZStackAlignment::Top | ZStackAlignment::Center | ZStackAlignment::Bottom => {
                TextAlignment::Center
            }

            ZStackAlignment::TopTrailing
            | ZStackAlignment::Trailing
            | ZStackAlignment::BottomTrailing => TextAlignment::End,
        }
    }

    fn label_vertical_factor(&self) -> f32 {
        match self.alignment {
            ZStackAlignment::TopLeading | ZStackAlignment::Top | ZStackAlignment::TopTrailing => {
                0.0
            }

            ZStackAlignment::Leading | ZStackAlignment::Center | ZStackAlignment::Trailing => 0.5,

            ZStackAlignment::BottomLeading
            | ZStackAlignment::Bottom
            | ZStackAlignment::BottomTrailing => 1.0,
        }
    }
}

impl View for Button {
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let horizontal_padding = self.size.horizontal_padding(context.theme);
        let label_role = self.size.text_role();
        let label_style = context.typography.style(label_role);

        self.interaction.set_enabled(self.enabled);

        let mut accessibility = AccessibilityNode::new(self.accessibility_role, bounds);
        accessibility.label = self
            .accessibility_label
            .clone()
            .or_else(|| self.label.clone());
        accessibility.value = self.accessibility_value.clone();
        accessibility.enabled = self.enabled;
        accessibility.focusable = true;
        accessibility.focused = self.interaction.inner.borrow().focused;
        accessibility.checked = self.accessibility_checked;
        accessibility.selected = self.accessibility_selected;
        context.record_accessibility(accessibility);

        let visual_state = self.interaction.visual_state();

        let mut appearance = self.style.resolve(context.theme, visual_state);

        if visual_state == ControlVisualState::Disabled {
            appearance = appearance.with_opacity(context.theme.button.disabled_opacity);
        }

        let shadow = if visual_state == ControlVisualState::Pressed {
            ShadowStyle::None
        } else {
            self.shadow
        };

        let radius = self.radius.unwrap_or_else(|| {
            context
                .inherited_corner_radius()
                .map(CornerRadius::Custom)
                .unwrap_or(context.theme.button.radius)
        });

        if self.interaction.inner.borrow().focused {
            let ring_width = context.theme.button.focus_ring_width;
            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.button.focus_ring))
                .radius(radius)
                .shadow(ShadowStyle::None)
                .border(BorderStyle::None)
                .paint(
                    Rect::new(
                        bounds.origin.x - ring_width,
                        bounds.origin.y - ring_width,
                        bounds.size.width + ring_width * 2.0,
                        bounds.size.height + ring_width * 2.0,
                    ),
                    context,
                );
        }

        Rectangle::new()
            .color(RectangleColor::Custom(appearance.background))
            .radius(radius)
            .shadow(shadow)
            .border(BorderStyle::custom(
                appearance.border,
                context.theme.button.stroke_width,
            ))
            .paint(bounds, context);

        if let Some(label) = self.label.as_ref() {
            let text_height = (label_style.line_height * context.text_measurer.font_scale())
                .min(bounds.size.height);

            let text_y =
                bounds.origin.y + (bounds.size.height - text_height) * self.label_vertical_factor();

            let text_bounds = Rect::new(
                bounds.origin.x + horizontal_padding,
                text_y,
                (bounds.size.width - horizontal_padding * 2.0).max(0.0),
                text_height,
            );

            Text::styled(label.as_str(), label_role)
                .accessibility_hidden(true)
                .alignment(self.label_text_alignment())
                .color(appearance.foreground)
                .paint(text_bounds, context);

            return;
        }

        let Some(content) = self.content.as_ref() else {
            return;
        };

        let content_size = content.overlay_size(bounds.size);

        let content_bounds = self.alignment.child_bounds(bounds, content_size);

        context
            .display_list
            .push(DrawCommand::PushClip { rect: bounds });

        content.paint(content_bounds, context);

        context.display_list.push(DrawCommand::PopClip);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let enabled_changed = self.interaction.set_enabled(self.enabled);

        if enabled_changed {
            context.request_redraw_in(bounds.expanded(16.0));
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let focused = target.is_some_and(|target| target == bounds);
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != focused;
                inner.focused = focused;
                if !focused {
                    inner.armed = false;
                    inner.pressed = false;
                }
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }
                EventResult::Ignored
            }

            ViewEvent::KeyPressed {
                key: Key::Enter | Key::Space,
                ..
            } if self.interaction.inner.borrow().focused => {
                self.interaction.inner.borrow_mut().clicked = true;
                if let Some(callback) = self.on_click.as_ref() {
                    (callback.borrow_mut())();
                }
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::PointerMoved { position } => {
                let mut inner = self.interaction.inner.borrow_mut();

                let hovered = bounds.contains(*position);

                let pressed = inner.armed && hovered;

                let changed = inner.hovered != hovered || inner.pressed != pressed;

                inner.hovered = hovered;
                inner.pressed = pressed;

                let armed = inner.armed;

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                if hovered || armed {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !bounds.contains(*position) {
                    return EventResult::Ignored;
                }

                let mut inner = self.interaction.inner.borrow_mut();

                inner.hovered = true;
                inner.armed = true;
                inner.pressed = true;

                drop(inner);

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let inside = bounds.contains(*position);

                let (was_armed, clicked, changed) = {
                    let mut inner = self.interaction.inner.borrow_mut();
                    let was_armed = inner.armed;
                    let clicked = was_armed && inside;
                    let changed =
                        inner.hovered != inside || inner.armed || inner.pressed || clicked;

                    inner.hovered = inside;
                    inner.armed = false;
                    inner.pressed = false;

                    if clicked {
                        inner.clicked = true;
                    }

                    (was_armed, clicked, changed)
                };

                if clicked && let Some(callback) = self.on_click.as_ref() {
                    (callback.borrow_mut())();
                }

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                if was_armed {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::PointerLeft => {
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.hovered || inner.armed || inner.pressed;

                inner.hovered = false;
                inner.armed = false;
                inner.pressed = false;

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            ViewEvent::FocusChanged { focused: false } => {
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.armed || inner.pressed || inner.focused;

                inner.armed = false;
                inner.pressed = false;
                inner.focused = false;

                drop(inner);

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let horizontal_padding = self.size.horizontal_padding(context.theme);
        let intrinsic_height = self.size.height(context.theme);
        let label_role = self.size.text_role();

        let width_is_fixed = constraints.minimum.width.is_finite()
            && constraints.maximum.width.is_finite()
            && (constraints.maximum.width - constraints.minimum.width).abs() <= 0.001;

        let height_is_fixed = constraints.minimum.height.is_finite()
            && constraints.maximum.height.is_finite()
            && (constraints.maximum.height - constraints.minimum.height).abs() <= 0.001;

        if width_is_fixed && height_is_fixed {
            return constraints.minimum;
        }

        if let Some(content) = self.content.as_ref() {
            return content.measure(constraints, context);
        }

        let width = if width_is_fixed {
            constraints.minimum.width
        } else if let Some(label) = self.label.as_ref() {
            let maximum_text_width = if constraints.maximum.width.is_finite() {
                (constraints.maximum.width - horizontal_padding * 2.0).max(0.0)
            } else {
                f32::INFINITY
            };

            let measured = Text::styled(label.as_str(), label_role).measure(
                Constraints::loose(Size::new(maximum_text_width, f32::INFINITY)),
                context,
            );

            measured.width + horizontal_padding * 2.0
        } else {
            0.0
        };

        let height = if height_is_fixed {
            constraints.minimum.height
        } else {
            intrinsic_height
        };

        constraints.constrain(Size::new(width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_sizes_follow_layout_tokens() {
        let theme = Theme::LIGHT;

        assert_eq!(
            ButtonSize::Small.height(&theme),
            theme.layout.compact_control_height
        );
        assert_eq!(
            ButtonSize::Medium.height(&theme),
            theme.layout.control_height
        );
        assert_eq!(
            ButtonSize::Large.height(&theme),
            theme.layout.large_control_height
        );
        assert_eq!(
            ButtonSize::Small.icon_size(&theme),
            theme.layout.compact_icon_size
        );
        assert_eq!(
            ButtonSize::Medium.icon_size(&theme),
            theme.layout.control_icon_size
        );
        assert_eq!(
            ButtonSize::Large.icon_size(&theme),
            theme.layout.stepper_icon_size
        );
    }
}

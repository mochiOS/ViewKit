//! ボタンはあったら押したくなる

use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::animation::AppearanceTransition;
use crate::components::{BorderStyle, Icon, SymbolName, Text};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::platform::{Key, PointerButton};
use crate::theme::{
    Color, ControlAppearance, ControlVisualState, CornerRadius, ShadowStyle, Theme,
};
use crate::typography::{TextAlignment, TextRole};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::omochi_shape::{OmochiPreset, OmochiShape};
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
    #[default]
    Small,

    Medium,

    Large,
}

impl ButtonSize {
    pub const fn control_size(self) -> crate::theme::ControlSize {
        match self {
            Self::Small => crate::theme::ControlSize::Small,
            Self::Medium => crate::theme::ControlSize::Medium,
            Self::Large => crate::theme::ControlSize::Large,
        }
    }
    const fn text_role(self) -> TextRole {
        match self {
            Self::Small => TextRole::Caption,
            Self::Medium | Self::Large => TextRole::Label,
        }
    }

    pub(crate) const fn height(self, theme: &Theme) -> f32 {
        self.control_size().height(&theme.controls)
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
            Self::Large => theme.button.horizontal_padding + theme.spacing.extra_small,
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
    focus_visible: bool,

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
    appearance: AppearanceTransition,
    keyboard_pressure: bool,
}

#[derive(Clone)]
pub struct ButtonInteractionState {
    inner: Rc<RefCell<ButtonInteractionInner>>,
    omochi: OmochiShape,
}

impl Default for ButtonInteractionState {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(ButtonInteractionInner {
                enabled: true,

                ..ButtonInteractionInner::default()
            })),
            omochi: OmochiShape::displacement(OmochiPreset::CompactControl),
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

    pub(crate) fn is_focus_visible(&self) -> bool {
        let inner = self.inner.borrow();
        inner.focused && inner.focus_visible
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
        inner.keyboard_pressure = false;
        drop(inner);
        self.omochi.reset();
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
    leading_symbol: Option<SymbolName>,
    content: Option<StackChild>,
    content_padding: bool,
    intrinsic_label_size: RefCell<Option<Size>>,
    style: ButtonStyle,
    size: ButtonSize,
    radius: Option<CornerRadius>,
    shadow: Option<ShadowStyle>,
    alignment: ZStackAlignment,
    enabled: bool,
    accessibility_label: Option<String>,
    accessibility_value: Option<String>,
    accessibility_role: AccessibilityRole,
    accessibility_checked: Option<bool>,
    accessibility_selected: bool,
    on_click: Option<RefCell<Box<dyn FnMut()>>>,
    omochi_enabled: bool,
    focus_ring_visible: bool,
    toolbar: bool,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            interaction: ButtonInteractionState::new(),
            label: Some(label.into()),
            leading_symbol: None,
            content: None,
            content_padding: true,
            style: ButtonStyle::Standard,
            size: ButtonSize::Small,
            radius: None,
            shadow: None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_role: AccessibilityRole::Button,
            accessibility_checked: None,
            accessibility_selected: false,
            on_click: None,
            omochi_enabled: true,
            focus_ring_visible: true,
            toolbar: false,
            intrinsic_label_size: RefCell::new(None),
        }
    }

    pub fn leading_symbol(mut self, symbol: SymbolName) -> Self {
        self.leading_symbol = Some(symbol);
        self
    }

    pub fn content_padding(mut self, enabled: bool) -> Self {
        self.content_padding = enabled;
        self
    }

    pub fn content<C>(mut self, content: C) -> Self
    where
        C: IntoStackChild,
    {
        self.label = None;
        self.leading_symbol = None;
        self.content = Some(content.into_stack_child());

        self
    }

    pub fn style(mut self, style: ButtonStyle) -> Self {
        self.style = style;
        self
    }

    /// Uses the common toolbar recipe: quiet at rest, compact symbols, and no
    /// standalone-button shadow or minimum label width. Interaction is retained.
    pub fn toolbar(mut self) -> Self {
        self.toolbar = true;
        self.style = ButtonStyle::Ghost;
        self.size = ButtonSize::Medium;
        self.shadow = Some(ShadowStyle::None);
        self
    }

    fn symbol_size(&self, theme: &Theme) -> f32 {
        if self.toolbar {
            theme.layout.compact_icon_size
        } else {
            self.size.icon_size(theme)
        }
    }

    fn horizontal_padding(&self, theme: &Theme) -> f32 {
        if self.toolbar {
            if self.label.as_ref().is_some_and(|label| label.is_empty()) {
                theme.spacing.extra_small
            } else {
                theme.spacing.small
            }
        } else {
            self.size.horizontal_padding(theme)
        }
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
        self.shadow = Some(shadow);
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

    /// Retained-state constructor kept for existing AppCore applications.
    pub fn with_interaction_and_label(
        interaction: ButtonInteractionState,
        label: impl Into<String>,
    ) -> Self {
        Self::with_interaction_state(label, interaction)
    }

    /// Creates a regular button with interaction state retained by the caller.
    /// This keeps omochi recovery continuous when the action rebuilds the view.
    pub fn with_interaction_state(
        label: impl Into<String>,
        interaction: ButtonInteractionState,
    ) -> Self {
        let mut button = Self::with_interaction(interaction);
        button.label = Some(label.into());
        button.size = ButtonSize::Small;
        button.omochi_enabled = true;
        button
    }

    /// Enables or disables omochi contour feedback for this button.
    pub fn omochi(mut self, enabled: bool) -> Self {
        self.omochi_enabled = enabled;
        self
    }

    pub(crate) fn focus_ring(mut self, visible: bool) -> Self {
        self.focus_ring_visible = visible;
        self
    }

    pub(crate) fn with_interaction(interaction: ButtonInteractionState) -> Self {
        Self {
            interaction,
            label: None,
            leading_symbol: None,
            content: None,
            content_padding: true,
            style: ButtonStyle::Standard,
            size: ButtonSize::Medium,
            radius: None,
            shadow: None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_role: AccessibilityRole::Button,
            accessibility_checked: None,
            accessibility_selected: false,
            on_click: None,
            omochi_enabled: false,
            focus_ring_visible: true,
            toolbar: false,
            intrinsic_label_size: RefCell::new(None),
        }
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

    fn local_point(bounds: Rect, point: Point) -> Point {
        Point::new(
            point.x - bounds.origin.x - bounds.size.width / 2.0,
            point.y - bounds.origin.y - bounds.size.height / 2.0,
        )
    }
}

impl View for Button {
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let horizontal_padding = self.horizontal_padding(context.theme);
        let label_role = self.size.text_role();
        let mut label_style = context.typography.style(label_role);
        if self.size == ButtonSize::Medium {
            label_style.size = context.theme.controls.font_size;
            label_style.line_height = context.theme.controls.line_height;
        }

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

        let (mut appearance, next) = self.interaction.inner.borrow_mut().appearance.sample(
            appearance,
            context.theme.motion.fast,
            std::time::Instant::now(),
            matches!(
                visual_state,
                ControlVisualState::Pressed | ControlVisualState::Disabled
            ),
        );
        if let Some(next) = next {
            context.request_redraw_in_at(bounds.expanded(16.0), next);
        }

        if self.omochi_enabled
            && self.interaction.omochi.is_animating()
            && appearance.background.alpha < 64
        {
            appearance.background = context.theme.colors.surface_muted;
        }

        let shadow = if let Some(shadow) = self.shadow {
            shadow
        } else if matches!(self.style, ButtonStyle::Ghost) {
            ShadowStyle::None
        } else if visual_state == ControlVisualState::Pressed {
            context.theme.button.pressed_shadow
        } else {
            context.theme.button.shadow
        };
        let omochi_shadow = if matches!(self.style, ButtonStyle::Ghost) {
            ShadowStyle::None
        } else if let Some(shadow) = self.shadow {
            shadow
        } else {
            context.theme.button.omochi_shadow
        };

        let radius = self.radius.unwrap_or_else(|| {
            context
                .inherited_corner_radius()
                .map(CornerRadius::Custom)
                .unwrap_or(context.theme.button.radius)
        });

        let interaction = self.interaction.inner.borrow();
        if interaction.focused && interaction.focus_visible && self.focus_ring_visible {
            let ring_width = context.theme.button.focus_ring_width;
            Rectangle::new()
                .color(RectangleColor::Custom(Color::TRANSPARENT))
                .radius(CornerRadius::Custom(
                    radius.resolve(&context.theme.radius, bounds.size.width, bounds.size.height)
                        + ring_width,
                ))
                .shadow(ShadowStyle::None)
                .border(BorderStyle::custom(
                    context.theme.button.focus_ring,
                    ring_width,
                ))
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
        drop(interaction);

        if self.omochi_enabled && self.interaction.omochi.is_animating() {
            let resolved_radius =
                radius.resolve(&context.theme.radius, bounds.size.width, bounds.size.height);
            self.interaction.omochi.paint_styled(
                bounds,
                resolved_radius,
                appearance.background,
                appearance.border,
                context.theme.button.stroke_width,
                omochi_shadow,
                context,
            );
        } else {
            Rectangle::new()
                .color(RectangleColor::Custom(appearance.background))
                .radius(radius)
                .shadow(shadow)
                .border(BorderStyle::custom(
                    appearance.border,
                    context.theme.button.stroke_width,
                ))
                .paint(bounds, context);
        }

        if let Some(label) = self.label.as_ref() {
            if let Some(symbol) = self.leading_symbol {
                let single_line = label.replace(['\n', '\r'], " ");
                let label = &single_line;
                let inner = Rect::new(
                    bounds.origin.x + horizontal_padding,
                    bounds.origin.y,
                    (bounds.size.width - 2.0 * horizontal_padding).max(0.0),
                    bounds.size.height,
                );
                let icon_size = self
                    .symbol_size(context.theme)
                    .min(inner.size.width)
                    .min(inner.size.height);
                let gap = if label.is_empty() {
                    0.0
                } else {
                    context
                        .theme
                        .spacing
                        .extra_small
                        .min((inner.size.width - icon_size).max(0.0))
                };
                let available = (inner.size.width - icon_size - gap).max(0.0);
                let text = Text::styled(label.as_str(), label_role)
                    .font_size(label_style.size)
                    .line_height(label_style.line_height);
                let natural = text
                    .measure_unbounded_with_typography(context.text_measurer, context.typography);
                let row = Size::new(
                    (icon_size + gap + natural.width).min(inner.size.width),
                    inner.size.height,
                );
                let row = self.alignment.child_bounds(inner, row);
                let icon_bounds = Rect::new(
                    row.origin.x,
                    row.origin.y + (row.size.height - icon_size) * self.label_vertical_factor(),
                    icon_size,
                    icon_size,
                );
                let height = (label_style.line_height * context.text_measurer.font_scale())
                    .min(row.size.height);
                let text_bounds = Rect::new(
                    row.origin.x + icon_size + gap,
                    row.origin.y + (row.size.height - height) * self.label_vertical_factor(),
                    available.min(natural.width),
                    height,
                );
                let fitted = fit_symbol_label(
                    label,
                    available,
                    label_role,
                    label_style.size,
                    label_style.line_height,
                    context,
                );
                context
                    .display_list
                    .push(DrawCommand::PushClip { rect: inner });
                Icon::new(symbol)
                    .size(icon_size)
                    .color(appearance.foreground)
                    .paint(icon_bounds, context);
                Text::styled(fitted, label_role)
                    .font_size(label_style.size)
                    .line_height(label_style.line_height)
                    .accessibility_hidden(true)
                    .color(appearance.foreground)
                    .paint(text_bounds, context);
                context.display_list.push(DrawCommand::PopClip);
                return;
            }
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
                .font_size(label_style.size)
                .line_height(label_style.line_height)
                .accessibility_hidden(true)
                .alignment(self.label_text_alignment())
                .color(appearance.foreground)
                .paint(text_bounds, context);

            return;
        }

        let Some(content) = self.content.as_ref() else {
            return;
        };

        let inset = if self.content_padding {
            horizontal_padding.min(bounds.size.width.max(0.0) / 2.0)
        } else {
            0.0
        };
        let inner = Rect::new(
            bounds.origin.x + inset,
            bounds.origin.y,
            (bounds.size.width - inset * 2.0).max(0.0),
            bounds.size.height,
        );
        if inner.size.width <= 0.0 || inner.size.height <= 0.0 {
            return;
        }
        let content_size = content.overlay_size(inner.size);
        let content_bounds = self.alignment.child_bounds(inner, content_size);

        context.display_list.push(DrawCommand::PushRoundedClip {
            rect: bounds,
            radius: radius.resolve(&context.theme.radius, bounds.size.width, bounds.size.height),
        });

        if self.content_padding {
            context
                .display_list
                .push(DrawCommand::PushClip { rect: inner });
        }
        context.with_control_foreground(appearance.foreground, |context| {
            content.paint(content_bounds, context);
        });
        if self.content_padding {
            context.display_list.push(DrawCommand::PopClip);
        }
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
            if self.interaction.omochi.is_animating() {
                self.interaction.omochi.reset();
            }
            return EventResult::Ignored;
        }

        if self.omochi_enabled {
            match event {
                ViewEvent::KeyPressed {
                    key: Key::Enter | Key::Space,
                    ..
                } if self.interaction.is_focused()
                    && !self.interaction.inner.borrow().keyboard_pressure =>
                {
                    self.interaction.omochi.begin(Point::new(0.0, 0.0));
                    self.interaction.inner.borrow_mut().keyboard_pressure = true;
                }
                ViewEvent::KeyReleased {
                    key: Key::Enter | Key::Space,
                    ..
                } if self.interaction.inner.borrow().keyboard_pressure => {
                    self.interaction.omochi.end();
                    self.interaction.inner.borrow_mut().keyboard_pressure = false;
                }
                ViewEvent::PointerPressed {
                    position,
                    button: PointerButton::Primary,
                } if bounds.contains(*position) => self
                    .interaction
                    .omochi
                    .begin(Self::local_point(bounds, *position)),
                ViewEvent::PointerMoved { position } if self.interaction.omochi.is_active() => self
                    .interaction
                    .omochi
                    .moved(Self::local_point(bounds, *position)),
                ViewEvent::PointerReleased {
                    button: PointerButton::Primary,
                    ..
                }
                | ViewEvent::PointerLeft
                | ViewEvent::FocusChanged { focused: false }
                    if self.interaction.omochi.is_active() =>
                {
                    self.interaction.omochi.end();
                }
                _ => {}
            }
            if self.interaction.omochi.is_animating() {
                context.request_redraw_in(bounds.expanded(24.0));
            }
        }

        match event {
            ViewEvent::FocusVisibilityChanged { visible } => {
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focus_visible != *visible;
                inner.focus_visible = *visible;
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }
                EventResult::Ignored
            }
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
        let horizontal_padding = self.horizontal_padding(context.theme);
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
            let padding = if self.content_padding {
                horizontal_padding * 2.0
            } else {
                0.0
            };
            let child = Constraints::new(
                Size::new(
                    (constraints.minimum.width - padding).max(0.0),
                    constraints.minimum.height,
                ),
                Size::new(
                    (constraints.maximum.width - padding).max(0.0),
                    constraints.maximum.height,
                ),
            );
            let size = content.measure(child, context);
            return constraints.constrain(Size::new(size.width + padding, size.height));
        }

        let width = if width_is_fixed {
            constraints.minimum.width
        } else if let Some(label) = self.label.as_ref() {
            let label = if self.leading_symbol.is_some() {
                std::borrow::Cow::Owned(label.replace(['\n', '\r'], " "))
            } else {
                std::borrow::Cow::Borrowed(label.as_str())
            };
            let symbol_width = self.leading_symbol.map_or(0.0, |_| {
                self.symbol_size(context.theme)
                    + if label.is_empty() {
                        0.0
                    } else {
                        context.theme.spacing.extra_small
                    }
            });
            let maximum_text_width = if constraints.maximum.width.is_finite() {
                (constraints.maximum.width - horizontal_padding * 2.0 - symbol_width).max(0.0)
            } else {
                f32::INFINITY
            };

            let text = Text::styled(label.as_ref(), label_role);
            let text = if self.size == ButtonSize::Medium {
                text.font_size(context.theme.controls.font_size)
                    .line_height(context.theme.controls.line_height)
            } else {
                text
            };
            let measured = text.measure(
                Constraints::loose(Size::new(maximum_text_width, f32::INFINITY)),
                context,
            );

            (measured.width + symbol_width + horizontal_padding * 2.0).max(
                if self.toolbar && label.is_empty() {
                    self.size.height(context.theme)
                } else if self.size == ButtonSize::Small || self.toolbar {
                    0.0
                } else {
                    context.theme.controls.button_min_width
                },
            )
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

fn fit_symbol_label(
    label: &str,
    width: f32,
    role: TextRole,
    size: f32,
    line_height: f32,
    context: &mut PaintContext<'_>,
) -> String {
    let label = label.replace(['\n', '\r'], " ");
    let measure = |value: &str, c: &mut PaintContext<'_>| {
        Text::styled(value, role)
            .font_size(size)
            .line_height(line_height)
            .measure_unbounded_with_typography(c.text_measurer, c.typography)
            .width
    };
    if measure(&label, context) <= width {
        return label;
    }
    if measure("…", context) > width {
        return String::new();
    }
    let boundaries: Vec<_> = label
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(label.len()))
        .collect();
    let mut low = 0;
    let mut high = boundaries.len() - 1;
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate = format!("{}…", &label[..boundaries[mid]]);
        if measure(&candidate, context) <= width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    format!("{}…", &label[..boundaries[low]])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn semantic_sizes_follow_layout_tokens() {
        let theme = Theme::LIGHT;

        assert_eq!(
            ButtonSize::Small.height(&theme),
            theme.controls.small_height
        );
        assert_eq!(ButtonSize::Medium.height(&theme), theme.controls.height);
        assert_eq!(
            ButtonSize::Large.height(&theme),
            theme.controls.large_height
        );
        assert_eq!(
            ButtonSize::Small.icon_size(&theme),
            theme.layout.compact_icon_size
        );
        assert_eq!(
            ButtonSize::Medium.icon_size(&theme),
            theme.layout.control_icon_size
        );
        assert_eq!(ButtonSize::Small.height(&theme), 24.0);
        assert_eq!(ButtonSize::Medium.height(&theme), 30.0);
        assert_eq!(ButtonSize::Large.height(&theme), 32.0);
        assert_eq!(
            ButtonSize::Large.icon_size(&theme),
            theme.layout.stepper_icon_size
        );
    }

    #[test]
    fn public_buttons_default_to_small() {
        assert_eq!(ButtonSize::default(), ButtonSize::Small);
        assert_eq!(Button::new("Default").size, ButtonSize::Small);
        assert_eq!(
            Button::with_interaction_state("Retained", ButtonInteractionState::new()).size,
            ButtonSize::Small
        );
    }

    #[test]
    fn retained_button_paints_omochi_contour_after_click_is_consumed() {
        let interaction = ButtonInteractionState::new();
        let button = Button::with_interaction_state("Continue", interaction.clone());
        let bounds = Rect::new(40.0, 30.0, 104.0, 28.0);
        let mut text_measurer = TextMeasurer::new();
        let mut event_context =
            EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut text_measurer);

        assert_eq!(
            button.handle_event(
                bounds,
                &ViewEvent::PointerPressed {
                    position: Point::new(130.0, 44.0),
                    button: PointerButton::Primary,
                },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert_eq!(
            button.handle_event(
                bounds,
                &ViewEvent::PointerMoved {
                    position: Point::new(142.0, 44.0),
                },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert_eq!(
            button.handle_event(
                bounds,
                &ViewEvent::PointerReleased {
                    position: Point::new(142.0, 44.0),
                    button: PointerButton::Primary,
                },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert!(interaction.take_clicked());

        let rebuilt = Button::with_interaction_state("Continue", interaction);
        let mut display_list = DisplayList::new();
        let mut paint_context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        rebuilt.paint(bounds, &mut paint_context);

        let polygon_colors = display_list
            .commands()
            .iter()
            .filter_map(|command| match command {
                DrawCommand::FillPolygon { color, .. } => Some(*color),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            polygon_colors.len() > 2,
            "deformed button must retain shadow, border, and fill layers"
        );
        assert!(polygon_colors.contains(&Theme::LIGHT.button.standard.hovered.border));
        assert_eq!(
            polygon_colors.last(),
            Some(&Theme::LIGHT.button.standard.hovered.background)
        );
    }

    #[test]
    fn focus_ring_is_visible_only_for_keyboard_modality() {
        let interaction = ButtonInteractionState::new();
        let button = Button::with_interaction_state("Continue", interaction);
        let bounds = Rect::new(40.0, 30.0, 104.0, 30.0);
        let mut text_measurer = TextMeasurer::new();
        let mut event_context =
            EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut text_measurer);

        button.handle_event(
            bounds,
            &ViewEvent::FocusVisibilityChanged { visible: false },
            &mut event_context,
        );
        button.handle_event(
            bounds,
            &ViewEvent::KeyboardFocusRequested {
                bounds: Some(bounds),
            },
            &mut event_context,
        );
        drop(event_context);
        let mut pointer_list = DisplayList::new();
        let mut pointer_context = PaintContext::new(
            &mut pointer_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        button.paint(bounds, &mut pointer_context);
        let pointer_strokes = pointer_list
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    DrawCommand::StrokeRoundedRect { .. }
                        | DrawCommand::StrokeInsideRoundedRect { .. }
                )
            })
            .count();

        {
            let mut event_context =
                EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut text_measurer);
            button.handle_event(
                bounds,
                &ViewEvent::FocusVisibilityChanged { visible: true },
                &mut event_context,
            );
        }
        let mut keyboard_list = DisplayList::new();
        let mut keyboard_context = PaintContext::new(
            &mut keyboard_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        button.paint(bounds, &mut keyboard_context);
        let keyboard_strokes = keyboard_list
            .commands()
            .iter()
            .filter(|command| {
                matches!(
                    command,
                    DrawCommand::StrokeRoundedRect { .. }
                        | DrawCommand::StrokeInsideRoundedRect { .. }
                )
            })
            .count();

        assert_eq!(keyboard_strokes, pointer_strokes + 1);
    }
}

#[cfg(test)]
mod compatibility_tests {
    use super::*;

    struct ContentProbe(Rc<RefCell<Option<Rect>>>);
    impl View for ContentProbe {
        fn measure(&self, c: Constraints, _: &mut MeasureContext<'_>) -> Size {
            c.constrain(Size::new(42.0, 18.0))
        }
        fn paint(&self, b: Rect, _: &mut PaintContext<'_>) {
            *self.0.borrow_mut() = Some(b);
        }
    }
    #[test]
    fn custom_content_uses_shared_size_padding_in_measurement_paint_and_clip() {
        let mut theme = Theme::LIGHT;
        theme.spacing.small = 10.0;
        theme.button.horizontal_padding = 13.0;
        for size in [ButtonSize::Small, ButtonSize::Medium, ButtonSize::Large] {
            for hovered in [false, true] {
                let painted = Rc::new(RefCell::new(None));
                let button = Button::new("")
                    .size(size)
                    .content(ContentProbe(painted.clone()));
                button.interaction.inner.borrow_mut().hovered = hovered;
                let pad = size.horizontal_padding(&theme);
                let mut measurer = crate::typography::TextMeasurer::new();
                let natural = button.measure(
                    Constraints::loose(Size::new(200.0, 60.0)),
                    &mut MeasureContext {
                        theme: &theme,
                        typography: &theme.typography,
                        text_measurer: &mut measurer,
                    },
                );
                assert_eq!(natural, Size::new(42.0 + pad * 2.0, 18.0));
                let bounds = Rect::new(20.0, 12.0, 120.0, 32.0);
                let (list, nodes) = render(&button, &theme, bounds);
                let child = painted.borrow().unwrap();
                assert_eq!(child.origin.x, bounds.origin.x + pad);
                assert_eq!(child.size.width, bounds.size.width - 2.0 * pad);
                assert!(list.commands().iter().any(|command| matches!(command,
                    DrawCommand::PushClip { rect } if rect.origin.x == bounds.origin.x + pad
                        && rect.size.width == bounds.size.width - 2.0 * pad)));
                assert_eq!(nodes[0].bounds, bounds);
            }
        }
    }
    #[test]
    fn owned_content_layout_can_opt_out_without_changing_label_padding() {
        let theme = Theme::LIGHT;
        let painted = Rc::new(RefCell::new(None));
        let button = Button::new("")
            .content_padding(false)
            .content(ContentProbe(painted.clone()));
        let bounds = Rect::new(20.0, 12.0, 120.0, 32.0);
        render(&button, &theme, bounds);
        assert_eq!(*painted.borrow(), Some(bounds));
        let mut measurer = crate::typography::TextMeasurer::new();
        let natural = button.measure(
            Constraints::loose(Size::new(200.0, 60.0)),
            &mut MeasureContext {
                theme: &theme,
                typography: &theme.typography,
                text_measurer: &mut measurer,
            },
        );
        assert_eq!(natural, Size::new(42.0, 18.0));
        let (list, _) = render(
            &Button::new("Apply")
                .size(ButtonSize::Medium)
                .content_padding(false),
            &theme,
            bounds,
        );
        assert!(list.commands().iter().any(|command| matches!(command,
            DrawCommand::DrawText { command } if command.bounds.origin.x == bounds.origin.x + theme.button.horizontal_padding)));
    }
    #[test]
    fn narrow_custom_content_does_not_escape_and_outer_edges_remain_clickable() {
        use crate::geometry::Point;
        use crate::platform::PointerButton;
        use std::cell::Cell;
        let theme = Theme::LIGHT;
        let painted = Rc::new(RefCell::new(None));
        let clicked = Rc::new(Cell::new(false));
        let callback = clicked.clone();
        let button = Button::new("")
            .size(ButtonSize::Small)
            .content(ContentProbe(painted.clone()))
            .on_click(move || callback.set(true));
        let bounds = Rect::new(0.0, 0.0, 4.0, 30.0);
        render(&button, &theme, bounds);
        assert!(painted.borrow().is_none());
        let mut measurer = crate::typography::TextMeasurer::new();
        let mut ctx = EventContext::new(&theme, &theme.typography, &mut measurer);
        for event in [
            ViewEvent::PointerPressed {
                position: Point::new(2.0, 15.0),
                button: PointerButton::Primary,
            },
            ViewEvent::PointerReleased {
                position: Point::new(2.0, 15.0),
                button: PointerButton::Primary,
            },
        ] {
            button.handle_event(bounds, &event, &mut ctx);
        }
        assert!(clicked.get());
    }
    #[test]
    fn semantic_sizes_follow_shared_control_metrics() {
        let theme = Theme::LIGHT;

        assert_eq!(
            ButtonSize::Small.height(&theme),
            theme.controls.small_height
        );
        assert_eq!(ButtonSize::Medium.height(&theme), theme.controls.height);
        assert_eq!(
            ButtonSize::Large.height(&theme),
            theme.controls.large_height
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

    fn render(
        button: &Button,
        theme: &Theme,
        bounds: Rect,
    ) -> (crate::draw_command::DisplayList, Vec<AccessibilityNode>) {
        let mut measurer = crate::typography::TextMeasurer::new();
        let mut list = crate::draw_command::DisplayList::new();
        let mut nodes = vec![];
        button.paint(
            bounds,
            &mut PaintContext::new(&mut list, theme, &theme.typography, &mut measurer)
                .with_accessibility_nodes(&mut nodes),
        );
        (list, nodes)
    }
    #[test]
    fn symbol_labels_have_shared_padding_centering_and_one_accessible_button() {
        let theme = Theme::LIGHT;
        let bounds = Rect::new(20.0, 12.0, 176.0, 30.0);
        let button = Button::new("New Message")
            .size(ButtonSize::Small)
            .leading_symbol(SymbolName::Pencil);
        let (list, nodes) = render(&button, &theme, bounds);
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].label.as_deref(), Some("New Message"));
        assert_eq!(nodes[0].bounds, bounds);
        let text = list
            .commands()
            .iter()
            .find_map(|c| match c {
                DrawCommand::DrawText { command } => Some(command),
                _ => None,
            })
            .unwrap();
        let icon = button.size.icon_size(&theme);
        let gap = theme.spacing.extra_small;
        let left = text.bounds.origin.x - icon - gap;
        let right = text.bounds.origin.x + text.bounds.size.width;
        assert!((left + right - (bounds.origin.x * 2.0 + bounds.size.width)).abs() < 0.1);
        assert!(left >= bounds.origin.x + button.size.horizontal_padding(&theme));
        assert!(
            (text.bounds.origin.y + text.bounds.size.height / 2.0
                - (bounds.origin.y + bounds.size.height / 2.0))
                .abs()
                < 0.1
        );
        assert_eq!(
            text.font_size,
            theme.typography.style(TextRole::Caption).size
        );
    }
    #[test]
    fn symbol_and_label_share_state_colors_in_both_themes() {
        for theme in [Theme::LIGHT, Theme::DARK] {
            for style in [
                ButtonStyle::Standard,
                ButtonStyle::Primary,
                ButtonStyle::Accent,
                ButtonStyle::Ghost,
                ButtonStyle::Danger,
            ] {
                for enabled in [true, false] {
                    let button = Button::new("Apply")
                        .leading_symbol(SymbolName::Pencil)
                        .style(style)
                        .enabled(enabled);
                    let (list, _) = render(&button, &theme, Rect::new(0.0, 0.0, 140.0, 30.0));
                    let text = list
                        .commands()
                        .iter()
                        .find_map(|c| match c {
                            DrawCommand::DrawText { command } => Some(command.color),
                            _ => None,
                        })
                        .unwrap();
                    let tint = list
                        .commands()
                        .iter()
                        .find_map(|c| match c {
                            DrawCommand::DrawSvg { command } => command.tint,
                            _ => None,
                        })
                        .unwrap();
                    assert_eq!(text, tint);
                    let expected = style.resolve(&theme, ControlVisualState::Rest);
                    assert_eq!(
                        text,
                        if enabled {
                            expected.foreground
                        } else {
                            expected
                                .with_opacity(theme.button.disabled_opacity)
                                .foreground
                        }
                    );
                }
            }
        }
    }
    #[test]
    fn symbol_measurement_includes_padding_and_constrained_labels_stay_on_one_line() {
        let theme = Theme::LIGHT;
        let mut measurer = crate::typography::TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut measurer,
        };
        let constraints = Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY));
        let plain = Button::new("New Message")
            .size(ButtonSize::Small)
            .measure(constraints, &mut context);
        let symbol = Button::new("New Message")
            .size(ButtonSize::Small)
            .leading_symbol(SymbolName::Pencil)
            .measure(constraints, &mut context);
        assert!(
            (symbol.width
                - plain.width
                - ButtonSize::Small.icon_size(&theme)
                - theme.spacing.extra_small)
                .abs()
                < 0.1
        );
        assert_eq!(symbol.height, plain.height);
        for scale in [1.0, 2.0] {
            let button = Button::new("Very long 日本語 label\nNew Message")
                .size(ButtonSize::Small)
                .leading_symbol(SymbolName::Pencil);
            let mut measurer = crate::typography::TextMeasurer::new();
            measurer.set_font_scale(scale);
            let mut list = crate::draw_command::DisplayList::new();
            button.paint(
                Rect::new(0.0, 0.0, 100.0, 30.0),
                &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut measurer),
            );
            let text = list
                .commands()
                .iter()
                .find_map(|c| match c {
                    DrawCommand::DrawText { command } => Some(command),
                    _ => None,
                })
                .unwrap();
            assert!(text.text.ends_with('…'));
            assert!(!text.text.contains('\n'));
            assert!(
                text.bounds.origin.x + text.bounds.size.width
                    <= 100.0 - ButtonSize::Small.horizontal_padding(&theme) + 0.1
            );
        }
    }
    #[test]
    fn plain_and_custom_content_buttons_keep_existing_layout_contract() {
        let plain = Button::new("Apply").size(ButtonSize::Medium);
        assert!(plain.leading_symbol.is_none());
        let custom = Button::new("Apply")
            .leading_symbol(SymbolName::Pencil)
            .content(Text::label("Custom"));
        assert!(custom.leading_symbol.is_none());
        assert!(custom.label.is_none());
        let theme = Theme::LIGHT;
        let bounds = Rect::new(0.0, 0.0, 110.0, 30.0);
        let (list, _) = render(&plain, &theme, bounds);
        let text = list
            .commands()
            .iter()
            .find_map(|c| match c {
                DrawCommand::DrawText { command } => Some(command),
                _ => None,
            })
            .unwrap();
        assert_eq!(text.bounds.origin.x, theme.button.horizontal_padding);
        assert_eq!(
            text.bounds.size.width,
            110.0 - 2.0 * theme.button.horizontal_padding
        );
    }
}

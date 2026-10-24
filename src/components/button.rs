//! ボタンはあったら押したくなる

use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::animation::AppearanceTransition;
use crate::components::{BorderStyle, Icon, SymbolName, Text};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::omochi::{OmochiMotion, OmochiSurface, button_contour};
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
            Self::Small => theme.controls.small_height,
            Self::Medium => theme.controls.height,
            Self::Large => theme.controls.large_height,
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
    appearance: AppearanceTransition,
    surface: OmochiSurface,
    keyboard_pressure: bool,

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
        inner.focus_visible = false;
        inner.armed = false;
        inner.pressed = false;
        inner.clicked = false;
        inner.surface = OmochiSurface::default();
        inner.keyboard_pressure = false;
    }

    fn set_enabled(&self, enabled: bool) -> bool {
        let mut inner = self.inner.borrow_mut();

        let changed = inner.enabled != enabled;

        inner.enabled = enabled;

        if !enabled {
            inner.hovered = false;
            inner.focused = false;
            inner.focus_visible = false;
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
    intrinsic_label_size: RefCell<Option<Size>>,
    style: ButtonStyle,
    size: ButtonSize,
    radius: Option<CornerRadius>,
    shadow: ShadowStyle,
    alignment: ZStackAlignment,
    enabled: bool,
    omochi: bool,
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
            leading_symbol: None,
            content: None,
            style: ButtonStyle::Standard,
            size: ButtonSize::Medium,
            radius: None,
            shadow: ShadowStyle::None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            omochi: true,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_role: AccessibilityRole::Button,
            accessibility_checked: None,
            accessibility_selected: false,
            on_click: None,
            intrinsic_label_size: RefCell::new(None),
        }
    }

    /// 独立した操作面のomochi反応を有効にします。Ghost/Customには適用しません。
    pub fn omochi(mut self, enabled: bool) -> Self {
        self.omochi = enabled;
        self
    }

    fn omochi_motion(&self, theme: &Theme, bounds: Rect) -> OmochiMotion {
        let mut motion = theme.motion.omochi;
        motion.enabled &= self.enabled && self.omochi_surface_enabled();
        motion.material = motion.material.for_height(bounds.size.height);
        motion
    }

    fn omochi_surface_enabled(&self) -> bool {
        self.omochi
            && matches!(
                self.style,
                ButtonStyle::Standard
                    | ButtonStyle::Primary
                    | ButtonStyle::Accent
                    | ButtonStyle::Danger
            )
    }

    /// テキストラベルの先頭にVK Symbolを配置します。
    /// 寸法・余白・文字サイズ・状態色はButtonの共通規則に従います。
    /// `.content()`による任意の子Viewには適用しません。
    pub fn leading_symbol(mut self, symbol: SymbolName) -> Self {
        self.leading_symbol = Some(symbol);
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
            leading_symbol: None,
            content: None,
            style: ButtonStyle::Standard,
            size: ButtonSize::Medium,
            radius: None,
            shadow: ShadowStyle::None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            omochi: true,
            accessibility_label: None,
            accessibility_value: None,
            accessibility_role: AccessibilityRole::Button,
            accessibility_checked: None,
            accessibility_selected: false,
            on_click: None,
            intrinsic_label_size: RefCell::new(None),
        }
    }

    /// View再構築後もホバー・押下・色の遷移を保つ共有操作状態で作成します。
    pub fn with_interaction_and_label(
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
        let (appearance, next) = self.interaction.inner.borrow_mut().appearance.sample(
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

        let interaction = self.interaction.inner.borrow();
        if interaction.focused && interaction.focus_visible {
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
        drop(interaction);

        let motion = self.omochi_motion(context.theme, bounds);
        let sample = self
            .interaction
            .inner
            .borrow_mut()
            .surface
            .sample(motion, std::time::Instant::now());
        if let Some(next) = sample.next_redraw_at {
            context.request_redraw_in_at(bounds.expanded(motion.material.max_pull + 6.0), next);
        }
        if sample.press_depth > 0.001 || sample.pull.x.hypot(sample.pull.y) > 0.001 {
            if shadow != ShadowStyle::None {
                Rectangle::new()
                    .color(RectangleColor::Custom(Color::TRANSPARENT))
                    .radius(radius)
                    .shadow(shadow)
                    .paint(bounds, context);
            }
            context.display_list.push(DrawCommand::DrawContour {
                points: button_contour(
                    bounds,
                    radius.resolve(&context.theme.radius, bounds.size.width, bounds.size.height),
                    sample,
                    motion.material,
                ),
                fill: appearance.background,
                stroke: appearance.border,
                stroke_width: context.theme.button.stroke_width,
            });
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
                    .size
                    .icon_size(context.theme)
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

        let content_size = content.overlay_size(bounds.size);

        let content_bounds = self.alignment.child_bounds(bounds, content_size);

        context.display_list.push(DrawCommand::PushRoundedClip {
            rect: bounds,
            radius: radius.resolve(&context.theme.radius, bounds.size.width, bounds.size.height),
        });

        content.paint(content_bounds, context);

        context.display_list.push(DrawCommand::PopClip);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let radius = self
            .radius
            .unwrap_or_else(|| {
                context
                    .inherited_corner_radius()
                    .map(CornerRadius::Custom)
                    .unwrap_or(context.theme.button.radius)
            })
            .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);
        let hit = |p| bounds.contains_rounded(radius, p);
        let enabled_changed = self.interaction.set_enabled(self.enabled);

        if enabled_changed {
            context.request_redraw_in(bounds.expanded(16.0));
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        let motion = self.omochi_motion(context.theme(), bounds);
        let local = |point: crate::geometry::Point| {
            crate::geometry::Point::new(point.x - bounds.origin.x, point.y - bounds.origin.y)
        };
        if motion.enabled {
            let now = std::time::Instant::now();
            let mut inner = self.interaction.inner.borrow_mut();
            match event {
                ViewEvent::PointerPressed {
                    position,
                    button: PointerButton::Primary,
                } if hit(*position) => {
                    inner.surface.begin(local(*position), now);
                    inner.keyboard_pressure = false;
                }
                ViewEvent::PointerMoved { position } => {
                    inner.surface.move_to(local(*position), now)
                }
                ViewEvent::PointerReleased {
                    button: PointerButton::Primary,
                    ..
                }
                | ViewEvent::FocusChanged { focused: false } => {
                    inner.surface.end(motion, now);
                    inner.keyboard_pressure = false;
                }
                ViewEvent::KeyReleased {
                    key: Key::Enter | Key::Space,
                    ..
                } if inner.keyboard_pressure => {
                    inner.surface.end(motion, now);
                    inner.keyboard_pressure = false;
                    context.request_redraw_in(bounds.expanded(motion.material.max_pull + 6.0));
                }
                _ => {}
            }
        }
        match event {
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let focused = target.is_some_and(|target| target == bounds);
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != focused
                    || inner.focus_visible != (focused && context.focus_visible());
                inner.focused = focused;
                inner.focus_visible = focused && context.focus_visible();
                if !focused {
                    inner.armed = false;
                    inner.pressed = false;
                    inner.surface.end(motion, std::time::Instant::now());
                    inner.keyboard_pressure = false;
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
                if motion.enabled {
                    let mut inner = self.interaction.inner.borrow_mut();
                    if !inner.surface.is_active() {
                        inner.surface.begin(
                            crate::geometry::Point::new(
                                bounds.size.width * 0.5,
                                bounds.size.height * 0.5,
                            ),
                            std::time::Instant::now(),
                        );
                        inner.keyboard_pressure = true;
                    }
                }
                if let Some(callback) = self.on_click.as_ref() {
                    (callback.borrow_mut())();
                }
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::PointerMoved { position } => {
                let mut inner = self.interaction.inner.borrow_mut();

                let hovered = hit(*position);

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
                if !hit(*position) {
                    return EventResult::Ignored;
                }

                let mut inner = self.interaction.inner.borrow_mut();

                inner.hovered = true;
                inner.focus_visible = false;
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
                let inside = hit(*position);

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
            let label = if self.leading_symbol.is_some() {
                std::borrow::Cow::Owned(label.replace(['\n', '\r'], " "))
            } else {
                std::borrow::Cow::Borrowed(label.as_str())
            };
            let symbol_width = self.leading_symbol.map_or(0.0, |_| {
                self.size.icon_size(context.theme)
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
                if self.size == ButtonSize::Small {
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
    #[test]
    fn material_surface_preserves_text_hit_area_and_immediate_actions() {
        use crate::{draw_command::DisplayList, geometry::Point, typography::TextMeasurer};
        use std::{
            cell::Cell,
            time::{Duration, Instant},
        };
        let calls = Rc::new(Cell::new(0));
        let callback = Rc::clone(&calls);
        let button = Button::new("Apply").on_click(move || callback.set(callback.get() + 1));
        let theme = Theme::LIGHT;
        let bounds = Rect::new(0.0, 0.0, 96.0, 30.0);
        let mut measurer = TextMeasurer::new();
        let mut rest = DisplayList::new();
        let mut nodes = Vec::new();
        button.paint(
            bounds,
            &mut PaintContext::new(&mut rest, &theme, &theme.typography, &mut measurer)
                .with_accessibility_nodes(&mut nodes),
        );
        {
            let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
            button.handle_event(
                bounds,
                &ViewEvent::PointerPressed {
                    position: Point::new(48.0, 15.0),
                    button: PointerButton::Primary,
                },
                &mut context,
            );
            // Advance a held press without sleeping or depending on host timing.
            button.interaction.inner.borrow_mut().surface.begin(
                Point::new(48.0, 15.0),
                Instant::now() - Duration::from_millis(300),
            );
        }
        let mut pressed = DisplayList::new();
        button.paint(
            bounds,
            &mut PaintContext::new(&mut pressed, &theme, &theme.typography, &mut measurer),
        );
        let text = |list: &DisplayList| {
            list.commands()
                .iter()
                .find_map(|command| {
                    if let DrawCommand::DrawText { command } = command {
                        Some((command.bounds, command.font_size))
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        assert_eq!(text(&rest), text(&pressed));
        assert_eq!(text(&rest).1, 13.0);
        assert_eq!(nodes[0].bounds, bounds);
        let points = pressed
            .commands()
            .iter()
            .find_map(|command| {
                if let DrawCommand::DrawContour { points, .. } = command {
                    Some(points)
                } else {
                    None
                }
            })
            .unwrap();
        let position = Point::new(48.0, 29.99);
        assert!(points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max) < position.y);
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        button.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert_eq!(calls.get(), 1);
        assert!(!button.interaction.inner.borrow().surface.is_active());
        button.handle_event(
            bounds,
            &ViewEvent::KeyboardFocusRequested {
                bounds: Some(bounds),
            },
            &mut context,
        );
        button.handle_event(
            bounds,
            &ViewEvent::KeyPressed {
                key: Key::Space,
                modifiers: Default::default(),
            },
            &mut context,
        );
        assert_eq!(calls.get(), 2);
        assert!(button.interaction.inner.borrow().surface.is_active());
        button.handle_event(
            bounds,
            &ViewEvent::KeyReleased {
                key: Key::Space,
                modifiers: Default::default(),
            },
            &mut context,
        );
        assert!(!button.interaction.inner.borrow().surface.is_active());
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
        let plain = Button::new("Apply");
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

//! ボタンはあったら押したくなる

use std::cell::RefCell;
use std::rc::Rc;

use crate::components::{BorderStyle, Text};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::platform::PointerButton;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::typography::TextAlignment;
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
pub enum ButtonColor {
    Surface,

    #[default]
    Accent,

    Destructive,

    Custom(Color),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ButtonAppearance {
    background: Color,
    border: Color,
    foreground: Color,
}

impl ButtonAppearance {
    fn with_opacity(self, opacity: f32) -> Self {
        Self {
            background: color_with_opacity(self.background, opacity),

            border: color_with_opacity(self.border, opacity),

            foreground: color_with_opacity(self.foreground, opacity),
        }
    }
}

impl ButtonStyle {
    fn resolve(self, theme: &Theme, state: ButtonVisualState) -> ButtonAppearance {
        let hovered = state == ButtonVisualState::Hovered;

        let pressed = state == ButtonVisualState::Pressed;

        match self {
            Self::Standard => ButtonAppearance {
                background: if pressed {
                    theme.colors.surface_muted
                } else if hovered {
                    theme.colors.surface_subtle
                } else {
                    theme.colors.surface
                },

                border: if pressed {
                    Color::rgba(0, 0, 0, 64)
                } else if hovered {
                    Color::rgba(0, 0, 0, 56)
                } else {
                    theme.colors.border_strong
                },

                foreground: theme.colors.text_primary,
            },

            Self::Primary => {
                let color = if pressed {
                    Color::BLACK
                } else if hovered {
                    Color::from_rgb_hex(0x303030)
                } else {
                    theme.colors.text_primary
                };

                ButtonAppearance {
                    background: color,
                    border: color,
                    foreground: Color::WHITE,
                }
            }

            Self::Accent => {
                let color = if pressed {
                    theme.colors.accent_pressed
                } else if hovered {
                    theme.colors.accent_hovered
                } else {
                    theme.colors.accent
                };

                ButtonAppearance {
                    background: color,
                    border: color,
                    foreground: Color::WHITE,
                }
            }

            Self::Ghost => ButtonAppearance {
                background: if pressed {
                    Color::rgba(0, 0, 0, 28)
                } else if hovered {
                    Color::rgba(0, 0, 0, 14)
                } else {
                    Color::TRANSPARENT
                },

                border: Color::TRANSPARENT,

                foreground: theme.colors.text_primary,
            },

            Self::Danger => ButtonAppearance {
                background: if pressed {
                    Color::from_rgb_hex(0xffd8d4)
                } else if hovered {
                    Color::from_rgb_hex(0xffe5e2)
                } else {
                    theme.colors.destructive_soft
                },

                border: if pressed {
                    Color::rgba(196, 43, 28, 112)
                } else if hovered {
                    Color::rgba(196, 43, 28, 87)
                } else {
                    Color::rgba(196, 43, 28, 56)
                },

                foreground: theme.colors.destructive,
            },

            Self::Custom {
                background,
                hovered_background,
                border,
                hovered_border,
                foreground,
            } => ButtonAppearance {
                background: if hovered || pressed {
                    hovered_background
                } else {
                    background
                },

                border: if hovered || pressed {
                    hovered_border
                } else {
                    border
                },

                foreground,
            },
        }
    }

    pub fn foreground_color(self, theme: &Theme) -> Color {
        self.resolve(theme, ButtonVisualState::Normal).foreground
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

    pub fn take_clicked(&self) -> bool {
        let mut inner = self.inner.borrow_mut();

        let clicked = inner.clicked;

        inner.clicked = false;

        clicked
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();

        inner.hovered = false;
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
            inner.armed = false;
            inner.pressed = false;
        }

        changed
    }

    fn visual_state(&self) -> ButtonVisualState {
        let inner = self.inner.borrow();

        if !inner.enabled {
            ButtonVisualState::Disabled
        } else if inner.pressed {
            ButtonVisualState::Pressed
        } else if inner.hovered {
            ButtonVisualState::Hovered
        } else {
            ButtonVisualState::Normal
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonVisualState {
    Normal,
    Hovered,
    Pressed,
    Disabled,
}

pub struct Button {
    interaction: ButtonInteractionState,
    label: Option<String>,
    content: Option<StackChild>,
    style: ButtonStyle,
    radius: CornerRadius,
    shadow: ShadowStyle,
    alignment: ZStackAlignment,
    enabled: bool,
    on_click: Option<RefCell<Box<dyn FnMut()>>>,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            interaction: ButtonInteractionState::new(),
            label: Some(label.into()),
            content: None,
            style: ButtonStyle::Standard,
            radius: CornerRadius::Medium,
            shadow: ShadowStyle::None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            on_click: None,
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

    pub fn color(mut self, color: ButtonColor) -> Self {
        self.style = ButtonStyle::from(color);

        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = radius;
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

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    pub(crate) fn with_interaction(interaction: ButtonInteractionState) -> Self {
        Self {
            interaction,
            label: None,
            content: None,
            style: ButtonStyle::Standard,
            radius: CornerRadius::Medium,
            shadow: ShadowStyle::None,
            alignment: ZStackAlignment::Center,
            enabled: true,
            on_click: None,
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
}

impl View for Button {
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        self.interaction.set_enabled(self.enabled);

        let visual_state = self.interaction.visual_state();

        let mut appearance = self.style.resolve(context.theme, visual_state);

        if visual_state == ButtonVisualState::Disabled {
            appearance = appearance.with_opacity(0.42);
        }

        let shadow = if visual_state == ButtonVisualState::Pressed {
            ShadowStyle::None
        } else {
            self.shadow
        };

        Rectangle::new()
            .color(RectangleColor::Custom(appearance.background))
            .radius(self.radius)
            .shadow(shadow)
            .border(BorderStyle::custom(appearance.border, 1.0))
            .paint(bounds, context);

        if let Some(label) = self.label.as_ref() {
            const HORIZONTAL_PADDING: f32 = 12.0;
            const LINE_HEIGHT: f32 = 20.0;

            let text_height = LINE_HEIGHT.min(bounds.size.height);

            let text_y =
                bounds.origin.y + (bounds.size.height - text_height) * self.label_vertical_factor();

            let text_bounds = Rect::new(
                bounds.origin.x + HORIZONTAL_PADDING,
                text_y,
                (bounds.size.width - HORIZONTAL_PADDING * 2.0).max(0.0),
                text_height,
            );

            Text::new(label.as_str())
                .font_size(13.0)
                .line_height(LINE_HEIGHT)
                .weight(600)
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
            context.request_redraw();
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        match event {
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
                    context.request_redraw();
                }

                if armed {
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

                context.request_redraw();

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

                if clicked {
                    if let Some(callback) = self.on_click.as_ref() {
                        (callback.borrow_mut())();
                    }
                }

                if changed {
                    context.request_redraw();
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
                    context.request_redraw();
                }

                EventResult::Ignored
            }

            ViewEvent::FocusChanged { focused: false } => {
                let mut inner = self.interaction.inner.borrow_mut();

                let changed = inner.armed || inner.pressed;

                inner.armed = false;
                inner.pressed = false;

                drop(inner);

                if changed {
                    context.request_redraw();
                }

                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        const HORIZONTAL_PADDING: f32 = 14.0;
        const INTRINSIC_HEIGHT: f32 = 32.0;

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
                (constraints.maximum.width - HORIZONTAL_PADDING * 2.0).max(0.0)
            } else {
                f32::INFINITY
            };

            let measured = Text::new(label.as_str())
                .font_size(13.0)
                .line_height(20.0)
                .weight(600)
                .measure(
                    Constraints::loose(Size::new(maximum_text_width, f32::INFINITY)),
                    context,
                );

            measured.width + HORIZONTAL_PADDING * 2.0
        } else {
            0.0
        };

        let height = if height_is_fixed {
            constraints.minimum.height
        } else {
            INTRINSIC_HEIGHT
        };

        constraints.constrain(Size::new(width, height))
    }
}

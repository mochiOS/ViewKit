use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, Shadow, ShadowSet, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};

const TRACK_WIDTH: f32 = 44.0;
const TRACK_HEIGHT: f32 = 26.0;
const KNOB_SIZE: f32 = 22.0;
const PRESSED_KNOB_WIDTH: f32 = 25.0;
const KNOB_INSET: f32 = 2.0;

pub struct Switch {
    checked: Binding<bool>,
    label: Option<String>,
    enabled: bool,
    interaction: ButtonInteractionState,
}

impl Switch {
    pub fn new(checked: Binding<bool>) -> Self {
        Self {
            checked,
            label: None,
            enabled: true,
            interaction: ButtonInteractionState::new(),
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked.get()
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    fn button(&self, theme: &Theme) -> Button {
        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small);

        if let Some(label) = self.label.as_ref() {
            content = content.child(
                Text::new(label.clone())
                    .font_size(12.0)
                    .line_height(20.0)
                    .weight(500)
                    .color(if self.enabled {
                        theme.colors.text_primary
                    } else {
                        theme.colors.text_disabled
                    })
                    .height(20.0)
                    .flex_shrink(0.0),
            );
        }

        content = content.child(
            SwitchMark {
                checked: self.checked.get(),
                enabled: self.enabled,
                interaction: self.interaction.clone(),
            }
            .frame(TRACK_WIDTH, TRACK_HEIGHT)
            .flex_shrink(0.0),
        );

        let checked = self.checked.clone();

        Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::TRANSPARENT,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.colors.text_primary,
            })
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::symmetric(6.0, 4.0).content(content))
            .on_click(move || {
                checked.set(!checked.get());
            })
    }
}

impl View for Switch {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.button(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.button(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.button(context.theme)
            .handle_event(bounds, event, context)
    }
}

struct SwitchMark {
    checked: bool,
    enabled: bool,
    interaction: ButtonInteractionState,
}

impl View for SwitchMark {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(TRACK_WIDTH, TRACK_HEIGHT))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let hovered = self.interaction.is_hovered();
        let pressed = self.interaction.is_pressed();

        let track_color = self.track_color(context.theme, hovered, pressed);

        Rectangle::new()
            .color(RectangleColor::Custom(track_color))
            .radius(CornerRadius::Full)
            .paint(bounds, context);

        let knob_width = if pressed {
            PRESSED_KNOB_WIDTH
        } else {
            KNOB_SIZE
        };

        let knob_x = if self.checked {
            bounds.origin.x + bounds.size.width - KNOB_INSET - knob_width
        } else {
            bounds.origin.x + KNOB_INSET
        };

        let knob_y = bounds.origin.y + (bounds.size.height - KNOB_SIZE) / 2.0;

        let knob_bounds = Rect::new(knob_x, knob_y, knob_width, KNOB_SIZE);

        let knob_color = if self.enabled {
            Color::WHITE
        } else {
            Color::rgba(255, 255, 255, 170)
        };

        let knob_shadow = if self.enabled {
            ShadowStyle::Custom(ShadowSet::double(
                Shadow::new(Color::rgba(0, 0, 0, 28), 0.0, 1.0, 2.0, 0.0),
                Shadow::new(Color::rgba(0, 0, 0, 14), 0.0, 2.0, 4.0, 0.0),
            ))
        } else {
            ShadowStyle::None
        };

        Rectangle::new()
            .color(RectangleColor::Custom(knob_color))
            .radius(CornerRadius::Full)
            .shadow(knob_shadow)
            .paint(knob_bounds, context);
    }
}

impl SwitchMark {
    fn track_color(&self, theme: &Theme, hovered: bool, pressed: bool) -> Color {
        let color = if self.checked {
            if pressed {
                theme.colors.accent_pressed
            } else if hovered {
                theme.colors.accent_hovered
            } else {
                theme.colors.accent
            }
        } else if pressed {
            Color::from_rgb_hex(0xc7c7cc)
        } else if hovered {
            Color::from_rgb_hex(0xd1d1d6)
        } else {
            Color::from_rgb_hex(0xe5e5ea)
        };

        if self.enabled {
            color
        } else {
            with_opacity(color, 0.45)
        }
    }
}

fn with_opacity(color: Color, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };

    color.with_alpha((color.alpha as f32 * opacity).round() as u8)
}

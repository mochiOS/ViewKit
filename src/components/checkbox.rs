use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::typography::TextAlignment;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle,
    RectangleColor, Text, ZStackAlignment,
};

pub struct Checkbox {
    checked: Binding<bool>,
    label: Option<String>,
    enabled: bool,
    interaction: ButtonInteractionState,
}

impl Checkbox {
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

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction
    }

    fn button(&self, theme: &Theme) -> Button {
        let checked = self.checked.get();

        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                CheckboxMark {
                    checked,
                    enabled: self.enabled,
                }
                .frame(18.0, 18.0),
            );

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
                    }),
            );
        }

        let checked = self.checked.clone();

        Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::rgba(0, 0, 0, 14),
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.colors.text_primary,
            })
            .radius(CornerRadius::Small)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::symmetric(6.0, 5.0).content(content))
            .on_click(move || {
                checked.set(!checked.get());
            })
    }
}

impl View for Checkbox {
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

struct CheckboxMark {
    checked: bool,
    enabled: bool,
}

impl View for CheckboxMark {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(18.0, 18.0))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if self.checked {
            let color = if self.enabled {
                context.theme.colors.accent
            } else {
                context.theme.colors.accent.alpha(0.42)
            };

            Rectangle::new()
                .color(RectangleColor::Custom(color))
                .radius(CornerRadius::Custom(4.0))
                .paint(bounds, context);

            // TODO: SVGとかにする
            Text::new("✓")
                .font_size(12.0)
                .line_height(18.0)
                .weight(700)
                .alignment(TextAlignment::Center)
                .color(Color::WHITE)
                .paint(bounds, context);
        } else {
            let border = if self.enabled {
                context.theme.colors.border_strong
            } else {
                context.theme.colors.border
            };

            Rectangle::new()
                .color(RectangleColor::Surface)
                .radius(CornerRadius::Custom(4.0))
                .border(BorderStyle::custom(border, 1.0))
                .paint(bounds, context);
        }
    }
}

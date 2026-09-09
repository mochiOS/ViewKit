use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::typography::TextAlignment;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Spacer, Text, ZStackAlignment,
};

pub struct Picker {
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

impl Picker {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            enabled: true,
            interaction: ButtonInteractionState::new(),
        }
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn button(&self, theme: &Theme) -> Button {
        let foreground = if self.enabled {
            theme.colors.text_primary
        } else {
            theme.colors.text_disabled
        };

        Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: theme.colors.surface,
                hovered_background: theme.colors.surface_subtle,
                border: theme.colors.border,
                hovered_border: theme.colors.border_strong,
                foreground,
            })
            .radius(CornerRadius::Small)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(
                Padding::symmetric(8.0, 2.0).content(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Small)
                        .child(
                            Text::new(self.label.clone())
                                .font_size(13.0)
                                .line_height(18.0)
                                .weight(500)
                                .color(foreground)
                                .layout()
                                .flex_grow(1.0),
                        )
                        .child(Spacer::new())
                        .child(
                            Text::new("v")
                                .font_size(11.0)
                                .line_height(18.0)
                                .alignment(TextAlignment::Center)
                                .color(theme.colors.text_secondary)
                                .frame(12.0, 18.0),
                        ),
                ),
            )
    }
}

impl View for Picker {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(self.button(context.theme).measure(
            Constraints::new(Size::new(120.0, 24.0), Size::new(f32::INFINITY, 24.0)),
            context,
        ))
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

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::theme::{CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Icon, IconName, Padding, Spacer, Text,
    ZStackAlignment,
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
                Padding::symmetric(theme.spacing.small, theme.spacing.micro).content(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Small)
                        .child(
                            Text::label(self.label.clone())
                                .color(foreground)
                                .layout()
                                .flex_grow(1.0),
                        )
                        .child(Spacer::new())
                        .child(
                            Icon::new(IconName::ChevronDown)
                                .size(theme.layout.compact_icon_size)
                                .color(theme.colors.text_secondary)
                                .frame(
                                    theme.layout.compact_icon_size,
                                    theme.layout.compact_icon_size,
                                ),
                        ),
                ),
            )
    }
}

impl View for Picker {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(self.button(context.theme).measure(
            Constraints::new(
                Size::new(
                    context.theme.layout.control_min_width,
                    context.theme.layout.compact_control_height,
                ),
                Size::new(f32::INFINITY, context.theme.layout.compact_control_height),
            ),
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

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackDistribution, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::typography::TextAlignment;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Text, ZStackAlignment};

pub struct Stepper {
    value: Binding<i32>,
    minimum: i32,
    maximum: i32,
    step: i32,
    enabled: bool,
    interaction: ButtonInteractionState,
}

impl Stepper {
    pub fn new(value: Binding<i32>) -> Self {
        Self {
            value,
            minimum: i32::MIN,
            maximum: i32::MAX,
            step: 1,
            enabled: true,
            interaction: ButtonInteractionState::new(),
        }
    }

    pub fn range(mut self, minimum: i32, maximum: i32) -> Self {
        self.minimum = minimum.min(maximum);
        self.maximum = minimum.max(maximum);
        self
    }

    pub fn step(mut self, step: i32) -> Self {
        self.step = step.max(1);
        self
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
            .alignment(ZStackAlignment::Center)
            .enabled(self.enabled)
            .content(
                Padding::symmetric(theme.spacing.small, theme.spacing.micro).content(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .distribution(StackDistribution::SpaceBetween)
                        .gap(StackGap::Small)
                        .child(label("−", foreground, theme.layout.stepper_icon_size))
                        .child(
                            Text::label(self.value.get().to_string())
                                .alignment(TextAlignment::Center)
                                .color(foreground)
                                .layout()
                                .flex_grow(1.0),
                        )
                        .child(label("+", foreground, theme.layout.stepper_icon_size)),
                ),
            )
    }

    fn apply_delta(&self, delta: i32) {
        let next = self
            .value
            .get()
            .saturating_add(delta.saturating_mul(self.step))
            .clamp(self.minimum, self.maximum);

        self.value.set(next);
    }
}

impl View for Stepper {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(self.button(context.theme).measure(
            Constraints::new(
                Size::new(
                    context.theme.layout.control_min_width,
                    context.theme.layout.compact_control_height,
                ),
                Size::new(
                    context.theme.layout.control_min_width,
                    context.theme.layout.compact_control_height,
                ),
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
        let result = self
            .button(context.theme)
            .handle_event(bounds, event, context);

        if !self.enabled || !self.interaction.take_clicked() {
            return result;
        }

        if let Some(position) = event.position() {
            if position.x < bounds.origin.x + bounds.size.width / 2.0 {
                self.apply_delta(-1);
            } else {
                self.apply_delta(1);
            }
            return EventResult::Consumed;
        }

        result
    }
}

fn label(text: &'static str, color: Color, size: f32) -> crate::layout::StackChild {
    Text::label(text)
        .alignment(TextAlignment::Center)
        .color(color)
        .frame(size, size)
}

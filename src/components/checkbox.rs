use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, HStack, Icon, IconName, Padding,
    Rectangle, RectangleColor, Text, ZStackAlignment,
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
                .frame(theme.layout.checkbox_size, theme.layout.checkbox_size),
            );

        if let Some(label) = self.label.as_ref() {
            content = content.child(Text::label(label.clone()).color(if self.enabled {
                theme.colors.text_primary
            } else {
                theme.colors.text_disabled
            }));
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
            .content(Padding::all(theme.spacing.extra_small).content(content))
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
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let size = context.theme.layout.checkbox_size;
        constraints.constrain(Size::new(size, size))
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
                .radius(CornerRadius::Custom(context.theme.layout.checkbox_radius))
                .paint(bounds, context);

            Icon::new(IconName::Check)
                .size(context.theme.layout.checkbox_glyph_size)
                .color(Color::WHITE)
                .paint(bounds, context);
        } else {
            let border = if self.enabled {
                context.theme.colors.border_strong
            } else {
                context.theme.colors.border
            };

            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.colors.surface_muted))
                .radius(CornerRadius::Custom(context.theme.layout.checkbox_radius))
                .border(BorderStyle::custom(border, 1.0))
                .paint(bounds, context);
        }
    }
}

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle,
    RectangleColor, Text, ZStackAlignment,
};

pub struct RadioButton {
    selection: Binding<usize>,
    value: usize,

    label: Option<String>,
    enabled: bool,

    interaction: ButtonInteractionState,
}

impl RadioButton {
    pub fn new(selection: Binding<usize>, value: usize) -> Self {
        Self {
            selection,
            value,

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

    pub fn is_selected(&self) -> bool {
        self.selection.get() == self.value
    }

    fn button(&self, theme: &Theme) -> Button {
        let selected = self.is_selected();

        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                RadioMark {
                    selected,
                    enabled: self.enabled,
                }
                .frame(theme.layout.radio_size, theme.layout.radio_size)
                .flex_shrink(0.0),
            );

        if let Some(label) = self.label.as_ref() {
            content = content.child(
                Text::label(label.clone())
                    .color(if self.enabled {
                        theme.colors.text_primary
                    } else {
                        theme.colors.text_disabled
                    })
                    .layout()
                    .flex_shrink(0.0),
            );
        }

        let selection = self.selection.clone();
        let value = self.value;

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
                selection.set(value);
            })
    }
}

impl View for RadioButton {
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

struct RadioMark {
    selected: bool,
    enabled: bool,
}

impl View for RadioMark {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let size = context.theme.layout.radio_size;
        constraints.constrain(Size::new(size, size))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let accent = if self.enabled {
            context.theme.colors.accent
        } else {
            context.theme.colors.accent.alpha(0.42)
        };

        let border = if self.selected {
            accent
        } else if self.enabled {
            context.theme.colors.border_strong
        } else {
            context.theme.colors.border
        };

        let inset = context.theme.layout.radio_inset;
        let indicator_bounds = Rect::new(
            bounds.origin.x + inset,
            bounds.origin.y + inset,
            (bounds.size.width - inset * 2.0).max(0.0),
            (bounds.size.height - inset * 2.0).max(0.0),
        );

        Rectangle::new()
            .color(RectangleColor::Custom(if self.enabled {
                context.theme.colors.surface
            } else {
                context.theme.colors.surface_subtle
            }))
            .radius(CornerRadius::Full)
            .border(BorderStyle::custom(border, 1.0))
            .paint(indicator_bounds, context);

        if !self.selected {
            return;
        }

        let dot_size = context.theme.layout.radio_dot_size;
        let dot_bounds = Rect::new(
            bounds.origin.x + (bounds.size.width - dot_size) / 2.0,
            bounds.origin.y + (bounds.size.height - dot_size) / 2.0,
            dot_size,
            dot_size,
        );

        Rectangle::new()
            .color(RectangleColor::Custom(accent))
            .radius(CornerRadius::Full)
            .paint(dot_bounds, context);
    }
}

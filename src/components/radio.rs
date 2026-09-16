use crate::accessibility::AccessibilityRole;
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
                    .accessibility_hidden(true)
                    .color(if self.enabled {
                        theme.selection_control.foreground
                    } else {
                        theme.selection_control.disabled_foreground
                    })
                    .layout()
                    .flex_shrink(0.0),
            );
        }

        let selection = self.selection.clone();
        let value = self.value;

        let mut button = Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: theme.selection_control.interaction_background,
                hovered_background: theme.selection_control.interaction_hovered_background,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.selection_control.foreground,
            })
            .radius(theme.selection_control.interaction_radius)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::all(theme.selection_control.content_padding).content(content))
            .accessibility_role(AccessibilityRole::RadioButton)
            .accessibility_checked(selected)
            .on_click(move || {
                selection.set(value);
            });

        if let Some(label) = self.label.as_ref() {
            button = button.accessibility_label(label.clone());
        }

        button
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
            context.theme.selection_control.selected
        } else {
            context.theme.selection_control.disabled_selected
        };

        let border = if self.selected {
            accent
        } else if self.enabled {
            context.theme.selection_control.border
        } else {
            context.theme.selection_control.disabled_border
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
                context.theme.selection_control.indicator_background
            } else {
                context.theme.selection_control.disabled_indicator_background
            }))
            .radius(CornerRadius::Full)
            .border(BorderStyle::custom(
                border,
                context.theme.selection_control.stroke_width,
            ))
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

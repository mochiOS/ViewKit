use crate::accessibility::AccessibilityRole;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, HStack, Icon, Padding, SymbolName,
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
            content = content.child(Text::label(label.clone()).accessibility_hidden(true).color(
                if self.enabled {
                    theme.selection_control.foreground
                } else {
                    theme.selection_control.disabled_foreground
                },
            ));
        }

        let checked_binding = self.checked.clone();

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
            .accessibility_role(AccessibilityRole::Checkbox)
            .accessibility_checked(checked)
            .on_click(move || {
                checked_binding.set(!checked_binding.get());
            });

        if let Some(label) = self.label.as_ref() {
            button = button.accessibility_label(label.clone());
        }

        button
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
                context.theme.selection_control.selected
            } else {
                context.theme.selection_control.disabled_selected
            };

            Rectangle::new()
                .color(RectangleColor::Custom(color))
                .radius(CornerRadius::Custom(context.theme.layout.checkbox_radius))
                .paint(bounds, context);

            Icon::new(SymbolName::Check)
                .size(context.theme.layout.checkbox_glyph_size)
                .color(Color::WHITE)
                .paint(bounds, context);
        } else {
            let border = if self.enabled {
                context.theme.selection_control.border
            } else {
                context.theme.selection_control.disabled_border
            };

            Rectangle::new()
                .color(RectangleColor::Custom(if self.enabled {
                    context.theme.selection_control.indicator_background
                } else {
                    context
                        .theme
                        .selection_control
                        .disabled_indicator_background
                }))
                .radius(CornerRadius::Custom(context.theme.layout.checkbox_radius))
                .border(BorderStyle::custom(
                    border,
                    context.theme.selection_control.stroke_width,
                ))
                .paint(bounds, context);
        }
    }
}

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Text, ZStackAlignment};

struct TabItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

pub struct Tabs {
    selection: Binding<usize>,
    items: Vec<TabItem>,
    enabled: bool,
}

impl Tabs {
    pub fn new(selection: Binding<usize>) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
        }
    }

    pub fn item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(TabItem {
            value,
            label: label.into(),
            enabled: true,
            interaction: ButtonInteractionState::new(),
        });
        self
    }

    pub fn disabled_item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(TabItem {
            value,
            label: label.into(),
            enabled: false,
            interaction: ButtonInteractionState::new(),
        });
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    fn item_button(&self, item: &TabItem, theme: &Theme) -> Button {
        let selected = self.selection.get() == item.value;
        let enabled = self.enabled && item.enabled;
        let selection = self.selection.clone();
        let value = item.value;

        let foreground = if !enabled {
            theme.colors.text_disabled
        } else if selected {
            theme.colors.accent
        } else {
            theme.colors.text_secondary
        };

        let background = if selected {
            theme.colors.accent_soft
        } else {
            Color::TRANSPARENT
        };

        Button::with_interaction(item.interaction.clone())
            .style(ButtonStyle::Custom {
                background,
                hovered_background: if selected {
                    theme.colors.accent_soft
                } else {
                    theme.colors.surface_subtle
                },
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground,
            })
            .radius(CornerRadius::Small)
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Center)
            .enabled(enabled)
            .content(
                Padding::symmetric(theme.spacing.small, theme.spacing.micro)
                    .content(Text::label(item.label.clone()).color(foreground)),
            )
            .on_click(move || selection.set(value))
    }
}

impl View for Tabs {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        HStack::new()
            .gap(StackGap::Small)
            .children(self.items.iter().map(|item| {
                self.item_button(item, context.theme).frame(
                    context.theme.layout.tab_width,
                    context.theme.layout.compact_control_height,
                )
            }))
            .measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .children(self.items.iter().map(|item| {
                self.item_button(item, context.theme).frame(
                    context.theme.layout.tab_width,
                    context.theme.layout.compact_control_height,
                )
            }))
            .paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .children(self.items.iter().map(|item| {
                self.item_button(item, context.theme).frame(
                    context.theme.layout.tab_width,
                    context.theme.layout.compact_control_height,
                )
            }))
            .handle_event(bounds, event, context)
    }
}

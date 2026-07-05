use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::state::Binding;
use crate::theme::{CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, Card, HStack, Padding, RectangleColor,
};

struct SegmentedItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

pub struct SegmentedControl {
    selection: Binding<usize>,
    items: Vec<SegmentedItem>,
    enabled: bool,
}

impl SegmentedControl {
    pub fn new(selection: Binding<usize>) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
        }
    }

    pub fn item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(SegmentedItem {
            value,
            label: label.into(),
            enabled: true,
            interaction: ButtonInteractionState::new(),
        });

        self
    }

    pub fn disabled_item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(SegmentedItem {
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

    pub fn selected_value(&self) -> usize {
        self.selection.get()
    }

    fn item_button(&self, item: &SegmentedItem) -> Button {
        let selected = self.selection.get() == item.value;
        let enabled = self.enabled && item.enabled;

        let selection = self.selection.clone();
        let value = item.value;

        Button::with_interaction_and_label(item.interaction.clone(), item.label.clone())
            .style(if selected {
                ButtonStyle::Standard
            } else {
                ButtonStyle::Ghost
            })
            .radius(CornerRadius::ExtraLarge)
            .shadow(ShadowStyle::None)
            .enabled(enabled)
            .on_click(move || {
                selection.set(value);
            })
    }

    fn control(&self, theme: &Theme) -> Card<Padding<HStack>> {
        let row = self.items.iter().fold(
            HStack::new()
                .alignment(StackAlignment::Center)
                .gap(StackGap::None),
            |row, item| row.child(self.item_button(item).height(30.0).flex_shrink(0.0)),
        );

        Card::new()
            .color(RectangleColor::Custom(theme.colors.surface_subtle))
            .radius(CornerRadius::ExtraLarge)
            .shadow(ShadowStyle::None)
            .border(BorderStyle::standard(1.0))
            .content(Padding::all(2.0).content(row))
    }
}

impl View for SegmentedControl {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.control(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.control(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.control(context.theme)
            .handle_event(bounds, event, context)
    }
}

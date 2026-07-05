use super::{BorderStyle, Button, ButtonInteractionState, ButtonStyle, Rectangle, RectangleColor};
use crate::animation::{Animation, Easing, interpolate};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::state::Binding;
use crate::theme::{CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::time::{Duration, Instant};

const CONTROL_INSET: f32 = 2.0;
const SEGMENT_HEIGHT: f32 = 30.0;
const SEGMENT_MIN_WIDTH: f32 = 64.0;
const ANIM_DURATION: Duration = Duration::from_millis(180);
const ANIM_FRAME: Duration = Duration::from_millis(16);

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
        let enabled = self.enabled && item.enabled;

        let selection = self.selection.clone();
        let value = item.value;

        Button::with_interaction_and_label(item.interaction.clone(), item.label.clone())
            .style(ButtonStyle::Ghost)
            .radius(CornerRadius::ExtraLarge)
            .shadow(ShadowStyle::None)
            .enabled(enabled)
            .on_click(move || {
                selection.set(value);
            })
    }

    fn selected_index(&self, value: usize) -> Option<usize> {
        self.items.iter().position(|item| item.value == value)
    }

    fn segment_bounds(&self, bounds: Rect) -> Vec<Rect> {
        if self.items.is_empty() {
            return Vec::new();
        }

        let inner_bounds = Rect::new(
            bounds.origin.x + CONTROL_INSET,
            bounds.origin.y + CONTROL_INSET,
            (bounds.size.width - CONTROL_INSET * 2.0).max(0.0),
            (bounds.size.height - CONTROL_INSET * 2.0).max(0.0),
        );

        let segment_width = inner_bounds.size.width / self.items.len() as f32;

        self.items
            .iter()
            .enumerate()
            .map(|(index, _)| {
                Rect::new(
                    inner_bounds.origin.x + segment_width * index as f32,
                    inner_bounds.origin.y,
                    segment_width,
                    inner_bounds.size.height,
                )
            })
            .collect()
    }

    fn animated_index(&self, now: Instant) -> (Option<f32>, Option<Instant>) {
        let current_value = self.selection.get();

        let Some(current_index) = self.selected_index(current_value) else {
            return (None, None);
        };

        let Some(transition) = self.selection.transition() else {
            return (Some(current_index as f32), None);
        };

        if transition.to != current_value {
            return (Some(current_index as f32), None);
        }

        let Some(from_index) = self.selected_index(transition.from) else {
            return (Some(current_index as f32), None);
        };

        let Some(to_index) = self.selected_index(transition.to) else {
            return (Some(current_index as f32), None);
        };

        if from_index == to_index {
            return (Some(to_index as f32), None);
        }

        let sample = Animation::new(transition.started_at, ANIM_DURATION)
            .easing(Easing::EaseOutCubic)
            .sample(now);

        let index = interpolate(from_index as f32, to_index as f32, sample.progress);

        (Some(index), sample.next_redraw_at)
    }
}

impl View for SegmentedControl {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        if self.items.is_empty() {
            return constraints.constrain(Size::ZERO);
        }

        let mut maximum_width = SEGMENT_MIN_WIDTH;

        for item in &self.items {
            let measured = self.item_button(item).measure(
                Constraints::loose(Size::new(f32::INFINITY, SEGMENT_HEIGHT)),
                context,
            );

            maximum_width = maximum_width.max(measured.width);
        }

        constraints.constrain(Size::new(
            maximum_width * self.items.len() as f32 + CONTROL_INSET * 2.0,
            SEGMENT_HEIGHT + CONTROL_INSET * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        Rectangle::new()
            .color(RectangleColor::Custom(context.theme.colors.surface_subtle))
            .radius(CornerRadius::ExtraLarge)
            .shadow(ShadowStyle::None)
            .border(BorderStyle::standard(1.0))
            .paint(bounds, context);

        let segment_bounds = self.segment_bounds(bounds);

        if segment_bounds.is_empty() {
            return;
        }

        let now = Instant::now();

        let (animated_index, next_redraw) = self.animated_index(now);

        if let Some(next_redraw) = next_redraw {
            context.request_redraw_at(next_redraw);
        }

        if let Some(animated_index) = animated_index {
            let segment_width = segment_bounds[0].size.width;

            let indicator_bounds = Rect::new(
                segment_bounds[0].origin.x + segment_width * animated_index,
                segment_bounds[0].origin.y,
                segment_width,
                segment_bounds[0].size.height,
            );

            let outer_radius = CornerRadius::ExtraLarge.resolve(
                &context.theme.radius,
                bounds.size.width,
                bounds.size.height,
            );

            let indicator_radius = (outer_radius - CONTROL_INSET).max(0.0);

            Rectangle::new()
                .color(RectangleColor::Surface)
                .radius(CornerRadius::Custom(indicator_radius))
                .shadow(ShadowStyle::Card)
                .border(BorderStyle::standard(1.0))
                .paint(indicator_bounds, context);
        }

        for (item, item_bounds) in self.items.iter().zip(segment_bounds) {
            self.item_button(item).paint(item_bounds, context);
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let segment_bounds = self.segment_bounds(bounds);

        let broadcast = event.requires_broadcast();
        let mut result = EventResult::Ignored;

        for (item, item_bounds) in self.items.iter().zip(segment_bounds) {
            if !broadcast && !event.is_inside(item_bounds) {
                continue;
            }

            let item_result = self
                .item_button(item)
                .handle_event(item_bounds, event, context);

            result = result.merge(item_result);

            if !broadcast && item_result.is_consumed() {
                break;
            }
        }

        result
    }
}

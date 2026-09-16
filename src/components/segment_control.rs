use super::{BorderStyle, Button, ButtonInteractionState, ButtonStyle, Rectangle, RectangleColor};
use crate::animation::{Animation, interpolate};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::Key;
use crate::state::Binding;
use crate::theme::{CornerRadius, Motion, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::time::Instant;

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
    accessibility_label: Option<String>,
}

impl SegmentedControl {
    pub fn new(selection: Binding<usize>) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
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

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
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
            .accessibility_role(AccessibilityRole::RadioButton)
            .accessibility_checked(self.selection.get() == value)
            .accessibility_selected(self.selection.get() == value)
            .on_click(move || {
                if selection.get() != value {
                    selection.set(value);
                }
            })
    }

    fn selected_index(&self, value: usize) -> Option<usize> {
        self.items.iter().position(|item| item.value == value)
    }

    fn segment_bounds(&self, bounds: Rect, inset: f32) -> Vec<Rect> {
        if self.items.is_empty() {
            return Vec::new();
        }

        let inner_bounds = Rect::new(
            bounds.origin.x + inset,
            bounds.origin.y + inset,
            (bounds.size.width - inset * 2.0).max(0.0),
            (bounds.size.height - inset * 2.0).max(0.0),
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

    fn animated_index(&self, now: Instant, motion: Motion) -> (Option<f32>, Option<Instant>) {
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

        let sample = Animation::new(transition.started_at, motion.duration)
            .easing(motion.easing)
            .sample(now);

        let index = interpolate(from_index as f32, to_index as f32, sample.progress);

        (Some(index), sample.next_redraw_at)
    }

    fn keyboard_target(&self, event: &ViewEvent) -> Option<usize> {
        if !self.enabled || self.items.is_empty() {
            return None;
        }
        let current = self
            .items
            .iter()
            .position(|item| item.interaction.is_focused())?;
        let direction = match event {
            ViewEvent::KeyPressed {
                key: Key::ArrowRight | Key::ArrowDown,
                ..
            } => 1,
            ViewEvent::KeyPressed {
                key: Key::ArrowLeft | Key::ArrowUp,
                ..
            } => -1,
            ViewEvent::KeyPressed { key: Key::Home, .. } => {
                return self.items.iter().position(|item| item.enabled);
            }
            ViewEvent::KeyPressed { key: Key::End, .. } => {
                return self.items.iter().rposition(|item| item.enabled);
            }
            _ => return None,
        };
        let mut index = current;
        for _ in 0..self.items.len() {
            index = if direction > 0 {
                (index + 1) % self.items.len()
            } else {
                (index + self.items.len() - 1) % self.items.len()
            };
            if self.items[index].enabled {
                return Some(index);
            }
        }
        None
    }
}

impl View for SegmentedControl {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        if self.items.is_empty() {
            return constraints.constrain(Size::ZERO);
        }

        let inset = context.theme.layout.segmented_control_inset;
        let segment_height = context.theme.layout.segmented_control_height;
        let mut maximum_width = context.theme.layout.segmented_item_min_width;

        for item in &self.items {
            let measured = self.item_button(item).measure(
                Constraints::loose(Size::new(f32::INFINITY, segment_height)),
                context,
            );

            maximum_width = maximum_width.max(measured.width);
        }

        constraints.constrain(Size::new(
            maximum_width * self.items.len() as f32 + inset * 2.0,
            segment_height + inset * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let mut node = AccessibilityNode::new(AccessibilityRole::RadioGroup, bounds);
        node.label = self.accessibility_label.clone();
        node.enabled = self.enabled;
        context.record_accessibility(node);

        Rectangle::new()
            .color(RectangleColor::Custom(
                context.theme.segmented_control.background,
            ))
            .radius(context.theme.segmented_control.radius)
            .shadow(ShadowStyle::None)
            .border(BorderStyle::custom(
                context.theme.segmented_control.border,
                context.theme.segmented_control.stroke_width,
            ))
            .paint(bounds, context);

        let inset = context.theme.layout.segmented_control_inset;
        let segment_bounds = self.segment_bounds(bounds, inset);

        if segment_bounds.is_empty() {
            return;
        }

        let now = Instant::now();
        let motion = context.theme.motion.selection;

        let (animated_index, next_redraw) = self.animated_index(now, motion);

        if let Some(next_redraw) = next_redraw {
            context.request_redraw_in_at(bounds.expanded(16.0), next_redraw);
        }

        if let Some(animated_index) = animated_index {
            let segment_width = segment_bounds[0].size.width;

            let indicator_bounds = Rect::new(
                segment_bounds[0].origin.x + segment_width * animated_index,
                segment_bounds[0].origin.y,
                segment_width,
                segment_bounds[0].size.height,
            );

            let outer_radius = context.theme.segmented_control.radius.resolve(
                &context.theme.radius,
                bounds.size.width,
                bounds.size.height,
            );

            let indicator_radius = (outer_radius - inset).max(0.0);

            Rectangle::new()
                .color(RectangleColor::Custom(
                    context.theme.segmented_control.indicator_background,
                ))
                .radius(CornerRadius::Custom(indicator_radius))
                .shadow(ShadowStyle::Card)
                .border(BorderStyle::custom(
                    context.theme.segmented_control.indicator_border,
                    context.theme.segmented_control.stroke_width,
                ))
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
        let segment_bounds =
            self.segment_bounds(bounds, context.theme().layout.segmented_control_inset);

        if let Some(index) = self.keyboard_target(event)
            && let Some(target_bounds) = segment_bounds.get(index).copied()
        {
            self.selection.set_if_changed(self.items[index].value);
            context.request_keyboard_focus(target_bounds);
            context.request_redraw_in(bounds.expanded(16.0));
            return EventResult::Consumed;
        }

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
use crate::accessibility::{AccessibilityNode, AccessibilityRole};

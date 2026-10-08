use super::omochi_shape::{OmochiPreset, OmochiShape, SelectionMotion};
use super::{BorderStyle, Button, ButtonInteractionState, ButtonStyle, Rectangle, RectangleColor};
use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;

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
    on_change: Option<Rc<RefCell<Box<dyn FnMut(usize)>>>>,
    indicator_shape: OmochiShape,
    selection_motion: SelectionMotion,
}

impl SegmentedControl {
    pub fn new(selection: Binding<usize>) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
            on_change: None,
            indicator_shape: OmochiShape::velocity(OmochiPreset::SelectionIndicator),
            selection_motion: SelectionMotion::default(),
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

    pub fn on_change(mut self, callback: impl FnMut(usize) + 'static) -> Self {
        self.on_change = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }

    pub fn selected_value(&self) -> usize {
        self.selection.get()
    }

    fn item_button(&self, item: &SegmentedItem) -> Button {
        let enabled = self.enabled && item.enabled;

        let selection = self.selection.clone();
        let value = item.value;
        let on_change = self.on_change.clone();
        let selection_motion = self.selection_motion.clone();
        let from_index = self.selected_index(self.selection.get()).unwrap_or(0);
        let to_index = self.selected_index(value).unwrap_or(from_index);

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
                    selection.set_without_notification(value);
                    selection_motion.start(from_index, to_index);
                    if let Some(on_change) = on_change.as_ref() {
                        (on_change.borrow_mut())(value);
                    }
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

    fn animated_index(
        &self,
        motion: crate::theme::Motion,
    ) -> (Option<f32>, Option<std::time::Instant>) {
        let current_value = self.selection.get();

        let Some(current_index) = self.selected_index(current_value) else {
            return (None, None);
        };

        let (index, next_redraw) = self.selection_motion.sample(current_index as f32, motion);
        (Some(index), next_redraw)
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
        if !self.enabled && self.indicator_shape.is_animating() {
            self.indicator_shape.reset();
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

        let motion = context.theme.motion.selection;

        let (animated_index, next_redraw) = self.animated_index(motion);

        if let Some(next_redraw) = next_redraw {
            context.request_redraw_in_at(bounds.expanded(16.0), next_redraw);
        }

        if let Some(animated_index) = animated_index {
            let segment_width = segment_bounds[0].size.width;

            if let Some((from, to)) = self.selection_motion.take_launch() {
                let center = |index: f32| {
                    crate::geometry::Point::new(
                        segment_bounds[0].origin.x + segment_width * (index + 0.5),
                        segment_bounds[0].origin.y + segment_bounds[0].size.height / 2.0,
                    )
                };
                self.indicator_shape.begin(center(from));
                self.indicator_shape.moved(center(to));
                self.indicator_shape.end();
            }

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

            context.display_list.push(DrawCommand::PushRoundedClip {
                rect: bounds,
                radius: outer_radius,
            });
            self.indicator_shape.paint(
                indicator_bounds,
                indicator_radius,
                context.theme.segmented_control.indicator_background,
                context,
            );
            Rectangle::new()
                .color(RectangleColor::Custom(crate::theme::Color::TRANSPARENT))
                .radius(CornerRadius::Custom(indicator_radius))
                .shadow(ShadowStyle::Card)
                .border(BorderStyle::custom(
                    context.theme.segmented_control.indicator_border,
                    context.theme.segmented_control.stroke_width,
                ))
                .paint(indicator_bounds, context);
            context.display_list.push(DrawCommand::PopClip);

            if self.selection_motion.ready_to_commit() && !self.indicator_shape.is_animating() {
                self.selection_motion.finish();
                self.selection.commit();
            }
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
        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if self.enabled && bounds.contains(*position) => {
                self.indicator_shape.begin(*position)
            }
            ViewEvent::PointerMoved { position } if self.indicator_shape.is_active() => {
                self.indicator_shape.moved(*position);
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } if self.indicator_shape.is_active() => {
                self.indicator_shape.moved(*position);
                self.indicator_shape.end();
            }
            ViewEvent::FocusChanged { focused: false } if self.indicator_shape.is_active() => {
                self.indicator_shape.end();
            }
            _ => {}
        }
        let segment_bounds =
            self.segment_bounds(bounds, context.theme().layout.segmented_control_inset);

        if let Some(index) = self.keyboard_target(event)
            && let Some(target_bounds) = segment_bounds.get(index).copied()
        {
            let from_index = self.selected_index(self.selection.get()).unwrap_or(index);
            self.selection
                .set_without_notification(self.items[index].value);
            self.selection_motion.start(from_index, index);
            if let Some(on_change) = self.on_change.as_ref() {
                (on_change.borrow_mut())(self.items[index].value);
            }
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

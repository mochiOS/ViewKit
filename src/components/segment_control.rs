use super::omochi_shape::{OmochiPreset, OmochiShape};
use super::{
    BorderStyle, Button, ButtonInteractionState, ButtonStyle, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};
use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::ViewExt;
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

struct SegmentedItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

#[derive(Clone)]
pub struct SegmentedControlInteractionState {
    indicator_shape: OmochiShape,
    selection_motion: MagneticSelectionMotion,
}

impl SegmentedControlInteractionState {
    pub fn new() -> Self {
        Self {
            indicator_shape: OmochiShape::displacement(OmochiPreset::SelectionIndicator),
            selection_motion: MagneticSelectionMotion::default(),
        }
    }
}

#[derive(Clone, Default)]
struct MagneticSelectionMotion {
    state: Rc<RefCell<MagneticSelectionState>>,
}

#[derive(Default)]
struct MagneticSelectionState {
    initialized: bool,
    visual_center: f32,
    target_center: f32,
    velocity: f32,
    last_frame: Option<Instant>,
    dragging: bool,
    pointer_x: f32,
    pointer_velocity: f32,
    last_pointer_x: f32,
    last_pointer_at: Option<Instant>,
    transition_active: bool,
    surface_direction: f32,
}

struct MagneticSample {
    center: f32,
    direction: f32,
    pull_distance: f32,
    animating: bool,
    completed: bool,
}

impl MagneticSelectionMotion {
    fn nearest_index(centers: &[f32], x: f32) -> usize {
        centers
            .iter()
            .enumerate()
            .min_by(|(_, lhs), (_, rhs)| (*lhs - x).abs().total_cmp(&(*rhs - x).abs()))
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn magnetic_center(centers: &[f32], pointer_x: f32, speed: f32) -> f32 {
        let Some(first) = centers.first().copied() else {
            return pointer_x;
        };
        let last = centers.last().copied().unwrap_or(first);
        let bounded = pointer_x.clamp(first, last);
        let nearest = centers[Self::nearest_index(centers, bounded)];
        let distance = bounded - nearest;
        let field = (-(distance * distance) / (2.0 * 42.0 * 42.0)).exp();
        let speed_weakening = 1.0 - (speed.abs() / 1400.0).clamp(0.0, 0.58);
        let strength = 0.86 * field * speed_weakening;
        bounded + (nearest - bounded) * strength
    }

    fn begin_drag(&self, pointer_x: f32, current_center: f32, centers: &[f32]) {
        let now = Instant::now();
        let mut state = self.state.borrow_mut();
        if !state.initialized || !state.transition_active {
            state.initialized = true;
            state.visual_center = current_center;
            state.target_center = current_center;
            state.velocity = 0.0;
        }
        state.dragging = true;
        state.pointer_x = pointer_x;
        state.pointer_velocity = 0.0;
        state.last_pointer_x = pointer_x;
        state.last_pointer_at = Some(now);
        state.last_frame = Some(now);
        state.target_center = Self::magnetic_center(centers, pointer_x, 0.0);
        state.transition_active = true;
    }

    fn move_drag(&self, pointer_x: f32, centers: &[f32]) {
        let now = Instant::now();
        let mut state = self.state.borrow_mut();
        if !state.dragging {
            return;
        }
        let dt = state
            .last_pointer_at
            .map(|last| now.duration_since(last).as_secs_f32().max(0.001))
            .unwrap_or(0.001);
        let instant_velocity = (pointer_x - state.last_pointer_x) / dt;
        let blend = 1.0 - (-dt / 0.030).exp();
        state.pointer_velocity += (instant_velocity - state.pointer_velocity) * blend;
        state.pointer_x = pointer_x;
        state.last_pointer_x = pointer_x;
        state.last_pointer_at = Some(now);
        state.target_center = Self::magnetic_center(centers, pointer_x, state.pointer_velocity);
        state.transition_active = true;
    }

    fn end_drag(&self, pointer_x: f32, centers: &[f32]) -> usize {
        let index = Self::nearest_index(centers, pointer_x);
        let mut state = self.state.borrow_mut();
        state.pointer_x = pointer_x;
        state.dragging = false;
        state.target_center = centers.get(index).copied().unwrap_or(pointer_x);
        state.transition_active = true;
        index
    }

    fn select(&self, current_center: f32, target_center: f32) {
        let mut state = self.state.borrow_mut();
        if !state.initialized || !state.transition_active {
            state.initialized = true;
            state.visual_center = current_center;
            state.velocity = 0.0;
        }
        state.dragging = false;
        state.target_center = target_center;
        state.last_frame = Some(Instant::now());
        state.transition_active = true;
    }

    fn cancel(&self, center: f32) {
        let mut state = self.state.borrow_mut();
        state.initialized = true;
        state.visual_center = center;
        state.target_center = center;
        state.velocity = 0.0;
        state.dragging = false;
        state.transition_active = false;
        state.surface_direction = 0.0;
    }

    fn is_dragging(&self) -> bool {
        self.state.borrow().dragging
    }

    fn surface_direction(&self) -> f32 {
        self.state.borrow().surface_direction
    }

    fn set_surface_direction(&self, direction: f32) {
        self.state.borrow_mut().surface_direction = direction;
    }

    fn sample(&self, fallback_center: f32) -> MagneticSample {
        let now = Instant::now();
        let mut state = self.state.borrow_mut();
        if !state.initialized {
            state.initialized = true;
            state.visual_center = fallback_center;
            state.target_center = fallback_center;
            state.last_frame = Some(now);
        }
        if !state.transition_active {
            state.visual_center = fallback_center;
            state.target_center = fallback_center;
            state.velocity = 0.0;
            state.last_frame = Some(now);
            return MagneticSample {
                center: fallback_center,
                direction: 0.0,
                pull_distance: 0.0,
                animating: false,
                completed: false,
            };
        }

        let dt = state
            .last_frame
            .map(|last| now.duration_since(last).as_secs_f32().clamp(0.001, 0.033))
            .unwrap_or(0.001);
        state.last_frame = Some(now);
        let previous = state.visual_center;
        let error = state.target_center - state.visual_center;
        let follow_time = if state.dragging { 0.026 } else { 0.072 };
        let alpha = 1.0 - (-dt / follow_time).exp();
        state.visual_center += error * alpha;
        state.velocity = (state.visual_center - previous) / dt;

        let request = if state.dragging {
            state.pointer_x - state.visual_center
        } else {
            state.target_center - state.visual_center
        };
        let direction = if request.abs() > 0.35 {
            request.signum()
        } else {
            state.surface_direction
        };
        let speed_contribution = (state.velocity.abs() * 0.010).min(8.0);
        let gain = if state.dragging { 0.72 } else { 0.34 };
        let pull_distance = (request.abs() * gain + speed_contribution).min(34.0);
        let completed = !state.dragging && error.abs() < 0.06 && state.velocity.abs() < 0.8;
        if completed {
            state.visual_center = state.target_center;
            state.velocity = 0.0;
            state.transition_active = false;
        }

        MagneticSample {
            center: state.visual_center,
            direction,
            pull_distance,
            animating: state.transition_active,
            completed,
        }
    }
}

impl Default for SegmentedControlInteractionState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SegmentedControl {
    selection: Binding<usize>,
    items: Vec<SegmentedItem>,
    enabled: bool,
    accessibility_label: Option<String>,
    on_change: Option<Rc<RefCell<Box<dyn FnMut(usize)>>>>,
    interaction_state: SegmentedControlInteractionState,
}

impl SegmentedControl {
    pub fn new(selection: Binding<usize>) -> Self {
        Self::with_interaction(selection, SegmentedControlInteractionState::new())
    }

    pub fn with_interaction(
        selection: Binding<usize>,
        interaction_state: SegmentedControlInteractionState,
    ) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
            on_change: None,
            interaction_state,
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

    fn item_button(&self, item: &SegmentedItem, theme: &crate::theme::Theme) -> Button {
        let enabled = self.enabled && item.enabled;
        let selected = self.selection.get() == item.value;
        let foreground = if !enabled {
            theme.segmented_control.disabled_foreground
        } else if selected {
            theme.segmented_control.selected_foreground
        } else {
            theme.segmented_control.foreground
        };

        Button::with_interaction(item.interaction.clone())
            .style(ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::TRANSPARENT,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground,
            })
            .radius(CornerRadius::Full)
            .shadow(ShadowStyle::None)
            .focus_ring(false)
            .omochi(false)
            .alignment(ZStackAlignment::Center)
            .enabled(enabled)
            .accessibility_role(AccessibilityRole::RadioButton)
            .accessibility_label(item.label.clone())
            .accessibility_checked(selected)
            .accessibility_selected(selected)
            .content(
                Text::label(item.label.clone())
                    .accessibility_hidden(true)
                    .font_size(13.0)
                    .weight(if selected { 600 } else { 500 })
                    .alignment(crate::typography::TextAlignment::Center)
                    .color(foreground)
                    .height(16.0),
            )
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

    fn segment_centers(&self, segment_bounds: &[Rect]) -> Vec<f32> {
        segment_bounds
            .iter()
            .map(|segment| segment.origin.x + segment.size.width / 2.0)
            .collect()
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
            let measured = self.item_button(item, context.theme).measure(
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
        if !self.enabled && self.interaction_state.indicator_shape.is_animating() {
            self.interaction_state.indicator_shape.reset();
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

        if self
            .items
            .iter()
            .any(|item| item.interaction.is_focus_visible())
        {
            Rectangle::new()
                .color(RectangleColor::Custom(crate::theme::Color::TRANSPARENT))
                .radius(context.theme.segmented_control.radius)
                .border(BorderStyle::custom(
                    context.theme.segmented_control.focus_ring,
                    context.theme.segmented_control.focus_ring_width,
                ))
                .paint(bounds, context);
        }

        let inset = context.theme.layout.segmented_control_inset;
        let segment_bounds = self.segment_bounds(bounds, inset);

        if segment_bounds.is_empty() {
            return;
        }

        if let Some(selected_index) = self.selected_index(self.selection.get()) {
            let segment_width = segment_bounds[0].size.width;
            let fallback_center = segment_bounds[selected_index].origin.x + segment_width / 2.0;
            let sample = self
                .interaction_state
                .selection_motion
                .sample(fallback_center);
            let previous_direction = self.interaction_state.selection_motion.surface_direction();

            if sample.direction != 0.0 && sample.pull_distance > 0.0 {
                if previous_direction != 0.0 && previous_direction != sample.direction {
                    self.interaction_state.indicator_shape.end();
                }
                if previous_direction != sample.direction
                    || !self.interaction_state.indicator_shape.is_active()
                {
                    let edge = sample.direction * segment_width / 2.0;
                    self.interaction_state
                        .indicator_shape
                        .begin(Point::new(edge, 0.0));
                    self.interaction_state
                        .selection_motion
                        .set_surface_direction(sample.direction);
                }
                let edge = sample.direction * segment_width / 2.0;
                self.interaction_state.indicator_shape.moved(Point::new(
                    edge + sample.direction * sample.pull_distance,
                    0.0,
                ));
            }
            if sample.completed {
                self.interaction_state.indicator_shape.end();
                self.interaction_state
                    .selection_motion
                    .set_surface_direction(0.0);
                self.selection.commit();
            }
            if sample.animating || self.interaction_state.indicator_shape.is_animating() {
                context.request_redraw_in_at(
                    bounds.expanded(20.0),
                    Instant::now() + Duration::from_millis(8),
                );
            }

            let indicator_bounds = Rect::new(
                sample.center - segment_width / 2.0,
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
            self.interaction_state.indicator_shape.paint_styled(
                indicator_bounds,
                indicator_radius,
                context.theme.segmented_control.indicator_background,
                context.theme.segmented_control.indicator_border,
                context.theme.segmented_control.stroke_width,
                context.theme.segmented_control.indicator_shadow,
                context,
            );
            context.display_list.push(DrawCommand::PopClip);
        }

        for (item, item_bounds) in self.items.iter().zip(segment_bounds) {
            self.item_button(item, context.theme)
                .paint(item_bounds, context);
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let inset = context.theme().layout.segmented_control_inset;
        let segment_bounds = self.segment_bounds(bounds, inset);
        let centers = self.segment_centers(&segment_bounds);
        let selected_index = self.selected_index(self.selection.get()).unwrap_or(0);
        let current_center = centers
            .get(selected_index)
            .copied()
            .unwrap_or(bounds.origin.x);

        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if self.enabled && bounds.contains(*position) && !centers.is_empty() => {
                self.interaction_state.selection_motion.begin_drag(
                    position.x,
                    current_center,
                    &centers,
                );
                context.request_redraw_in(bounds.expanded(20.0));
                return EventResult::Consumed;
            }
            ViewEvent::PointerMoved { position }
                if self.interaction_state.selection_motion.is_dragging() =>
            {
                self.interaction_state
                    .selection_motion
                    .move_drag(position.x, &centers);
                context.request_redraw_in(bounds.expanded(20.0));
                return EventResult::Consumed;
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } if self.interaction_state.selection_motion.is_dragging() => {
                let mut index = self
                    .interaction_state
                    .selection_motion
                    .end_drag(position.x, &centers);
                if !self.items.get(index).is_some_and(|item| item.enabled) {
                    index = self
                        .items
                        .iter()
                        .enumerate()
                        .filter(|(_, item)| item.enabled)
                        .min_by(|(lhs, _), (rhs, _)| {
                            (centers[*lhs] - position.x)
                                .abs()
                                .total_cmp(&(centers[*rhs] - position.x).abs())
                        })
                        .map(|(index, _)| index)
                        .unwrap_or(selected_index);
                    self.interaction_state
                        .selection_motion
                        .select(current_center, centers[index]);
                }
                let value = self.items[index].value;
                if self.selection.get() != value {
                    self.selection.set_without_notification(value);
                    if let Some(on_change) = self.on_change.as_ref() {
                        (on_change.borrow_mut())(value);
                    }
                }
                context.request_keyboard_focus(segment_bounds[index]);
                context.request_redraw_in(bounds.expanded(20.0));
                return EventResult::Consumed;
            }
            ViewEvent::FocusChanged { focused: false }
                if self.interaction_state.selection_motion.is_dragging() =>
            {
                self.interaction_state
                    .selection_motion
                    .cancel(current_center);
                self.interaction_state.indicator_shape.end();
            }
            _ => {}
        }

        if let Some(index) = self.keyboard_target(event)
            && let Some(target_bounds) = segment_bounds.get(index).copied()
        {
            let from_center = centers
                .get(selected_index)
                .copied()
                .unwrap_or(current_center);
            self.selection
                .set_without_notification(self.items[index].value);
            self.interaction_state
                .selection_motion
                .select(from_center, centers[index]);
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

            let item_result =
                self.item_button(item, context.theme())
                    .handle_event(item_bounds, event, context);

            result = result.merge(item_result);

            if !broadcast && item_result.is_consumed() {
                break;
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::{MagneticSelectionMotion, SegmentedControl};
    use crate::draw_command::{DisplayList, DrawCommand};
    use crate::geometry::Rect;
    use crate::state::State;
    use crate::theme::Theme;
    use crate::typography::{TextAlignment, TextMeasurer, Typography};
    use crate::view::{PaintContext, View};

    #[test]
    fn magnetic_center_clamps_to_reference_end_stops() {
        let centers = [70.0, 170.0, 270.0];
        assert_eq!(
            MagneticSelectionMotion::magnetic_center(&centers, -50.0, 0.0),
            70.0
        );
        assert_eq!(
            MagneticSelectionMotion::magnetic_center(&centers, 400.0, 0.0),
            270.0
        );
    }

    #[test]
    fn fast_drag_weakens_magnetic_capture() {
        let centers = [70.0, 170.0, 270.0];
        let slow = MagneticSelectionMotion::magnetic_center(&centers, 100.0, 0.0);
        let fast = MagneticSelectionMotion::magnetic_center(&centers, 100.0, 2_000.0);
        assert!(slow < fast);
        assert!((70.0..=100.0).contains(&slow));
        assert!((70.0..=100.0).contains(&fast));
    }

    #[test]
    fn labels_are_centered_in_each_segment() {
        let selection = State::new(0_usize);
        let control = SegmentedControl::new(selection.binding()).item(0, "Canvas");
        let theme = Theme::LIGHT;
        let bounds = Rect::new(20.0, 30.0, 104.0, 34.0);
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut paint_context = PaintContext::new(
            &mut display_list,
            &theme,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        control.paint(bounds, &mut paint_context);

        let command = display_list
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::DrawText { command } if command.text == "Canvas" => Some(command),
                _ => None,
            });
        let command = command.expect("segment label should be painted");
        assert_eq!(command.alignment, TextAlignment::Center);
        assert_eq!(command.bounds.origin.x, bounds.origin.x + 2.0);
        assert_eq!(command.bounds.size.width, 100.0);
        assert_eq!(command.bounds.size.height, 16.0);
        assert_eq!(command.bounds.origin.y, bounds.origin.y + 9.0);
    }
}

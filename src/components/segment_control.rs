use super::{BorderStyle, Button, ButtonInteractionState, ButtonStyle, Rectangle, RectangleColor};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::omochi::moving::SelectionSurface;
use crate::omochi::{Material, button_contour};
use crate::platform::Key;
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

struct SegmentedItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

/// Retains the selected surface and each segment's focus state across rebuilds.
#[derive(Clone, Default)]
pub struct SegmentedControlInteractionState {
    surface: Rc<RefCell<SelectionSurface>>,
    items: Rc<RefCell<Vec<ButtonInteractionState>>>,
}
impl SegmentedControlInteractionState {
    pub fn new() -> Self {
        Self::default()
    }
    fn item(&self, index: usize) -> ButtonInteractionState {
        let mut items = self.items.borrow_mut();
        if index >= items.len() {
            items.resize_with(index + 1, ButtonInteractionState::new);
        }
        items[index].clone()
    }
}

pub struct SegmentedControl {
    interaction: SegmentedControlInteractionState,
    selection: Binding<usize>,
    items: Vec<SegmentedItem>,
    enabled: bool,
    accessibility_label: Option<String>,
    on_change: Option<Rc<RefCell<Box<dyn FnMut(usize)>>>>,
}

impl SegmentedControl {
    pub fn new(selection: Binding<usize>) -> Self {
        Self::with_interaction(selection, SegmentedControlInteractionState::new())
    }
    pub fn with_interaction(
        selection: Binding<usize>,
        interaction: SegmentedControlInteractionState,
    ) -> Self {
        Self {
            interaction,
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
            on_change: None,
        }
    }

    pub fn item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(SegmentedItem {
            value,
            label: label.into(),
            enabled: true,
            interaction: self.interaction.item(self.items.len()),
        });

        self
    }

    pub fn disabled_item(mut self, value: usize, label: impl Into<String>) -> Self {
        self.items.push(SegmentedItem {
            value,
            label: label.into(),
            enabled: false,
            interaction: self.interaction.item(self.items.len()),
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
        let mut motion = context.theme.motion.omochi;
        motion.enabled &= self.enabled;
        motion.material =
            Material::SELECTION.for_height(segment_bounds[0].size.height * 30.0 / 38.0);
        if let Some(index) = self.selected_index(self.selection.get()) {
            let segment_width = segment_bounds[0].size.width;
            let base_center = segment_bounds[0].origin.x - bounds.origin.x + segment_width * 0.5;
            let target = base_center + segment_width * index as f32;
            let mut surface = self.interaction.surface.borrow_mut();
            if let Some(transition) = self
                .selection
                .transition()
                .filter(|t| t.to == self.selection.get())
            {
                if now.saturating_duration_since(transition.started_at)
                    < context.theme.motion.selection.duration
                {
                    if let Some(from) = self.selected_index(transition.from) {
                        surface.initialize(
                            base_center + segment_width * from as f32,
                            transition.started_at,
                        );
                    }
                }
            }
            let sample = surface.sample(target, segment_bounds[0].size, motion, now);
            if let Some(next) = sample.next_redraw_at {
                context.request_redraw_in_at(bounds.expanded(motion.material.max_pull + 6.0), next);
            }
            let indicator_bounds = Rect::new(
                bounds.origin.x + surface.center - segment_width * 0.5,
                segment_bounds[0].origin.y,
                segment_width,
                segment_bounds[0].size.height,
            );
            drop(surface);
            let outer_radius = context.theme.segmented_control.radius.resolve(
                &context.theme.radius,
                bounds.size.width,
                bounds.size.height,
            );
            let indicator_radius = (outer_radius - inset).max(0.0);
            if sample.pull.x.hypot(sample.pull.y) > 0.001 {
                Rectangle::new()
                    .color(RectangleColor::Custom(Color::TRANSPARENT))
                    .radius(CornerRadius::Custom(indicator_radius))
                    .shadow(ShadowStyle::Card)
                    .paint(indicator_bounds, context);
                context.display_list.push(DrawCommand::DrawContour {
                    points: button_contour(
                        indicator_bounds,
                        indicator_radius,
                        sample,
                        motion.material,
                    ),
                    fill: context.theme.segmented_control.indicator_background,
                    stroke: context.theme.segmented_control.indicator_border,
                    stroke_width: context.theme.segmented_control.stroke_width,
                });
            } else {
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
use crate::accessibility::{AccessibilityNode, AccessibilityRole};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::State, theme::Theme, typography::TextMeasurer};
    #[test]
    fn rebuild_retains_segment_focus_and_keyboard_skips_disabled_items() {
        let value = State::new(10);
        let state = SegmentedControlInteractionState::new();
        let build = || {
            SegmentedControl::with_interaction(value.binding(), state.clone())
                .item(10, "First")
                .disabled_item(20, "Unavailable")
                .item(30, "Last")
        };
        let original = build();
        let bounds = Rect::new(0.0, 0.0, 300.0, 30.0);
        let theme = crate::theme::Theme::LIGHT;
        let regions = original.segment_bounds(bounds, theme.layout.segmented_control_inset);
        let mut text = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut text);
        original.handle_event(
            bounds,
            &ViewEvent::KeyboardFocusRequested {
                bounds: Some(regions[0]),
            },
            &mut context,
        );
        assert!(original.items[0].interaction.is_focused());
        let rebuilt = build();
        assert!(rebuilt.items[0].interaction.is_focused());
        assert!(Rc::ptr_eq(
            &original.interaction.surface,
            &rebuilt.interaction.surface
        ));
        rebuilt.handle_event(
            bounds,
            &ViewEvent::KeyPressed {
                key: Key::ArrowRight,
                modifiers: Default::default(),
            },
            &mut context,
        );
        assert_eq!(value.get(), 30);
        let disabled = build().enabled(false);
        disabled.handle_event(
            bounds,
            &ViewEvent::KeyPressed {
                key: Key::Home,
                modifiers: Default::default(),
            },
            &mut context,
        );
        assert_eq!(value.get(), 30);
        let mut reduced = Theme::LIGHT;
        reduced.motion = reduced.motion.reduced();
        let mut list = crate::draw_command::DisplayList::new();
        rebuilt.paint(
            bounds,
            &mut PaintContext::new(&mut list, &reduced, &reduced.typography, &mut text),
        );
        assert!(
            !list
                .commands()
                .iter()
                .any(|cmd| matches!(cmd, DrawCommand::DrawContour { .. }))
        );
        let expected = regions[2];
        assert!(list.commands().iter().any(|cmd| matches!(cmd,
            DrawCommand::FillRoundedRect { rect, .. } if *rect == expected)));
    }
}

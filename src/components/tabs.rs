use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::omochi_shape::{OmochiPreset, OmochiShape, SelectionMotion};
use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};

struct TabItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

#[derive(Clone)]
pub struct TabsInteractionState {
    indicator_shape: OmochiShape,
    selection_motion: SelectionMotion,
}

impl TabsInteractionState {
    pub fn new() -> Self {
        Self {
            indicator_shape: OmochiShape::displacement(OmochiPreset::SelectionIndicator),
            selection_motion: SelectionMotion::default(),
        }
    }
}

impl Default for TabsInteractionState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Tabs {
    selection: Binding<usize>,
    items: Vec<TabItem>,
    enabled: bool,
    accessibility_label: Option<String>,
    interaction_state: TabsInteractionState,
}

impl Tabs {
    pub fn new(selection: Binding<usize>) -> Self {
        Self::with_interaction(selection, TabsInteractionState::new())
    }

    pub fn with_interaction(
        selection: Binding<usize>,
        interaction_state: TabsInteractionState,
    ) -> Self {
        Self {
            selection,
            items: Vec::new(),
            enabled: true,
            accessibility_label: None,
            interaction_state,
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

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    fn item_button(&self, item: &TabItem, theme: &Theme) -> Button {
        let selected = self.selection.get() == item.value;
        let enabled = self.enabled && item.enabled;
        let selection = self.selection.clone();
        let value = item.value;
        let selection_motion = self.interaction_state.selection_motion.clone();
        let from_index = self.selected_index(self.selection.get()).unwrap_or(0);
        let to_index = self.selected_index(value).unwrap_or(from_index);

        let foreground = if !enabled {
            theme.tabs.disabled_foreground
        } else if selected {
            theme.tabs.selected_foreground
        } else {
            theme.tabs.foreground
        };

        let background = theme.tabs.background;

        Button::with_interaction(item.interaction.clone())
            .style(ButtonStyle::Custom {
                background,
                hovered_background: if selected {
                    theme.tabs.background
                } else {
                    theme.tabs.hovered_background
                },
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground,
            })
            .radius(theme.tabs.radius)
            .shadow(ShadowStyle::None)
            .focus_ring(false)
            .alignment(ZStackAlignment::Center)
            .enabled(enabled)
            .accessibility_role(AccessibilityRole::Tab)
            .accessibility_label(item.label.clone())
            .accessibility_selected(selected)
            .content(
                Padding::symmetric(theme.tabs.horizontal_padding, theme.tabs.vertical_padding)
                    .content(
                        Text::label(item.label.clone())
                            .accessibility_hidden(true)
                            .color(foreground),
                    ),
            )
            .on_click(move || {
                if selection.get() != value {
                    selection.set_without_notification(value);
                    selection_motion.start(from_index, to_index);
                }
            })
    }

    fn stack(&self, theme: &Theme) -> HStack {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::None)
            .children(self.items.iter().map(|item| {
                self.item_button(item, theme).frame(
                    theme.layout.tab_width,
                    theme.tabs.height - theme.tabs.tongue_depth,
                )
            }))
    }

    fn selected_index(&self, value: usize) -> Option<usize> {
        self.items.iter().position(|item| item.value == value)
    }

    fn indicator_local_point(&self, bounds: Rect, position: Point, theme: &Theme) -> Point {
        let index = self.selected_index(self.selection.get()).unwrap_or(0) as f32;
        let center = Point::new(
            bounds.origin.x + theme.layout.tab_width * (index + 0.5),
            bounds.origin.y + (theme.tabs.height - theme.tabs.tongue_depth) / 2.0,
        );
        Point::new(position.x - center.x, position.y - center.y)
    }

    fn animated_index(&self, theme: &Theme) -> (Option<f32>, Option<std::time::Instant>) {
        let value = self.selection.get();
        let Some(current) = self.selected_index(value) else {
            return (None, None);
        };
        let (index, next_redraw) = self
            .interaction_state
            .selection_motion
            .sample(current as f32, theme.motion.selection);
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

impl View for Tabs {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(
            context.theme.layout.tab_width * self.items.len() as f32,
            context.theme.tabs.height,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let mut node = AccessibilityNode::new(AccessibilityRole::TabList, bounds);
        node.label = self.accessibility_label.clone();
        node.enabled = self.enabled;
        context.record_accessibility(node);
        if !self.enabled && self.interaction_state.indicator_shape.is_animating() {
            self.interaction_state.indicator_shape.reset();
        }
        let strip_height = (context.theme.tabs.height - context.theme.tabs.tongue_depth)
            .min(bounds.size.height)
            .max(0.0);
        let strip_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            (context.theme.layout.tab_width * self.items.len() as f32).min(bounds.size.width),
            strip_height,
        );
        Rectangle::new()
            .color(RectangleColor::Custom(context.theme.tabs.strip_background))
            .radius(CornerRadius::Custom(14.0))
            .paint(strip_bounds, context);
        context.display_list.push(DrawCommand::FillRect {
            rect: Rect::new(
                strip_bounds.origin.x,
                strip_bounds.origin.y + strip_height / 2.0,
                strip_bounds.size.width,
                strip_height / 2.0,
            ),
            color: context.theme.tabs.strip_background,
        });
        if self.items.iter().any(|item| item.interaction.is_focused()) {
            Rectangle::new()
                .color(RectangleColor::Custom(Color::TRANSPARENT))
                .radius(CornerRadius::Custom(14.0))
                .border(super::BorderStyle::custom(
                    context.theme.button.focus_ring,
                    context.theme.button.focus_ring_width,
                ))
                .paint(strip_bounds, context);
        }
        let (index, next_redraw) = self.animated_index(context.theme);
        if let Some(next_redraw) = next_redraw {
            context.request_redraw_in_at(bounds.expanded(16.0), next_redraw);
        }
        if let Some(index) = index {
            let width = context.theme.layout.tab_width;
            if let Some((from, to)) = self.interaction_state.selection_motion.take_launch() {
                let direction = (to - from).signum();
                let edge = direction * (width / 2.0 - context.theme.tabs.indicator_inset);
                let pull = ((to - from).abs() * width * 0.34).min(34.0);
                self.interaction_state
                    .indicator_shape
                    .begin(crate::geometry::Point::new(edge, 0.0));
                self.interaction_state
                    .indicator_shape
                    .moved(crate::geometry::Point::new(edge + direction * pull, 0.0));
                self.interaction_state.indicator_shape.end();
            }
            let inset = context.theme.tabs.indicator_inset;
            let indicator = Rect::new(
                bounds.origin.x + width * index + inset,
                bounds.origin.y + inset,
                width - inset * 2.0,
                strip_height - inset,
            );
            let radius = context.theme.tabs.radius.resolve(
                &context.theme.radius,
                indicator.size.width,
                indicator.size.height,
            );
            context
                .display_list
                .push(DrawCommand::PushClip { rect: bounds });
            self.interaction_state.indicator_shape.paint(
                indicator,
                radius,
                context.theme.tabs.selected_background,
                context,
            );
            Rectangle::new()
                .color(RectangleColor::Custom(
                    context.theme.tabs.selected_background,
                ))
                .radius(CornerRadius::Custom(4.0))
                .paint(
                    Rect::new(
                        indicator.origin.x + 12.0,
                        bounds.origin.y + strip_height - 2.0,
                        (indicator.size.width - 24.0).max(0.0),
                        context.theme.tabs.tongue_depth + 2.0,
                    ),
                    context,
                );
            context.display_list.push(DrawCommand::PopClip);
            if self.interaction_state.selection_motion.ready_to_commit() {
                self.interaction_state.selection_motion.finish();
                self.selection.commit();
            }
        }
        self.stack(context.theme).paint(bounds, context);
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
            } if self.enabled && bounds.contains(*position) => self
                .interaction_state
                .indicator_shape
                .begin(self.indicator_local_point(bounds, *position, context.theme())),
            ViewEvent::PointerMoved { position }
                if self.interaction_state.indicator_shape.is_active() =>
            {
                self.interaction_state
                    .indicator_shape
                    .moved(self.indicator_local_point(bounds, *position, context.theme()));
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } if self.interaction_state.indicator_shape.is_active() => {
                self.interaction_state
                    .indicator_shape
                    .moved(self.indicator_local_point(bounds, *position, context.theme()));
                self.interaction_state.indicator_shape.end();
            }
            ViewEvent::FocusChanged { focused: false }
                if self.interaction_state.indicator_shape.is_active() =>
            {
                self.interaction_state.indicator_shape.end();
            }
            _ => {}
        }
        let stack = self.stack(context.theme);
        if let Some(index) = self.keyboard_target(event) {
            let child_bounds = stack.child_bounds_for_event(bounds, context);
            if let Some(target_bounds) = child_bounds.get(index).copied() {
                let from_index = self.selected_index(self.selection.get()).unwrap_or(index);
                self.selection
                    .set_without_notification(self.items[index].value);
                self.interaction_state
                    .selection_motion
                    .start(from_index, index);
                context.request_keyboard_focus(target_bounds);
                context.request_redraw_in(bounds.expanded(16.0));
                return EventResult::Consumed;
            }
        }
        stack.handle_event(bounds, event, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::geometry::Point;
    use crate::state::State;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn selection_changes_immediately_while_indicator_motion_is_retained() {
        let selection = State::new(0_usize);
        let interaction = TabsInteractionState::new();
        let tabs = Tabs::with_interaction(selection.binding(), interaction.clone())
            .item(0, "Overview")
            .item(1, "Activity")
            .item(2, "Settings");
        let theme = Theme::LIGHT;
        let bounds = Rect::new(30.0, 24.0, theme.layout.tab_width * 3.0, theme.tabs.height);
        let second = Point::new(
            bounds.origin.x + theme.layout.tab_width * 1.5,
            bounds.origin.y + (theme.tabs.height - theme.tabs.tongue_depth) / 2.0,
        );
        let mut text_measurer = TextMeasurer::new();
        let mut event_context = EventContext::new(&theme, &Typography::DEFAULT, &mut text_measurer);

        assert_eq!(
            tabs.handle_event(
                bounds,
                &ViewEvent::PointerPressed {
                    position: second,
                    button: PointerButton::Primary,
                },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert_eq!(
            tabs.handle_event(
                bounds,
                &ViewEvent::PointerReleased {
                    position: second,
                    button: PointerButton::Primary,
                },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert_eq!(selection.get(), 1);

        let rebuilt = Tabs::with_interaction(selection.binding(), interaction.clone())
            .item(0, "Overview")
            .item(1, "Activity")
            .item(2, "Settings");
        let mut display_list = DisplayList::new();
        let mut paint_context = PaintContext::new(
            &mut display_list,
            &theme,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        rebuilt.paint(bounds, &mut paint_context);

        assert!(interaction.indicator_shape.is_animating());
        assert!(display_list.commands().iter().any(|command| matches!(
            command,
            DrawCommand::FillPolygon { points, .. } if !points.is_empty()
        )));
    }
}

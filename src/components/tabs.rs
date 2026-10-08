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
    Button, ButtonInteractionState, ButtonStyle, HStack, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};

struct TabItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
}

const INDICATOR_DEFORMATION_ALLOWANCE: f32 = 34.0;

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
                Text::label(item.label.clone())
                    .accessibility_hidden(true)
                    .font_size(13.0)
                    .weight(if selected { 600 } else { 500 })
                    .alignment(crate::typography::TextAlignment::Center)
                    .color(foreground)
                    .height(16.0),
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

    fn indicator_points(&self, theme: &Theme) -> Vec<Point> {
        let half_width = (theme.layout.tab_width - theme.tabs.indicator_inset * 2.0) / 2.0;
        let half_height =
            (theme.tabs.height - theme.tabs.tongue_depth - theme.tabs.indicator_inset) / 2.0;
        let radius = theme
            .tabs
            .radius
            .resolve(&theme.radius, half_width * 2.0, half_height * 2.0);
        let left = -half_width;
        let right = half_width;
        let top = -half_height;
        let bottom = half_height;
        let mut points = vec![Point::new(left, bottom), Point::new(left, top + radius)];
        for index in 1..=12 {
            let angle = (180.0 + 90.0 * index as f32 / 12.0).to_radians();
            points.push(Point::new(
                left + radius + angle.cos() * radius,
                top + radius + angle.sin() * radius,
            ));
        }
        points.push(Point::new(right - radius, top));
        for index in 1..=12 {
            let angle = (-90.0 + 90.0 * index as f32 / 12.0).to_radians();
            points.push(Point::new(
                right - radius + angle.cos() * radius,
                top + radius + angle.sin() * radius,
            ));
        }
        let tongue_half = theme.tabs.tongue_width / 2.0;
        points.extend([
            Point::new(right, bottom),
            Point::new(tongue_half, bottom),
            Point::new(tongue_half, bottom + theme.tabs.tongue_depth),
            Point::new(-tongue_half, bottom + theme.tabs.tongue_depth),
            Point::new(-tongue_half, bottom),
        ]);
        points
    }

    fn indicator_clip_bounds(bounds: Rect) -> Rect {
        Rect::new(
            bounds.origin.x,
            bounds.origin.y - INDICATOR_DEFORMATION_ALLOWANCE,
            bounds.size.width,
            bounds.size.height + INDICATOR_DEFORMATION_ALLOWANCE * 2.0,
        )
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
        context.display_list.push(DrawCommand::FillRect {
            rect: Rect::new(
                strip_bounds.origin.x,
                strip_bounds.origin.y + strip_height,
                strip_bounds.size.width,
                (bounds.size.height - strip_height).max(0.0),
            ),
            color: context.theme.tabs.content_background,
        });
        if self
            .items
            .iter()
            .any(|item| item.interaction.is_focus_visible())
        {
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
                let pull = ((to - from).abs() * width * 0.34).min(INDICATOR_DEFORMATION_ALLOWANCE);
                self.interaction_state
                    .indicator_shape
                    .begin(crate::geometry::Point::new(edge, 0.0));
                self.interaction_state
                    .indicator_shape
                    .moved(crate::geometry::Point::new(edge + direction * pull, 0.0));
                self.interaction_state.indicator_shape.end();
            }
            let inset = context.theme.tabs.indicator_inset;
            let indicator_height = strip_height - inset;
            let center = Point::new(
                bounds.origin.x + width * (index + 0.5),
                bounds.origin.y + inset + indicator_height / 2.0,
            );
            let indicator_points = self.indicator_points(context.theme);
            let indicator_clip = Self::indicator_clip_bounds(bounds);
            context.display_list.push(DrawCommand::PushClip {
                rect: indicator_clip,
            });
            self.interaction_state.indicator_shape.paint_points(
                &indicator_points,
                center,
                context.theme.tabs.selected_background,
                indicator_clip,
                context,
            );
            context.display_list.push(DrawCommand::PopClip);
            if self.interaction_state.selection_motion.ready_to_commit() {
                self.interaction_state.selection_motion.finish();
                self.selection.commit();
            }
        }
        self.stack(context.theme).paint(strip_bounds, context);
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
        let strip_height = (context.theme().tabs.height - context.theme().tabs.tongue_depth)
            .min(bounds.size.height)
            .max(0.0);
        let strip_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            strip_height,
        );
        if let Some(index) = self.keyboard_target(event) {
            let child_bounds = stack.child_bounds_for_event(strip_bounds, context);
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
        stack.handle_event(strip_bounds, event, context)
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
    fn indicator_contour_matches_the_omochi_tab_geometry() {
        let selection = State::new(0_usize);
        let tabs = Tabs::new(selection.binding()).item(0, "Home");
        let points = tabs.indicator_points(&Theme::LIGHT);
        let minimum_x = points
            .iter()
            .map(|point| point.x)
            .fold(f32::INFINITY, f32::min);
        let maximum_x = points
            .iter()
            .map(|point| point.x)
            .fold(f32::NEG_INFINITY, f32::max);
        let minimum_y = points
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        let maximum_y = points
            .iter()
            .map(|point| point.y)
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(minimum_x, -46.0);
        assert_eq!(maximum_x, 46.0);
        assert_eq!(minimum_y, -17.5);
        assert_eq!(maximum_y, 23.5);
        assert!(points.contains(&Point::new(-34.0, 23.5)));
        assert!(points.contains(&Point::new(34.0, 23.5)));
    }

    #[test]
    fn indicator_clip_preserves_vertical_omochi_deformation() {
        let bounds = Rect::new(20.0, 30.0, 300.0, 45.0);
        let clip = Tabs::indicator_clip_bounds(bounds);

        assert_eq!(clip.origin.x, bounds.origin.x);
        assert_eq!(clip.size.width, bounds.size.width);
        assert_eq!(clip.origin.y, bounds.origin.y - 34.0);
        assert_eq!(clip.size.height, bounds.size.height + 68.0);
    }

    #[test]
    fn labels_are_centered_in_the_tab_strip() {
        let selection = State::new(0_usize);
        let tabs = Tabs::new(selection.binding()).item(0, "Overview");
        let theme = Theme::LIGHT;
        let bounds = Rect::new(20.0, 30.0, theme.layout.tab_width, theme.tabs.height);
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut paint_context = PaintContext::new(
            &mut display_list,
            &theme,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        tabs.paint(bounds, &mut paint_context);

        let command = display_list
            .commands()
            .iter()
            .find_map(|command| match command {
                DrawCommand::DrawText { command } if command.text == "Overview" => Some(command),
                _ => None,
            });
        let command = command.expect("tab label should be painted");
        assert_eq!(command.alignment, crate::typography::TextAlignment::Center);
        assert_eq!(command.bounds.origin.x, bounds.origin.x);
        assert_eq!(command.bounds.size.width, theme.layout.tab_width);
        assert_eq!(command.bounds.size.height, 16.0);
        assert_eq!(
            command.bounds.origin.y,
            bounds.origin.y
                + (theme.tabs.height - theme.tabs.tongue_depth - command.bounds.size.height) / 2.0
        );
    }

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

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{IntoStackChild, StackAlignment, StackGap, ViewExt};
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::time::{Duration, Instant};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use super::omochi_shape::{MagneticSelectionMotion, OmochiPreset, OmochiShape};
use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Icon, Rectangle, RectangleColor,
    SymbolName, Text, ZStackAlignment,
};

struct TabItem {
    value: usize,
    label: String,
    enabled: bool,
    interaction: ButtonInteractionState,
    close: Option<Rc<RefCell<Box<dyn FnMut()>>>>,
    close_interaction: ButtonInteractionState,
}

const INDICATOR_DEFORMATION_ALLOWANCE: f32 = 34.0;

#[derive(Clone)]
pub struct TabsInteractionState {
    indicator_shape: OmochiShape,
    selection_motion: MagneticSelectionMotion,
    items: Rc<RefCell<BTreeMap<usize, (ButtonInteractionState, ButtonInteractionState)>>>,
}

impl TabsInteractionState {
    pub fn new() -> Self {
        Self {
            indicator_shape: OmochiShape::displacement(OmochiPreset::SelectionIndicator),
            selection_motion: MagneticSelectionMotion::default(),
            items: Rc::new(RefCell::new(BTreeMap::new())),
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
    item_width: Option<f32>,
    document_style: bool,
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
            item_width: None,
            document_style: false,
        }
    }

    /// タブ1枚の幅を指定します。不正な値は標準幅へ戻します。
    pub fn item_width(mut self, width: f32) -> Self {
        self.item_width = (width.is_finite() && width > 0.0).then_some(width);
        self
    }
    /// 文書向けのタブ表示に切り替えます。グループの背景枠とインセットを使いません。
    pub fn document_style(mut self) -> Self {
        self.document_style = true;
        self
    }
    fn tongue(&self, theme: &Theme) -> f32 {
        if self.document_style {
            0.0
        } else {
            theme.tabs.tongue_depth
        }
    }
    fn inset(&self, theme: &Theme) -> f32 {
        if self.document_style {
            0.0
        } else {
            theme.tabs.indicator_inset
        }
    }
    fn width(&self, theme: &Theme) -> f32 {
        self.item_width.unwrap_or(theme.layout.tab_width)
    }
    /// 標準の閉じるボタン付きタブを追加します。閉じる操作は選択を変更しません。
    pub fn closable_item(
        mut self,
        value: usize,
        label: impl Into<String>,
        on_close: impl FnMut() + 'static,
    ) -> Self {
        self = self.item(value, label);
        self.items.last_mut().unwrap().close = Some(Rc::new(RefCell::new(Box::new(on_close))));
        self
    }
    pub fn item(mut self, value: usize, label: impl Into<String>) -> Self {
        let (interaction, close_interaction) = self
            .interaction_state
            .items
            .borrow_mut()
            .entry(value)
            .or_default()
            .clone();
        self.items.push(TabItem {
            value,
            label: label.into(),
            enabled: true,
            interaction,
            close_interaction,
            close: None,
        });
        self
    }

    pub fn disabled_item(mut self, value: usize, label: impl Into<String>) -> Self {
        let (interaction, close_interaction) = self
            .interaction_state
            .items
            .borrow_mut()
            .entry(value)
            .or_default()
            .clone();
        self.items.push(TabItem {
            value,
            label: label.into(),
            enabled: false,
            interaction,
            close_interaction,
            close: None,
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

        let foreground = if !enabled {
            theme.tabs.disabled_foreground
        } else if selected {
            theme.tabs.selected_foreground
        } else {
            theme.tabs.foreground
        };

        let background = theme.tabs.background;

        let button = Button::with_interaction(item.interaction.clone())
            .content_padding(false)
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
            .accessibility_selected(selected);
        let text = Text::label(item.label.clone())
            .accessibility_hidden(true)
            .font_size(13.0)
            .weight(if selected { 600 } else { 500 })
            .alignment(crate::typography::TextAlignment::Center)
            .color(foreground)
            .single_line(true)
            .height(16.0);
        if item.close.is_some() {
            button.content(
                HStack::new()
                    .alignment(StackAlignment::Center)
                    .gap(StackGap::None)
                    .child(text.flex_grow(1.0).flex_shrink(1.0))
                    .child(
                        super::Spacer::new()
                            .into_stack_child()
                            .width(theme.controls.small_height)
                            .height(1.0)
                            .flex_grow(0.0)
                            .flex_shrink(0.0),
                    )
                    .frame(self.width(theme), 16.0),
            )
        } else {
            button.content(text)
        }
    }

    fn close_bounds(&self, index: usize, b: Rect, theme: &Theme) -> Rect {
        let width = self.width(theme);
        let size = theme.controls.small_height.min(width).min(b.size.height);
        Rect::new(
            b.origin.x + (index + 1) as f32 * width - size,
            b.origin.y + (b.size.height - size) * 0.5,
            size,
            size,
        )
    }
    fn close_button(&self, item: &TabItem) -> Button {
        let close = item.close.clone();
        Button::with_interaction(item.close_interaction.clone())
            .content_padding(false)
            .style(ButtonStyle::Ghost)
            .enabled(self.enabled && item.enabled)
            .accessibility_label(format!("Close {}", item.label))
            .content(Icon::new(SymbolName::X).size(Theme::current().layout.compact_icon_size))
            .on_click(move || {
                if let Some(close) = &close {
                    (close.borrow_mut())();
                }
            })
    }
    fn stack(&self, theme: &Theme) -> HStack {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::None)
            .children(self.items.iter().map(|item| {
                self.item_button(item, theme)
                    .frame(self.width(theme), theme.tabs.height - self.tongue(theme))
            }))
    }

    fn selected_index(&self, value: usize) -> Option<usize> {
        self.items.iter().position(|item| item.value == value)
    }

    fn tab_centers(&self, bounds: Rect, theme: &Theme) -> Vec<f32> {
        self.items
            .iter()
            .enumerate()
            .map(|(index, _)| bounds.origin.x + self.width(theme) * (index as f32 + 0.5))
            .collect()
    }

    fn indicator_points(&self, theme: &Theme) -> Vec<Point> {
        let half_width = (self.width(theme) - self.inset(theme) * 2.0) / 2.0;
        let half_height = (theme.tabs.height - self.tongue(theme) - self.inset(theme)) / 2.0;
        let radius = theme
            .tabs
            .radius
            .resolve(&theme.radius, half_width * 2.0, half_height * 2.0);
        let left = -half_width;
        let right = half_width;
        let top = -half_height;
        let bottom = half_height;
        let shape = crate::continuous_corner::shape(
            Rect::new(left, top, half_width * 2.0, half_height * 2.0),
            [radius, radius, 0.0, 0.0],
        );
        let segments = crate::continuous_corner::polygon_segments(shape.max_radius());
        let mut points = vec![Point::new(left, bottom), Point::new(left, top + radius)];
        // Keep the tab tongue, but use the same top corners as fill, clip,
        // border and input geometry throughout ViewKit.
        for index in 1..=segments {
            let (x, y) =
                shape.corner_point(3, crate::continuous_corner::sample(index * 1024 / segments));
            points.push(Point::new(x, y));
        }
        for index in 0..=segments {
            let (x, y) =
                shape.corner_point(0, crate::continuous_corner::sample(index * 1024 / segments));
            points.push(Point::new(x, y));
        }
        if self.document_style {
            points.extend([Point::new(right, bottom), Point::new(left, bottom)]);
            return points;
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
            self.width(context.theme) * self.items.len() as f32,
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
        let strip_height = (context.theme.tabs.height - self.tongue(context.theme))
            .min(bounds.size.height)
            .max(0.0);
        let strip_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            (self.width(context.theme) * self.items.len() as f32).min(bounds.size.width),
            strip_height,
        );
        if !self.document_style {
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
        }
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
                .paint(
                    if self.document_style {
                        Rect::new(
                            strip_bounds.origin.x
                                + self.selected_index(self.selection.get()).unwrap_or(0) as f32
                                    * self.width(context.theme),
                            strip_bounds.origin.y,
                            self.width(context.theme),
                            strip_bounds.size.height,
                        )
                    } else {
                        strip_bounds
                    },
                    context,
                );
        }
        if let Some(selected_index) = self.selected_index(self.selection.get()) {
            let width = self.width(context.theme);
            let centers = self.tab_centers(bounds, context.theme);
            let fallback_center = centers[selected_index];
            let sample = self
                .interaction_state
                .selection_motion
                .sample_for_motion(fallback_center, context.theme.motion.omochi.enabled);
            let previous_direction = self.interaction_state.selection_motion.surface_direction();
            let indicator_half_width = (width - self.inset(context.theme) * 2.0) / 2.0;

            if sample.direction != 0.0 && sample.pull_distance > 0.0 {
                if previous_direction != 0.0 && previous_direction != sample.direction {
                    self.interaction_state.indicator_shape.end();
                }
                if previous_direction != sample.direction
                    || !self.interaction_state.indicator_shape.is_active()
                {
                    let edge = sample.direction * indicator_half_width;
                    self.interaction_state
                        .indicator_shape
                        .begin(Point::new(edge, 0.0));
                    self.interaction_state
                        .selection_motion
                        .set_surface_direction(sample.direction);
                }
                let edge = sample.direction * indicator_half_width;
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
                    Self::indicator_clip_bounds(bounds),
                    Instant::now() + Duration::from_millis(8),
                );
            }
            let inset = self.inset(context.theme);
            let indicator_height = strip_height - inset;
            let center = Point::new(
                sample.center,
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
        }
        self.stack(context.theme).paint(strip_bounds, context);
        self.interaction_state
            .items
            .borrow_mut()
            .retain(|value, _| self.items.iter().any(|item| item.value == *value));
        for (index, item) in self.items.iter().enumerate() {
            if item.close.is_some() {
                self.close_button(item).paint(
                    self.close_bounds(index, strip_bounds, context.theme),
                    context,
                );
            }
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let stack = self.stack(context.theme);
        let strip_height = (context.theme().tabs.height - self.tongue(context.theme()))
            .min(bounds.size.height)
            .max(0.0);
        let strip_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            strip_height,
        );
        let mut close_result = EventResult::Ignored;
        for (index, item) in self.items.iter().enumerate() {
            if item.close.is_some() {
                close_result = close_result.merge(self.close_button(item).handle_event(
                    self.close_bounds(index, strip_bounds, context.theme()),
                    event,
                    context,
                ));
            }
        }
        if close_result.is_consumed() {
            return close_result;
        }
        let child_bounds = stack.child_bounds_for_event(strip_bounds, context);
        let centers = self.tab_centers(bounds, context.theme());
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
                context.request_redraw_in(Self::indicator_clip_bounds(bounds));
                return EventResult::Consumed;
            }
            ViewEvent::PointerMoved { position }
                if self.interaction_state.selection_motion.is_dragging() =>
            {
                self.interaction_state
                    .selection_motion
                    .move_drag(position.x, &centers);
                context.request_redraw_in(Self::indicator_clip_bounds(bounds));
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
                self.selection
                    .set_without_notification(self.items[index].value);
                if let Some(target_bounds) = child_bounds.get(index).copied() {
                    context.request_keyboard_focus(target_bounds);
                }
                context.request_redraw_in(Self::indicator_clip_bounds(bounds));
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

        if let Some(index) = self.keyboard_target(event) {
            if let Some(target_bounds) = child_bounds.get(index).copied() {
                self.selection
                    .set_without_notification(self.items[index].value);
                self.interaction_state
                    .selection_motion
                    .select(current_center, centers[index]);
                context.request_keyboard_focus(target_bounds);
                context.request_redraw_in(Self::indicator_clip_bounds(bounds));
                return EventResult::Consumed;
            }
        }
        close_result.merge(stack.handle_event(strip_bounds, event, context))
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
    fn indicator_top_corners_follow_the_shared_curve_instead_of_circular_arcs() {
        let selection = State::new(0_usize);
        let tabs = Tabs::new(selection.binding()).item(0, "Home");
        for (width, height, radius) in [(100.0, 45.0, 8.0), (600.0, 300.0, 100.0)] {
            let mut theme = Theme::LIGHT;
            theme.layout.tab_width = width;
            theme.tabs.height = height;
            theme.tabs.radius = CornerRadius::Custom(radius);
            let points = tabs.indicator_points(&theme);
            let half_width = (width - theme.tabs.indicator_inset * 2.0) / 2.0;
            let top = -(height - theme.tabs.tongue_depth - theme.tabs.indicator_inset) / 2.0;
            let (x, y) = crate::continuous_corner::sample(512);
            let expected = Point::new(half_width - radius + radius * x, top + radius * y);
            assert!(
                points
                    .iter()
                    .any(|p| (p.x - expected.x).hypot(p.y - expected.y) < 0.001)
            );
            // Both ends use the same reference and retain reflection symmetry.
            assert!(
                points
                    .iter()
                    .any(|p| (p.x + expected.x).hypot(p.y - expected.y) < 0.001)
            );
            let circle_y = top + radius * (1.0 - std::f32::consts::FRAC_1_SQRT_2);
            assert!((expected.y - circle_y).abs() > radius * 0.02);
        }
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
    fn magnetic_drag_defers_selection_until_release() {
        let selection = State::new(0_usize);
        let interaction = TabsInteractionState::new();
        let tabs = Tabs::with_interaction(selection.binding(), interaction.clone())
            .item(0, "Overview")
            .item(1, "Activity")
            .item(2, "Settings");
        let theme = Theme::LIGHT;
        let bounds = Rect::new(30.0, 24.0, theme.layout.tab_width * 3.0, theme.tabs.height);
        let first = Point::new(
            bounds.origin.x + theme.layout.tab_width * 0.5,
            bounds.origin.y + (theme.tabs.height - theme.tabs.tongue_depth) / 2.0,
        );
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
                    position: first,
                    button: PointerButton::Primary,
                },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert_eq!(
            tabs.handle_event(
                bounds,
                &ViewEvent::PointerMoved { position: second },
                &mut event_context,
            ),
            EventResult::Consumed
        );
        assert_eq!(selection.get(), 0);
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

#[cfg(test)]
mod close_tests {
    use super::*;
    use crate::{state::State, typography::TextMeasurer};
    #[test]
    fn close_survives_rebuild_without_selecting_the_tab_or_starting_a_drag() {
        let selected = State::new(1usize);
        let state = TabsInteractionState::new();
        let count = Rc::new(RefCell::new(0));
        let theme = Theme::LIGHT;
        let b = Rect::new(0., 0., 400., theme.tabs.height);
        let mut text = TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        let build = || {
            let count = count.clone();
            Tabs::with_interaction(selected.binding(), state.clone())
                .item_width(200.)
                .item(1, "Shell")
                .closable_item(2, "Build", move || *count.borrow_mut() += 1)
        };
        let tabs = build();
        let close = tabs.close_bounds(
            1,
            Rect::new(0., 0., 400., theme.tabs.height - theme.tabs.tongue_depth),
            &theme,
        );
        let p = Point::new(
            close.origin.x + close.size.width / 2.,
            close.origin.y + close.size.height / 2.,
        );
        tabs.handle_event(
            b,
            &ViewEvent::PointerPressed {
                position: p,
                button: PointerButton::Primary,
            },
            &mut cx,
        );
        build().handle_event(
            b,
            &ViewEvent::PointerReleased {
                position: p,
                button: PointerButton::Primary,
            },
            &mut cx,
        );
        assert_eq!(*count.borrow(), 1);
        assert_eq!(selected.get(), 1);
        assert!(!state.selection_motion.is_dragging());
    }
    #[test]
    fn width_override_keeps_indicator_hit_geometry_and_measurement_together() {
        let selected = State::new(0usize);
        let t = Tabs::new(selected.binding())
            .item_width(220.)
            .item(0, "Shell")
            .item(1, "Build");
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut m = MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut text,
        };
        assert_eq!(
            t.measure(Constraints::loose(Size::new(800., 100.)), &mut m)
                .width,
            440.
        );
        assert_eq!(
            t.tab_centers(Rect::new(0., 0., 440., 45.), &theme),
            vec![110., 330.]
        );
    }
}

#[cfg(test)]
mod document_tests {
    use super::*;
    use crate::{draw_command::DisplayList, state::State, typography::TextMeasurer};
    #[test]
    fn document_tabs_omit_the_group_backplate_and_ellipsize_full_titles() {
        let state = State::new(1usize);
        let theme = Theme::LIGHT;
        let title =
            "A very long terminal title with 日本語 and a directory path /Projects/abcdefghi";
        let tabs = Tabs::new(state.binding())
            .document_style()
            .item_width(180.)
            .closable_item(1, title, || {});
        let mut list = DisplayList::new();
        let mut text = TextMeasurer::new();
        let mut nodes = vec![];
        tabs.paint(
            Rect::new(0., 0., 180., theme.tabs.height),
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text)
                .with_accessibility_nodes(&mut nodes),
        );
        let label = list
            .commands()
            .iter()
            .find_map(|c| match c {
                DrawCommand::DrawText { command } => Some(command),
                _ => None,
            })
            .unwrap();
        assert!(label.text.ends_with('…'), "{}", label.text);
        assert!(label.bounds.size.width <= 180. - theme.controls.small_height);
        assert!(nodes.iter().any(|n| n.label.as_deref() == Some(title)));
        assert!(!list.commands().iter().any(|c| match c {
            DrawCommand::FillRect { color, .. } | DrawCommand::FillRoundedRect { color, .. } =>
                *color == theme.tabs.strip_background,
            _ => false,
        }));
        let points = tabs.indicator_points(&theme);
        assert_eq!(
            points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
            -90.
        );
        assert_eq!(
            points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
            -theme.tabs.height / 2.
        );
    }
}

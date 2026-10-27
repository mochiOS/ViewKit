//! Content-only toolbar layout shared by application chrome. The caller owns
//! the existing background and toolbar height; geometry, overflow and input
//! use the same measured action frames.
use super::{Button, ButtonInteractionState, SymbolName, Text};
use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{ContextMenuItem, ContextMenuRequest, EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;

pub struct ActionToolbarState<Id> {
    inner: Rc<RefCell<State<Id>>>,
}
struct State<Id> {
    more: ButtonInteractionState,
    request: Option<(u64, u64, Vec<(Id, bool)>)>,
}
impl<Id> Clone for ActionToolbarState<Id> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}
impl<Id> Default for ActionToolbarState<Id> {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(State {
                more: ButtonInteractionState::new(),
                request: None,
            })),
        }
    }
}
impl<Id> ActionToolbarState<Id> {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Leading actions stay left, trailing actions stay right, and all controls
/// share the theme's vertical center and spacing. The first leading action
/// remains visible as navigation. Other actions overflow in reverse order.
/// IDs must be unique and stable across rebuilds. Give symbol-only buttons an
/// accessibility label; that same label is used in the native overflow menu.
pub struct ActionToolbar<Id> {
    state: ActionToolbarState<Id>,
    title: Option<String>,
    items: Vec<(Id, Button, bool)>,
    context_key: u64,
}
struct Geometry {
    frames: Vec<Option<Rect>>,
    overflow: Option<Rect>,
    title: Option<Rect>,
}
impl<Id: Clone + Eq> ActionToolbar<Id> {
    pub fn new(state: ActionToolbarState<Id>) -> Self {
        Self {
            state,
            title: None,
            items: Vec::new(),
            context_key: 0,
        }
    }
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
    pub fn leading(mut self, id: Id, button: Button) -> Self {
        self.items.push((id, button.toolbar(), false));
        self
    }
    /// Change this key when an action's target changes while a menu is open.
    /// Replies for the previous target do not activate actions.
    pub fn context_key(mut self, key: u64) -> Self {
        self.context_key = key;
        self
    }
    pub fn trailing(mut self, id: Id, button: Button) -> Self {
        self.items.push((id, button.toolbar(), true));
        self
    }
    fn more(&self) -> Button {
        Button::with_interaction_and_label(self.state.inner.borrow().more.clone(), "")
            .leading_symbol(SymbolName::More)
            .toolbar()
            .accessibility_label("More actions")
    }
    fn geometry(&self, bounds: Rect, context: &mut MeasureContext<'_>) -> Geometry {
        let width = bounds.size.width.max(0.0);
        let height = bounds.size.height.max(0.0);
        let inset = context.theme.spacing.medium.min(width / 2.0);
        let available = (width - inset * 2.0).max(0.0);
        let gap = context.theme.spacing.extra_small;
        let group_gap = context.theme.spacing.medium;
        let control = context.theme.controls.height.min(height);
        let x = bounds.origin.x + inset;
        let y = bounds.origin.y + (height - control) / 2.0;
        let mut widths: Vec<_> = self
            .items
            .iter()
            .map(|(_, button, _)| {
                button
                    .measure(
                        Constraints::loose(Size::new(f32::INFINITY, control)),
                        context,
                    )
                    .width
            })
            .collect();
        let title_size = self.title.as_ref().map_or(Size::new(0.0, 0.0), |title| {
            Text::label(title).measure(
                Constraints::loose(Size::new(f32::INFINITY, control)),
                context,
            )
        });
        let mut title_width = title_size.width;
        let protected = self.items.iter().position(|(_, _, trailing)| !trailing);
        let mut visible = vec![true; self.items.len()];
        let required = |visible: &[bool], widths: &[f32], title: f32| {
            let mut left = title;
            let mut right = 0.0;
            let mut left_count = usize::from(title > 0.0);
            let mut right_count: usize = 0;
            for (index, (_, _, trailing)) in self.items.iter().enumerate() {
                if visible[index] {
                    if *trailing {
                        right += widths[index];
                        right_count += 1;
                    } else {
                        left += widths[index];
                        left_count += 1;
                    }
                }
            }
            if visible.iter().any(|visible| !visible) {
                right += control.min(available);
                right_count += 1;
            }
            left + right
                + gap * (left_count.saturating_sub(1) + right_count.saturating_sub(1)) as f32
                + if left_count > 0 && right_count > 0 {
                    group_gap
                } else {
                    0.0
                }
        };
        while required(&visible, &widths, title_width) > available {
            let Some(index) = (0..self.items.len())
                .rev()
                .find(|index| visible[*index] && Some(*index) != protected)
            else {
                break;
            };
            visible[index] = false;
        }
        if required(&visible, &widths, title_width) > available {
            title_width =
                (title_width - (required(&visible, &widths, title_width) - available)).max(0.0);
        }
        if let Some(index) = protected {
            widths[index] = (widths[index]
                - (required(&visible, &widths, title_width) - available).max(0.0))
            .max(0.0);
        }
        let has_overflow = visible.iter().any(|visible| !visible);
        let more_width = control.min(available);
        let mut right_width = if has_overflow { more_width } else { 0.0 };
        let mut right_count = usize::from(has_overflow);
        for (index, (_, _, trailing)) in self.items.iter().enumerate() {
            if visible[index] && *trailing {
                right_width += widths[index];
                right_count += 1;
            }
        }
        right_width += gap * right_count.saturating_sub(1) as f32;
        let mut left_x = x;
        let mut right_x = x + (available - right_width).max(0.0);
        let title = (title_width > 0.0).then(|| {
            let result = Rect::new(
                left_x,
                y + (control - title_size.height.min(control)) / 2.0,
                title_width,
                title_size.height.min(control),
            );
            left_x += title_width + gap;
            result
        });
        let mut frames = vec![None; self.items.len()];
        for (index, (_, _, trailing)) in self.items.iter().enumerate() {
            if !visible[index] || widths[index] <= 0.0 || control <= 0.0 {
                continue;
            }
            let origin = if *trailing { &mut right_x } else { &mut left_x };
            frames[index] = Some(Rect::new(*origin, y, widths[index], control));
            *origin += widths[index] + gap;
        }
        let overflow = (has_overflow && more_width > 0.0 && control > 0.0)
            .then_some(Rect::new(right_x, y, more_width, control));
        Geometry {
            frames,
            overflow,
            title,
        }
    }
    pub fn into_visible_actions(
        self,
        bounds: Rect,
        context: &mut MeasureContext<'_>,
    ) -> Vec<(Id, Rect, Button)> {
        let geometry = self.geometry(bounds, context);
        self.items
            .into_iter()
            .zip(geometry.frames)
            .filter_map(|((id, button, _), frame)| frame.map(|frame| (id, frame, button)))
            .collect()
    }
    /// Hidden actions anchor any subsequent popover to the overflow button.
    pub fn action_bounds(
        &self,
        id: &Id,
        bounds: Rect,
        context: &mut MeasureContext<'_>,
    ) -> Option<Rect> {
        let geometry = self.geometry(bounds, context);
        self.items
            .iter()
            .position(|(candidate, _, _)| candidate == id)
            .and_then(|index| geometry.frames[index].or(geometry.overflow))
    }
}
impl<Id: Clone + Eq> View for ActionToolbar<Id> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let mut leading = self.title.as_ref().map_or(Vec::new(), |title| {
            vec![
                Text::label(title)
                    .measure(
                        Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
                        context,
                    )
                    .width,
            ]
        });
        let mut trailing = Vec::new();
        for (_, button, right) in &self.items {
            let width = button
                .measure(
                    Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
                    context,
                )
                .width;
            if *right {
                trailing.push(width);
            } else {
                leading.push(width);
            }
        }
        let sum = |values: &[f32]| {
            values.iter().sum::<f32>()
                + context.theme.spacing.extra_small * values.len().saturating_sub(1) as f32
        };
        let width = sum(&leading)
            + sum(&trailing)
            + context.theme.spacing.medium * 2.0
            + if !leading.is_empty() && !trailing.is_empty() {
                context.theme.spacing.medium
            } else {
                0.0
            };
        constraints.constrain(Size::new(
            width,
            context
                .theme
                .layout
                .top_bar_height
                .max(context.theme.controls.height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let geometry = self.geometry(
            bounds,
            &mut MeasureContext {
                theme: context.theme,
                typography: context.typography,
                text_measurer: &mut *context.text_measurer,
            },
        );
        context.record_accessibility(AccessibilityNode::new(AccessibilityRole::Toolbar, bounds));
        if let (Some(title), Some(frame)) = (&self.title, geometry.title) {
            context
                .display_list
                .push(crate::draw_command::DrawCommand::PushClip { rect: frame });
            Text::label(title).paint(frame, context);
            context
                .display_list
                .push(crate::draw_command::DrawCommand::PopClip);
        }
        for ((_, button, _), frame) in self.items.iter().zip(geometry.frames) {
            if let Some(frame) = frame {
                button.paint(frame, context);
            } else {
                button.cancel_tracking();
            }
        }
        if let Some(frame) = geometry.overflow {
            self.more().paint(frame, context);
        } else {
            self.state.inner.borrow().more.reset();
        }
    }
    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if let ViewEvent::ContextMenuResult {
            request_id,
            command_id,
        } = event
        {
            let snapshot = {
                let mut state = self.state.inner.borrow_mut();
                if !state
                    .request
                    .as_ref()
                    .is_some_and(|(id, _, _)| id == request_id)
                {
                    return EventResult::Ignored;
                }
                let (_, key, snapshot) = state.request.take().unwrap();
                if key != self.context_key {
                    return EventResult::Consumed;
                }
                snapshot
            };
            if let Some((id, true)) = command_id
                .and_then(|id| id.checked_sub(1))
                .and_then(|index| snapshot.get(index as usize))
            {
                if let Some((_, button, _)) =
                    self.items.iter().find(|(candidate, _, _)| candidate == id)
                {
                    button.activate_from_menu();
                }
            }
            context.request_redraw();
            return EventResult::Consumed;
        }
        let geometry = self.geometry(bounds, &mut context.measure_context());
        for ((_, button, _), frame) in self.items.iter().zip(&geometry.frames) {
            if let Some(frame) = frame {
                let result = button.handle_event(*frame, event, context);
                if result.is_consumed() {
                    return result;
                }
            } else {
                button.cancel_tracking();
            }
        }
        if let Some(frame) = geometry.overflow {
            let result = self.more().handle_event(frame, event, context);
            if self.state.inner.borrow().more.take_clicked() {
                let request_id = ContextMenuRequest::next_id();
                let hidden: Vec<_> = self
                    .items
                    .iter()
                    .zip(&geometry.frames)
                    .filter(|(_, frame)| frame.is_none())
                    .collect();
                let choices = hidden
                    .iter()
                    .map(|((id, button, _), _)| (id.clone(), button.action_enabled()))
                    .collect();
                let items = hidden
                    .iter()
                    .enumerate()
                    .map(|(index, ((_, button, _), _))| ContextMenuItem {
                        command_id: index as u32 + 1,
                        label: button.action_label(),
                        enabled: button.action_enabled(),
                        checked: button.action_checked(),
                        destructive: button.action_destructive(),
                        separator: false,
                    })
                    .collect();
                self.state.inner.borrow_mut().request =
                    Some((request_id, self.context_key, choices));
                context.show_context_menu(ContextMenuRequest {
                    request_id,
                    position: Point::new(frame.origin.x, frame.origin.y + frame.size.height),
                    items,
                });
            }
            return result;
        }
        EventResult::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::PointerButton;
    use crate::theme::Theme;
    use crate::typography::TextMeasurer;
    use std::cell::Cell;

    #[test]
    fn overflow_input_and_menu_anchor_use_the_painted_accessibility_font_scale() {
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        text.set_font_scale(2.0);
        let toolbar = fixture(ActionToolbarState::new(), Rc::new(Cell::new(0)), true);
        let bounds = Rect::new(0.0, 0.0, 260.0, 48.0);
        let mut commands = crate::draw_command::DisplayList::new();
        toolbar.paint(
            bounds,
            &mut PaintContext::new(&mut commands, &theme, &theme.typography, &mut text),
        );
        let geometry = toolbar.geometry(
            bounds,
            &mut MeasureContext {
                theme: &theme,
                typography: &theme.typography,
                text_measurer: &mut text,
            },
        );
        let more = geometry.overflow.unwrap();
        let position = Point::new(
            more.origin.x + more.size.width / 2.0,
            more.origin.y + more.size.height / 2.0,
        );
        let mut events = EventContext::new(&theme, &theme.typography, &mut text);
        toolbar.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            },
            &mut events,
        );
        toolbar.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            },
            &mut events,
        );
        let request = events.take_context_menu_request().unwrap();
        assert_eq!(
            request.position,
            Point::new(more.origin.x, more.origin.y + more.size.height)
        );
    }

    #[test]
    fn hidden_pressed_actions_are_cancelled_and_overflow_preserves_semantic_state() {
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let hits = Rc::new(Cell::new(0));
        let toolbar = ActionToolbar::new(ActionToolbarState::new())
            .leading(1, Button::new("Back"))
            .trailing(
                2,
                Button::new("Remove selected item")
                    .style(super::super::ButtonStyle::Danger)
                    .accessibility_selected(true)
                    .on_click({
                        let hits = hits.clone();
                        move || hits.set(hits.get() + 1)
                    }),
            );
        let wide = Rect::new(0.0, 0.0, 800.0, 48.0);
        let frame = toolbar
            .action_bounds(
                &2,
                wide,
                &mut MeasureContext {
                    theme: &theme,
                    typography: &theme.typography,
                    text_measurer: &mut text,
                },
            )
            .unwrap();
        let position = Point::new(frame.origin.x + frame.size.width / 2.0, 24.0);
        toolbar.handle_event(
            wide,
            &ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            },
            &mut EventContext::new(&theme, &theme.typography, &mut text),
        );
        let mut commands = crate::draw_command::DisplayList::new();
        toolbar.paint(
            Rect::new(0.0, 0.0, 100.0, 48.0),
            &mut PaintContext::new(&mut commands, &theme, &theme.typography, &mut text),
        );
        toolbar.handle_event(
            wide,
            &ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            },
            &mut EventContext::new(&theme, &theme.typography, &mut text),
        );
        assert_eq!(hits.get(), 0);
        let request = open_overflow(&toolbar, &theme, &mut text);
        assert!(request.items[0].destructive && request.items[0].checked);
        let toolbar = toolbar.context_key(1);
        toolbar.handle_event(
            wide,
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id,
                command_id: Some(1),
            },
            &mut EventContext::new(&theme, &theme.typography, &mut text),
        );
        assert_eq!(hits.get(), 0);
    }

    fn fixture(
        state: ActionToolbarState<u8>,
        hits: Rc<Cell<u8>>,
        enabled: bool,
    ) -> ActionToolbar<u8> {
        ActionToolbar::new(state)
            .leading(
                1,
                Button::new("")
                    .leading_symbol(SymbolName::ChevronLeft)
                    .accessibility_label("Back"),
            )
            .trailing(
                2,
                Button::new("Install Font…")
                    .enabled(enabled)
                    .on_click(move || hits.set(hits.get() + 1)),
            )
            .trailing(
                3,
                Button::new("")
                    .leading_symbol(SymbolName::Refresh)
                    .accessibility_label("Refresh"),
            )
    }
    #[test]
    fn actions_remain_inside_and_disjoint_at_all_widths_and_font_scales() {
        for theme in [Theme::LIGHT, Theme::DARK] {
            for scale in [1.0, 2.0] {
                let mut text = TextMeasurer::new();
                text.set_font_scale(scale);
                for width in [0.0, 12.0, 80.0, 140.0, 240.0, 420.0, 1000.0] {
                    let bounds = Rect::new(17.0, 23.0, width, 48.0);
                    let toolbar = fixture(ActionToolbarState::new(), Rc::new(Cell::new(0)), true)
                        .title("Fonts");
                    let geometry = toolbar.geometry(
                        bounds,
                        &mut MeasureContext {
                            theme: &theme,
                            typography: &theme.typography,
                            text_measurer: &mut text,
                        },
                    );
                    let mut frames: Vec<_> = geometry.frames.into_iter().flatten().collect();
                    frames.extend(geometry.overflow);
                    frames.extend(geometry.title);
                    for (i, frame) in frames.iter().enumerate() {
                        assert!(frame.origin.x >= bounds.origin.x);
                        assert!(
                            frame.origin.x + frame.size.width
                                <= bounds.origin.x + bounds.size.width + 0.01
                        );
                        assert!(frame.origin.y >= bounds.origin.y);
                        assert!(
                            frame.origin.y + frame.size.height
                                <= bounds.origin.y + bounds.size.height
                        );
                        assert!((frame.origin.y + frame.size.height / 2.0 - 47.0).abs() < 0.01);
                        for other in &frames[i + 1..] {
                            assert!(
                                frame.origin.x + frame.size.width <= other.origin.x + 0.01
                                    || other.origin.x + other.size.width <= frame.origin.x + 0.01
                            );
                        }
                    }
                }
            }
        }
    }
    fn open_overflow(
        toolbar: &ActionToolbar<u8>,
        theme: &Theme,
        text: &mut TextMeasurer,
    ) -> ContextMenuRequest {
        let bounds = Rect::new(0.0, 0.0, 100.0, 48.0);
        let geometry = toolbar.geometry(
            bounds,
            &mut MeasureContext {
                theme,
                typography: &theme.typography,
                text_measurer: text,
            },
        );
        let frame = geometry.overflow.unwrap();
        let position = Point::new(frame.origin.x + frame.size.width / 2.0, 24.0);
        let mut events = EventContext::new(theme, &theme.typography, text);
        toolbar.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            },
            &mut events,
        );
        toolbar.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            },
            &mut events,
        );
        events.take_context_menu_request().unwrap()
    }
    #[test]
    fn overflow_dispatches_stable_ids_and_rejects_stale_disabled_and_replayed_results() {
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let state = ActionToolbarState::new();
        let hits = Rc::new(Cell::new(0));
        let toolbar = fixture(state.clone(), hits.clone(), true);
        let request = open_overflow(&toolbar, &theme, &mut text);
        assert_eq!(request.items[0].label, "Install Font…");
        let event = ViewEvent::ContextMenuResult {
            request_id: request.request_id,
            command_id: Some(1),
        };
        // Reordering the current toolbar must not reinterpret the old index.
        let other = ActionToolbar::new(state.clone())
            .trailing(3, Button::new("Refresh"))
            .trailing(
                2,
                Button::new("Install").on_click({
                    let hits = hits.clone();
                    move || hits.set(hits.get() + 1)
                }),
            );
        let mut events = EventContext::new(&theme, &theme.typography, &mut text);
        other.handle_event(Rect::new(0.0, 0.0, 400.0, 48.0), &event, &mut events);
        other.handle_event(Rect::new(0.0, 0.0, 400.0, 48.0), &event, &mut events);
        assert_eq!(hits.get(), 1);
        drop(events);
        let request = open_overflow(&toolbar, &theme, &mut text);
        let disabled = fixture(state.clone(), hits.clone(), false);
        disabled.handle_event(
            Rect::new(0.0, 0.0, 100.0, 48.0),
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id,
                command_id: Some(1),
            },
            &mut EventContext::new(&theme, &theme.typography, &mut text),
        );
        assert_eq!(hits.get(), 1);
        let request = open_overflow(&disabled, &theme, &mut text);
        assert!(!request.items[0].enabled);
        toolbar.handle_event(
            Rect::new(0.0, 0.0, 100.0, 48.0),
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id,
                command_id: Some(1),
            },
            &mut EventContext::new(&theme, &theme.typography, &mut text),
        );
        assert_eq!(hits.get(), 1);
    }
}

//! Document-tab ordering belongs to the standard component. Values, not visual
//! indices, identify documents so closing/rebuilding tabs retains their meaning.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct DocumentDrag {
    value: usize,
    origin: Point,
    target: usize,
    active: bool,
}

impl Tabs {
    pub(super) fn restore_document_order(&mut self) {
        if !self.document_style || self.compact_style {
            return;
        }
        let mut order = self.interaction_state.document_order.borrow_mut();
        for item in &self.items {
            if !order.contains(&item.value) {
                order.push(item.value);
            }
        }
        self.items.sort_by_key(|item| {
            order
                .iter()
                .position(|v| *v == item.value)
                .unwrap_or(usize::MAX)
        });
    }

    pub(super) fn handle_document_drag(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> Option<EventResult> {
        let state = &self.interaction_state.document_drag;
        if !self.enabled
            || state.get().is_some_and(|drag| {
                !self
                    .items
                    .iter()
                    .any(|item| item.value == drag.value && item.enabled)
            })
        {
            state.set(None);
        }
        let hit = |position: Point| {
            if !bounds.contains(position) {
                return None;
            }
            let index =
                ((position.x - bounds.origin.x) / self.width(context.theme())).floor() as usize;
            self.items
                .get(index)
                .filter(|item| item.enabled)
                .map(|_| index)
        };
        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !self.enabled {
                    return None;
                }
                let Some(index) = hit(*position) else {
                    return Some(EventResult::Ignored);
                };
                state.set(Some(DocumentDrag {
                    value: self.items[index].value,
                    origin: *position,
                    target: index,
                    active: false,
                }));
                Some(EventResult::Consumed)
            }
            ViewEvent::PointerMoved { position } if state.get().is_some() => {
                let mut drag = state.get().unwrap();
                drag.active |= (position.x - drag.origin.x).hypot(position.y - drag.origin.y)
                    > context.theme.spacing.small;
                if drag.active {
                    drag.target = (((position.x - bounds.origin.x) / self.width(context.theme()))
                        .floor()
                        .max(0.0) as usize)
                        .min(self.items.len().saturating_sub(1));
                    // Moving a document never selects the tab underneath it.
                    self.selection.set_if_changed(drag.value);
                }
                state.set(Some(drag));
                context.request_redraw_in(Self::indicator_clip_bounds(bounds));
                Some(EventResult::Consumed)
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } if state.get().is_some() => {
                let drag = state.replace(None).unwrap();
                let Some(from) = self.selected_index(drag.value) else {
                    return Some(EventResult::Consumed);
                };
                if bounds.contains(*position) {
                    if drag.active {
                        let mut order: Vec<_> = self.items.iter().map(|item| item.value).collect();
                        let value = order.remove(from);
                        order.insert(drag.target.min(order.len()), value);
                        *self.interaction_state.document_order.borrow_mut() = order;
                    } else if self.selection.get() == drag.value {
                        if let Some(callback) = &self.on_selected_activate {
                            (callback.borrow_mut())();
                        }
                    }
                    self.selection.set_if_changed(drag.value);
                    let index = if drag.active { drag.target } else { from };
                    self.interaction_state.selection_motion.cancel(
                        bounds.origin.x + self.width(context.theme()) * (index as f32 + 0.5),
                    );
                    context.request_keyboard_focus(Rect::new(
                        bounds.origin.x + index as f32 * self.width(context.theme()),
                        bounds.origin.y,
                        self.width(context.theme()),
                        bounds.size.height,
                    ));
                }
                context.request_redraw_in(Self::indicator_clip_bounds(bounds));
                Some(EventResult::Consumed)
            }
            ViewEvent::PointerLeft | ViewEvent::FocusChanged { focused: false } => {
                if state.replace(None).is_some() {
                    context.request_redraw_in(Self::indicator_clip_bounds(bounds));
                }
                None
            }
            _ => None,
        }
    }

    pub(super) fn paint_document_drop(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let Some(drag) = self
            .interaction_state
            .document_drag
            .get()
            .filter(|drag| drag.active)
        else {
            return;
        };
        let Some(from) = self.selected_index(drag.value) else {
            return;
        };
        let boundary = drag.target + usize::from(drag.target > from);
        context.display_list.push(DrawCommand::FillRect {
            rect: Rect::new(
                bounds.origin.x + self.width(context.theme) * boundary as f32 - 1.0,
                bounds.origin.y + 4.0,
                2.0,
                (bounds.size.height - 8.0).max(0.0),
            ),
            color: context.theme.colors.accent,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::State, typography::TextMeasurer};

    #[test]
    fn close_callback_follows_document_value_after_visual_reordering() {
        let selected = State::new(20usize);
        let state = TabsInteractionState::new();
        *state.document_order.borrow_mut() = vec![20, 10];
        let closed = Rc::new(std::cell::Cell::new(0usize));
        let first = closed.clone();
        let second = closed.clone();
        let tabs = Tabs::with_interaction(selected.binding(), state.clone())
            .document_style()
            .item_width(100.0)
            .closable_item(10, "A", move || first.set(10))
            .closable_item(20, "B", move || second.set(20));
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        let bounds = Rect::new(0.0, 0.0, 200.0, 36.0);
        let close = tabs.close_bounds(1, bounds, &theme);
        let point = Point::new(
            close.origin.x + close.size.width / 2.0,
            close.origin.y + close.size.height / 2.0,
        );
        tabs.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position: point,
                button: PointerButton::Primary,
            },
            &mut cx,
        );
        tabs.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position: point,
                button: PointerButton::Primary,
            },
            &mut cx,
        );
        assert_eq!(closed.get(), 10);
        assert_eq!(selected.get(), 20);
        assert!(state.document_drag.get().is_none());
    }

    #[test]
    fn dragging_a_document_reorders_values_and_keeps_selection_after_rebuild() {
        let selection = State::new(10usize);
        let state = TabsInteractionState::new();
        let make = || {
            Tabs::with_interaction(selection.binding(), state.clone())
                .document_style()
                .item_width(100.0)
                .item(10, "A")
                .item(20, "B")
                .item(30, "C")
        };
        let tabs = make();
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        let bounds = Rect::new(0.0, 0.0, 300.0, 36.0);
        for event in [
            ViewEvent::PointerPressed {
                position: Point::new(40.0, 16.0),
                button: PointerButton::Primary,
            },
            ViewEvent::PointerMoved {
                position: Point::new(240.0, 16.0),
            },
            ViewEvent::PointerReleased {
                position: Point::new(240.0, 16.0),
                button: PointerButton::Primary,
            },
        ] {
            assert!(tabs.handle_event(bounds, &event, &mut cx).is_consumed());
            assert_eq!(selection.get(), 10);
        }
        assert_eq!(
            make()
                .items
                .iter()
                .map(|item| item.value)
                .collect::<Vec<_>>(),
            vec![20, 30, 10]
        );
        // Removing an unrelated document cannot transfer the dragged identity.
        let rebuilt = Tabs::with_interaction(selection.binding(), state.clone())
            .document_style()
            .item_width(100.0)
            .item(10, "A")
            .item(30, "C");
        assert_eq!(
            rebuilt
                .items
                .iter()
                .map(|item| item.value)
                .collect::<Vec<_>>(),
            vec![30, 10]
        );
    }

    #[test]
    fn leaving_or_removing_a_document_cancels_reordering() {
        let selection = State::new(1usize);
        let state = TabsInteractionState::new();
        let tabs = Tabs::with_interaction(selection.binding(), state.clone())
            .document_style()
            .item_width(100.0)
            .item(1, "A")
            .item(2, "B");
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut cx = EventContext::new(&theme, &theme.typography, &mut text);
        let bounds = Rect::new(0.0, 0.0, 200.0, 36.0);
        tabs.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position: Point::new(30.0, 16.0),
                button: PointerButton::Primary,
            },
            &mut cx,
        );
        tabs.handle_event(bounds, &ViewEvent::PointerLeft, &mut cx);
        assert!(state.document_drag.get().is_none());
        assert!(state.document_order.borrow().iter().copied().eq([1, 2]));
    }
}

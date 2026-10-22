//! A filled vertical level control. Standard horizontal sliders are unaffected.
use super::*;
use crate::draw_command::DrawCommand;
use crate::platform::Key;

impl Slider {
    pub(super) fn paint_level(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }
        self.interaction.set_enabled(self.enabled);
        let mut node = AccessibilityNode::new(AccessibilityRole::Slider, bounds);
        node.label = self.label.clone();
        node.numeric_value = Some(self.current_value());
        node.numeric_minimum = Some(self.minimum);
        node.numeric_maximum = Some(self.maximum);
        node.enabled = self.enabled;
        node.focusable = self.enabled;
        node.focused = self.interaction.inner.borrow().focused;
        context.record_accessibility(node);
        let radius = bounds.size.width.min(bounds.size.height) * 0.5;
        let track = context.theme.slider.level_track;
        let fill = context.theme.slider.level_fill;
        let opacity = if self.enabled {
            1.0
        } else {
            context.theme.slider.disabled_opacity
        };
        context.display_list.push(DrawCommand::PushRoundedClip {
            rect: bounds,
            radius,
        });
        Rectangle::new()
            .color(RectangleColor::Custom(with_opacity(track, opacity)))
            .paint(bounds, context);
        let height = bounds.size.height * self.progress();
        Rectangle::new()
            .color(RectangleColor::Custom(with_opacity(fill, opacity)))
            .paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + bounds.size.height - height,
                    bounds.size.width,
                    height,
                ),
                context,
            );
        context.display_list.push(DrawCommand::PopClip);
        if node_focused(&self.interaction) {
            // Neutral inset cue for keyboard focus; pointer clicks do not add a blue border.
            context.display_list.push(DrawCommand::StrokeRoundedRect {
                rect: Rect::new(
                    bounds.origin.x + 2.0,
                    bounds.origin.y + 2.0,
                    (bounds.size.width - 4.0).max(0.0),
                    (bounds.size.height - 4.0).max(0.0),
                ),
                radius: (radius - 2.0).max(0.0),
                color: context.theme.colors.text_secondary,
                width: 1.0,
            });
        }
    }

    fn update_level(&self, bounds: Rect, y: f32) {
        if bounds.size.height <= 0.0 {
            return;
        }
        let progress = (1.0 - (y - bounds.origin.y) / bounds.size.height).clamp(0.0, 1.0);
        self.value.set_without_notification(
            self.sanitize_value(self.minimum + progress * (self.maximum - self.minimum)),
        );
    }

    pub(super) fn handle_level_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.interaction.set_enabled(self.enabled);
        if !self.enabled {
            return EventResult::Ignored;
        }
        match event {
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                self.interaction.inner.borrow_mut().focused = *target == Some(bounds);
                context.request_redraw_in(bounds);
                EventResult::Ignored
            }
            ViewEvent::KeyPressed {
                key: Key::ArrowUp | Key::ArrowRight,
                ..
            } => self.adjust_from_keyboard(1.0, bounds, context),
            ViewEvent::KeyPressed {
                key: Key::ArrowDown | Key::ArrowLeft,
                ..
            } => self.adjust_from_keyboard(-1.0, bounds, context),
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if bounds.contains(*position) => {
                self.interaction.inner.borrow_mut().dragging = true;
                self.update_level(bounds, position.y);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerMoved { position } if self.interaction.is_dragging() => {
                self.update_level(bounds, position.y);
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } if self.interaction.is_dragging() => {
                self.update_level(bounds, position.y);
                self.interaction.inner.borrow_mut().dragging = false;
                self.value.commit();
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::FocusChanged { focused: false } => {
                if self.interaction.is_dragging() {
                    self.value.commit();
                }
                self.interaction.reset();
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}
fn node_focused(interaction: &SliderInteractionState) -> bool {
    interaction.inner.borrow().focused
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{state::State, theme::Theme, typography::TextMeasurer};
    #[test]
    fn vertical_level_tracks_drag_clamps_and_obeys_disabled_and_focus() {
        let state = State::new(50.0);
        let interaction = SliderInteractionState::new();
        let slider = Slider::with_interaction(state.binding(), interaction.clone())
            .style(SliderStyle::VerticalLevel)
            .range(0.0..=100.0)
            .step(1.0);
        let theme = Theme::current();
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        let bounds = Rect::new(20.0, 10.0, 60.0, 130.0);
        let point = |y| crate::geometry::Point::new(40.0, y);
        slider.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position: point(75.0),
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert_eq!(state.get(), 50.0);
        assert!(interaction.is_dragging());
        slider.handle_event(
            bounds,
            &ViewEvent::PointerMoved {
                position: point(-20.0),
            },
            &mut context,
        );
        assert_eq!(state.get(), 100.0);
        slider.handle_event(
            bounds,
            &ViewEvent::PointerReleased {
                position: point(200.0),
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert_eq!(state.get(), 0.0);
        assert!(!interaction.is_dragging());
        slider.handle_event(
            bounds,
            &ViewEvent::KeyboardFocusRequested {
                bounds: Some(bounds),
            },
            &mut context,
        );
        slider.handle_event(
            bounds,
            &ViewEvent::KeyPressed {
                key: Key::ArrowUp,
                modifiers: Default::default(),
            },
            &mut context,
        );
        assert_eq!(state.get(), 1.0);
        let slider = slider.enabled(false);
        slider.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position: point(10.0),
                button: PointerButton::Primary,
            },
            &mut context,
        );
        assert_eq!(state.get(), 1.0);
        assert!(!interaction.is_dragging());
    }
}

use std::cell::RefCell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::{Key, PointerButton};
use crate::state::Binding;
use crate::theme::{Color, CornerRadius};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::omochi_shape::{OmochiPreset, OmochiShape};
use super::{Rectangle, RectangleColor, Text};

#[derive(Clone, Copy, Default)]
struct RangeInteraction {
    hovered: bool,
    focused: bool,
    dragging: Option<usize>,
    active_thumb: usize,
}

#[derive(Clone)]
pub struct RangeSliderInteractionState {
    inner: Rc<RefCell<RangeInteraction>>,
    lower_shape: OmochiShape,
    upper_shape: OmochiShape,
}

impl RangeSliderInteractionState {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(RangeInteraction::default())),
            lower_shape: OmochiShape::velocity(OmochiPreset::Thumb),
            upper_shape: OmochiShape::velocity(OmochiPreset::Thumb),
        }
    }
}

impl Default for RangeSliderInteractionState {
    fn default() -> Self {
        Self::new()
    }
}

/// A two-thumb range selector whose values follow input immediately while
/// omochi deformation remains purely visual.
pub struct RangeSlider {
    lower: Binding<f32>,
    upper: Binding<f32>,
    minimum: f32,
    maximum: f32,
    step: Option<f32>,
    label: Option<String>,
    enabled: bool,
    interaction: RangeSliderInteractionState,
}

impl RangeSlider {
    pub fn new(lower: Binding<f32>, upper: Binding<f32>) -> Self {
        Self::with_interaction(lower, upper, RangeSliderInteractionState::new())
    }

    pub fn with_interaction(
        lower: Binding<f32>,
        upper: Binding<f32>,
        interaction: RangeSliderInteractionState,
    ) -> Self {
        Self {
            lower,
            upper,
            minimum: 0.0,
            maximum: 1.0,
            step: None,
            label: None,
            enabled: true,
            interaction,
        }
    }

    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        let (start, end) = (*range.start(), *range.end());
        if start.is_finite() && end.is_finite() && start != end {
            self.minimum = start.min(end);
            self.maximum = start.max(end);
        }
        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = (step.is_finite() && step > 0.0).then_some(step);
        self
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn lower_value(&self) -> f32 {
        self.sanitize(self.lower.get()).min(self.upper_value_raw())
    }

    pub fn upper_value(&self) -> f32 {
        self.sanitize(self.upper.get()).max(self.lower_value_raw())
    }

    fn lower_value_raw(&self) -> f32 {
        self.sanitize(self.lower.get())
    }

    fn upper_value_raw(&self) -> f32 {
        self.sanitize(self.upper.get())
    }

    fn sanitize(&self, value: f32) -> f32 {
        let value = if value.is_finite() {
            value.clamp(self.minimum, self.maximum)
        } else {
            self.minimum
        };
        self.step.map_or(value, |step| {
            (((value - self.minimum) / step).round() * step + self.minimum)
                .clamp(self.minimum, self.maximum)
        })
    }

    fn progress(&self, value: f32) -> f32 {
        ((value - self.minimum) / (self.maximum - self.minimum)).clamp(0.0, 1.0)
    }

    fn geometry(&self, bounds: Rect, context: &impl RangeContext) -> (Rect, Rect, Rect) {
        let theme = context.theme();
        let thumb = theme.layout.control_thumb_size(false);
        let label_offset = self
            .label
            .as_ref()
            .map_or(0.0, |_| context.label_height() + theme.spacing.extra_small);
        let control = Rect::new(
            bounds.origin.x,
            bounds.origin.y + label_offset,
            bounds.size.width,
            (bounds.size.height - label_offset).max(0.0),
        );
        let track = Rect::new(
            control.origin.x + thumb.width / 2.0,
            control.origin.y + (control.size.height - theme.layout.range_track_height) / 2.0,
            (control.size.width - thumb.width).max(0.0),
            theme.layout.range_track_height,
        );
        let thumb_at = |value| {
            let center = track.origin.x + track.size.width * self.progress(value);
            Rect::new(
                center - thumb.width / 2.0,
                control.origin.y + (control.size.height - thumb.height) / 2.0,
                thumb.width,
                thumb.height,
            )
        };
        (
            track,
            thumb_at(self.lower_value()),
            thumb_at(self.upper_value()),
        )
    }

    fn value_at(&self, track: Rect, x: f32) -> f32 {
        if track.size.width <= 0.0 {
            return self.minimum;
        }
        self.sanitize(
            self.minimum
                + (self.maximum - self.minimum)
                    * ((x - track.origin.x) / track.size.width).clamp(0.0, 1.0),
        )
    }

    fn set_thumb(&self, thumb: usize, value: f32) -> bool {
        if thumb == 0 {
            self.lower
                .set_without_notification(value.min(self.upper_value()));
        } else {
            self.upper
                .set_without_notification(value.max(self.lower_value()));
        }
        true
    }

    fn shape(&self, thumb: usize) -> &OmochiShape {
        if thumb == 0 {
            &self.interaction.lower_shape
        } else {
            &self.interaction.upper_shape
        }
    }

    fn thumb_is_at_wall(&self, thumb: usize) -> bool {
        let scale = self.minimum.abs().max(self.maximum.abs()).max(1.0);
        let epsilon = f32::EPSILON * scale * 4.0;
        if thumb == 0 {
            let value = self.lower_value();
            value <= self.minimum + epsilon || value >= self.upper_value() - epsilon
        } else {
            let value = self.upper_value();
            value >= self.maximum - epsilon || value <= self.lower_value() + epsilon
        }
    }

    fn update_thumb_shape_for_drag(&self, thumb: usize, position: crate::geometry::Point) {
        let shape = self.shape(thumb);
        if self.thumb_is_at_wall(thumb) {
            if shape.is_active() {
                // The end stop (including the other thumb) absorbs further
                // pointer travel. Existing strain may only recover from here.
                shape.end();
            }
        } else if shape.is_active() {
            shape.moved(position);
        } else {
            // Do not turn pointer travel accumulated at either end stop (or at
            // the other thumb) into a velocity impulse when movement resumes.
            shape.begin(position);
        }
    }

    fn end_thumb_shape_drag(&self, thumb: usize, position: crate::geometry::Point) {
        self.update_thumb_shape_for_drag(thumb, position);
        if self.shape(thumb).is_active() {
            self.shape(thumb).end();
        }
    }
}

trait RangeContext {
    fn theme(&self) -> &crate::theme::Theme;
    fn label_height(&self) -> f32;
}

impl RangeContext for PaintContext<'_> {
    fn theme(&self) -> &crate::theme::Theme {
        self.theme
    }
    fn label_height(&self) -> f32 {
        self.typography.label.line_height
    }
}

impl RangeContext for EventContext<'_> {
    fn theme(&self) -> &crate::theme::Theme {
        self.theme()
    }
    fn label_height(&self) -> f32 {
        self.typography().label.line_height
    }
}

impl View for RangeSlider {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let mut height = context.theme.layout.range_control_height;
        if self.label.is_some() {
            height += context.typography.label.line_height + context.theme.spacing.extra_small;
        }
        constraints.constrain(Size::new(context.theme.layout.range_control_width, height))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if !self.enabled {
            self.interaction.lower_shape.reset();
            self.interaction.upper_shape.reset();
        }
        let (track, lower, upper) = self.geometry(bounds, context);
        let mut group = AccessibilityNode::new(AccessibilityRole::Group, bounds);
        group.label = self.label.clone();
        group.enabled = self.enabled;
        group.focusable = true;
        group.focused = self.interaction.inner.borrow().focused;
        context.record_accessibility(group);
        for (name, value, thumb) in [
            ("Minimum", self.lower_value(), lower),
            ("Maximum", self.upper_value(), upper),
        ] {
            let mut node = AccessibilityNode::new(AccessibilityRole::Slider, thumb);
            node.label = Some(name.into());
            node.numeric_value = Some(value);
            node.numeric_minimum = Some(self.minimum);
            node.numeric_maximum = Some(self.maximum);
            node.enabled = self.enabled;
            context.record_accessibility(node);
        }
        if let Some(label) = &self.label {
            Text::label(label).paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    context.typography.label.line_height,
                ),
                context,
            );
        }
        let opacity = if self.enabled {
            1.0
        } else {
            context.theme.slider.disabled_opacity
        };
        let alpha = |color: Color| color.with_alpha((color.alpha as f32 * opacity) as u8);
        Rectangle::new()
            .color(RectangleColor::Custom(alpha(context.theme.slider.track)))
            .radius(CornerRadius::Full)
            .paint(track, context);
        let fill = Rect::new(
            lower.origin.x + lower.size.width / 2.0,
            track.origin.y,
            (upper.origin.x - lower.origin.x).max(0.0),
            track.size.height,
        );
        Rectangle::new()
            .color(RectangleColor::Custom(alpha(context.theme.slider.fill)))
            .radius(CornerRadius::Full)
            .paint(fill, context);
        let interaction = *self.interaction.inner.borrow();
        for (index, thumb) in [lower, upper].into_iter().enumerate() {
            let color = if self.enabled && self.shape(index).is_animating() {
                context.theme.slider.hovered_knob
            } else if self.enabled {
                context.theme.slider.knob
            } else {
                context.theme.slider.disabled_knob
            };
            let radius = context.theme.slider.knob_radius.resolve(
                &context.theme.radius,
                thumb.size.width,
                thumb.size.height,
            );
            if interaction.focused && interaction.active_thumb == index {
                Rectangle::new()
                    .color(RectangleColor::Custom(context.theme.slider.focus_ring))
                    .radius(CornerRadius::Custom(
                        radius + context.theme.slider.focus_ring_width,
                    ))
                    .paint(
                        thumb.expanded(context.theme.slider.focus_ring_width),
                        context,
                    );
            }
            if self.shape(index).is_animating() {
                self.shape(index).paint_styled(
                    thumb,
                    radius,
                    color,
                    context.theme.slider.knob_border,
                    context.theme.slider.stroke_width,
                    context.theme.slider.knob_shadow,
                    context,
                );
            } else {
                Rectangle::new()
                    .color(RectangleColor::Custom(color))
                    .radius(context.theme.slider.knob_radius)
                    .border(super::BorderStyle::custom(
                        context.theme.slider.knob_border,
                        context.theme.slider.stroke_width,
                    ))
                    .shadow(context.theme.slider.knob_shadow)
                    .paint(thumb, context);
            }
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }
        let (track, lower, upper) = self.geometry(bounds, context);
        match event {
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                self.interaction.inner.borrow_mut().focused =
                    target.is_some_and(|target| target == bounds);
                EventResult::Ignored
            }
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } if bounds
                .expanded(context.theme().layout.range_hit_padding)
                .contains(*position) =>
            {
                let lower_distance = (position.x - (lower.origin.x + lower.size.width / 2.0)).abs();
                let upper_distance = (position.x - (upper.origin.x + upper.size.width / 2.0)).abs();
                let thumb = usize::from(upper_distance < lower_distance);
                let mut interaction = self.interaction.inner.borrow_mut();
                interaction.dragging = Some(thumb);
                interaction.active_thumb = thumb;
                drop(interaction);
                self.set_thumb(thumb, self.value_at(track, position.x));
                self.update_thumb_shape_for_drag(thumb, *position);
                context.request_redraw_in(bounds.expanded(20.0));
                EventResult::Consumed
            }
            ViewEvent::PointerMoved { position } => {
                let thumb = self.interaction.inner.borrow().dragging;
                if let Some(thumb) = thumb {
                    self.set_thumb(thumb, self.value_at(track, position.x));
                    self.update_thumb_shape_for_drag(thumb, *position);
                    context.request_redraw_in(bounds.expanded(20.0));
                    EventResult::Consumed
                } else {
                    self.interaction.inner.borrow_mut().hovered = bounds.contains(*position);
                    EventResult::Ignored
                }
            }
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let Some(thumb) = self.interaction.inner.borrow_mut().dragging.take() else {
                    return EventResult::Ignored;
                };
                self.set_thumb(thumb, self.value_at(track, position.x));
                self.end_thumb_shape_drag(thumb, *position);
                if thumb == 0 {
                    self.lower.commit();
                } else {
                    self.upper.commit();
                }
                context.request_redraw_in(bounds.expanded(20.0));
                EventResult::Consumed
            }
            ViewEvent::KeyPressed { key, .. }
                if self.interaction.inner.borrow().focused
                    && matches!(
                        key,
                        Key::ArrowLeft | Key::ArrowDown | Key::ArrowRight | Key::ArrowUp
                    ) =>
            {
                let interaction = *self.interaction.inner.borrow();
                let direction = if matches!(key, Key::ArrowLeft | Key::ArrowDown) {
                    -1.0
                } else {
                    1.0
                };
                let increment = self.step.unwrap_or((self.maximum - self.minimum) / 100.0);
                let current = if interaction.active_thumb == 0 {
                    self.lower_value()
                } else {
                    self.upper_value()
                };
                self.set_thumb(
                    interaction.active_thumb,
                    self.sanitize(current + increment * direction),
                );
                if interaction.active_thumb == 0 {
                    self.lower.commit();
                } else {
                    self.upper.commit();
                }
                context.request_redraw_in(bounds.expanded(20.0));
                EventResult::Consumed
            }
            ViewEvent::FocusChanged { focused: false } => {
                if let Some(thumb) = self.interaction.inner.borrow_mut().dragging.take() {
                    self.shape(thumb).end();
                    if thumb == 0 {
                        self.lower.commit();
                    } else {
                        self.upper.commit();
                    }
                }
                self.interaction.inner.borrow_mut().focused = false;
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventContext;
    use crate::geometry::Point;
    use crate::state::State;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn values_are_ordered_and_snapped() {
        let lower = State::new(82.0);
        let upper = State::new(18.0);
        let slider = RangeSlider::new(lower.binding(), upper.binding())
            .range(0.0..=100.0)
            .step(5.0);
        assert_eq!(slider.lower_value(), 20.0);
        assert_eq!(slider.upper_value(), 80.0);
    }

    #[test]
    fn end_stop_clears_thumb_velocity_until_it_leaves_the_wall() {
        let lower = State::new(0.25);
        let upper = State::new(0.75);
        let interaction = RangeSliderInteractionState::new();
        let slider =
            RangeSlider::with_interaction(lower.binding(), upper.binding(), interaction.clone());
        let bounds = Rect::new(0.0, 0.0, 200.0, 28.0);
        let mut text_measurer = TextMeasurer::new();
        let mut context =
            EventContext::new(&Theme::LIGHT, &Typography::DEFAULT, &mut text_measurer);

        slider.handle_event(
            bounds,
            &ViewEvent::PointerPressed {
                position: Point::new(56.5, 14.0),
                button: PointerButton::Primary,
            },
            &mut context,
        );
        slider.handle_event(
            bounds,
            &ViewEvent::PointerMoved {
                position: Point::new(-100.0, 14.0),
            },
            &mut context,
        );
        assert_eq!(lower.get(), 0.0);
        assert!(!interaction.lower_shape.is_active());
        assert!(
            interaction.lower_shape.is_animating(),
            "strain should recover at the wall instead of snapping"
        );

        slider.handle_event(
            bounds,
            &ViewEvent::PointerMoved {
                position: Point::new(80.0, 14.0),
            },
            &mut context,
        );
        assert!(lower.get() > 0.0);
        assert!(interaction.lower_shape.is_active());
    }
}

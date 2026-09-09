use std::cell::RefCell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::PointerButton;
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Ellipse, EllipseColor, Rectangle, RectangleColor, Text};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SliderInteractionInner {
    hovered: bool,
    dragging: bool,
    enabled: bool,
    drag_offset_x: f32,
}
#[derive(Clone)]
pub struct SliderInteractionState {
    inner: Rc<RefCell<SliderInteractionInner>>,
}

impl Default for SliderInteractionState {
    fn default() -> Self {
        Self {
            inner: Rc::new(RefCell::new(SliderInteractionInner {
                enabled: true,

                ..SliderInteractionInner::default()
            })),
        }
    }
}

impl SliderInteractionState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_hovered(&self) -> bool {
        self.inner.borrow().hovered
    }

    pub fn is_dragging(&self) -> bool {
        self.inner.borrow().dragging
    }

    pub fn is_enabled(&self) -> bool {
        self.inner.borrow().enabled
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();

        inner.hovered = false;
        inner.dragging = false;
        inner.drag_offset_x = 0.0;
    }

    fn set_enabled(&self, enabled: bool) -> bool {
        let mut inner = self.inner.borrow_mut();

        let changed = inner.enabled != enabled;

        inner.enabled = enabled;

        if !enabled {
            inner.hovered = false;
            inner.dragging = false;
            inner.drag_offset_x = 0.0;
        }

        changed
    }
}

pub struct Slider {
    value: Binding<f32>,

    minimum: f32,
    maximum: f32,
    step: Option<f32>,

    label: Option<String>,
    enabled: bool,

    interaction: SliderInteractionState,
}

impl Slider {
    pub fn new(value: Binding<f32>) -> Self {
        Self {
            value,

            minimum: 0.0,
            maximum: 1.0,
            step: None,

            label: None,
            enabled: true,

            interaction: SliderInteractionState::new(),
        }
    }

    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        let start = *range.start();
        let end = *range.end();

        if !start.is_finite() || !end.is_finite() {
            return self;
        }

        let minimum = start.min(end);
        let maximum = start.max(end);

        if minimum == maximum {
            self.minimum = minimum;
            self.maximum = minimum + 1.0;
        } else {
            self.minimum = minimum;
            self.maximum = maximum;
        }

        self
    }

    pub fn step(mut self, step: f32) -> Self {
        self.step = if step.is_finite() && step > 0.0 {
            Some(step)
        } else {
            None
        };

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

    pub fn current_value(&self) -> f32 {
        self.sanitize_value(self.value.get())
    }

    pub fn interaction(&self) -> &SliderInteractionState {
        &self.interaction
    }

    fn sanitize_value(&self, value: f32) -> f32 {
        if !value.is_finite() {
            return self.minimum;
        }

        let value = value.clamp(self.minimum, self.maximum);

        let Some(step) = self.step else {
            return value;
        };

        let stepped = ((value - self.minimum) / step).round() * step + self.minimum;

        stepped.clamp(self.minimum, self.maximum)
    }

    fn progress(&self) -> f32 {
        let range = self.maximum - self.minimum;

        if range <= 0.0 || !range.is_finite() {
            return 0.0;
        }

        ((self.current_value() - self.minimum) / range).clamp(0.0, 1.0)
    }

    fn slider_bounds(&self, bounds: Rect, label_height: f32, label_spacing: f32) -> Rect {
        if self.label.is_some() {
            Rect::new(
                bounds.origin.x,
                bounds.origin.y + label_height + label_spacing,
                bounds.size.width,
                (bounds.size.height - label_height - label_spacing).max(0.0),
            )
        } else {
            bounds
        }
    }

    fn track_bounds(&self, bounds: Rect, knob_size: f32, track_height: f32) -> Rect {
        let knob_radius = knob_size / 2.0;

        let width = (bounds.size.width - knob_size).max(0.0);

        Rect::new(
            bounds.origin.x + knob_radius,
            bounds.origin.y + (bounds.size.height - track_height) / 2.0,
            width,
            track_height,
        )
    }

    fn hit_bounds(
        &self,
        bounds: Rect,
        label_height: f32,
        label_spacing: f32,
        padding: f32,
    ) -> Rect {
        self.slider_bounds(bounds, label_height, label_spacing)
            .expanded(padding)
    }

    fn knob_center_x(
        &self,
        bounds: Rect,
        label_height: f32,
        label_spacing: f32,
        knob_size: f32,
        track_height: f32,
    ) -> f32 {
        let track = self.track_bounds(
            self.slider_bounds(bounds, label_height, label_spacing),
            knob_size,
            track_height,
        );

        track.origin.x + track.size.width * self.progress()
    }

    fn knob_bounds(
        &self,
        bounds: Rect,
        label_height: f32,
        label_spacing: f32,
        knob_size: f32,
        track_height: f32,
    ) -> Rect {
        let slider_bounds = self.slider_bounds(bounds, label_height, label_spacing);

        let center_x =
            self.knob_center_x(bounds, label_height, label_spacing, knob_size, track_height);

        Rect::new(
            center_x - knob_size / 2.0,
            slider_bounds.origin.y + (slider_bounds.size.height - knob_size) / 2.0,
            knob_size,
            knob_size,
        )
    }

    fn value_from_pointer(
        &self,
        bounds: Rect,
        pointer_x: f32,
        label_height: f32,
        label_spacing: f32,
        knob_size: f32,
        track_height: f32,
    ) -> f32 {
        let track = self.track_bounds(
            self.slider_bounds(bounds, label_height, label_spacing),
            knob_size,
            track_height,
        );

        if track.size.width <= 0.0 {
            return self.minimum;
        }

        let progress = ((pointer_x - track.origin.x) / track.size.width).clamp(0.0, 1.0);

        self.sanitize_value(self.minimum + (self.maximum - self.minimum) * progress)
    }

    fn update_from_pointer(
        &self,
        bounds: Rect,
        pointer_x: f32,
        drag_offset_x: f32,
        label_height: f32,
        label_spacing: f32,
        knob_size: f32,
        track_height: f32,
    ) -> bool {
        let value = self.value_from_pointer(
            bounds,
            pointer_x - drag_offset_x,
            label_height,
            label_spacing,
            knob_size,
            track_height,
        );

        let current = self.current_value();

        if values_equal(current, value) {
            return false;
        }

        self.value.set_without_notification(value);

        true
    }
}

impl View for Slider {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let label_height = context.typography.label.line_height;
        let label_spacing = context.theme.spacing.extra_small;
        let slider_height = context.theme.layout.range_control_height;
        let height = if self.label.is_some() {
            label_height + label_spacing + slider_height
        } else {
            slider_height
        };

        constraints.constrain(Size::new(context.theme.layout.range_control_width, height))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        self.interaction.set_enabled(self.enabled);

        if let Some(label) = self.label.as_ref() {
            Text::label(label.as_str())
                .color(if self.enabled {
                    context.theme.colors.text_primary
                } else {
                    context.theme.colors.text_disabled
                })
                .paint(
                    Rect::new(
                        bounds.origin.x,
                        bounds.origin.y,
                        bounds.size.width,
                        context.typography.label.line_height,
                    ),
                    context,
                );
        }

        let label_height = context.typography.label.line_height;
        let label_spacing = context.theme.spacing.extra_small;
        let knob_size = context.theme.layout.range_knob_size;
        let track_height = context.theme.layout.range_track_height;
        let slider_bounds = self.slider_bounds(bounds, label_height, label_spacing);

        let track_bounds = self.track_bounds(slider_bounds, knob_size, track_height);

        let progress = self.progress();

        let hovered = self.interaction.is_hovered();

        let dragging = self.interaction.is_dragging();

        let background_color = if self.enabled {
            context.theme.colors.surface_muted
        } else {
            with_opacity(context.theme.colors.surface_muted, 0.45)
        };

        let accent_color = if !self.enabled {
            with_opacity(context.theme.colors.accent, 0.45)
        } else if dragging {
            context.theme.colors.accent_pressed
        } else if hovered {
            context.theme.colors.accent_hovered
        } else {
            context.theme.colors.accent
        };

        Rectangle::new()
            .color(RectangleColor::Custom(background_color))
            .radius(CornerRadius::Full)
            .shadow(ShadowStyle::None)
            .paint(track_bounds, context);

        let filled_width = track_bounds.size.width * progress;

        if filled_width > 0.0 {
            Rectangle::new()
                .color(RectangleColor::Custom(accent_color))
                .radius(CornerRadius::Full)
                .shadow(ShadowStyle::None)
                .paint(
                    Rect::new(
                        track_bounds.origin.x,
                        track_bounds.origin.y,
                        filled_width,
                        track_bounds.size.height,
                    ),
                    context,
                );
        }

        let knob_center_x = track_bounds.origin.x + track_bounds.size.width * progress;

        let knob_size = if dragging {
            context.theme.layout.range_dragging_knob_size
        } else {
            knob_size
        };

        let knob_bounds = Rect::new(
            knob_center_x - knob_size / 2.0,
            slider_bounds.origin.y + (slider_bounds.size.height - knob_size) / 2.0,
            knob_size,
            knob_size,
        );

        let knob_color = if !self.enabled {
            context.theme.colors.surface_subtle
        } else if dragging {
            context.theme.colors.surface
        } else if hovered {
            context.theme.colors.surface
        } else {
            context.theme.colors.elevated_surface
        };

        let knob_shadow = if self.enabled {
            ShadowStyle::None
        } else {
            ShadowStyle::None
        };

        Ellipse::new()
            .color(EllipseColor::Custom(knob_color))
            .border(super::BorderStyle::custom(
                if hovered {
                    context.theme.colors.accent
                } else {
                    context.theme.colors.border
                },
                1.0,
            ))
            .shadow(knob_shadow)
            .paint(knob_bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let enabled_changed = self.interaction.set_enabled(self.enabled);

        if enabled_changed {
            context.request_redraw_in(bounds.expanded(16.0));
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        let label_height = context.typography().label.line_height;
        let label_spacing = context.theme().spacing.extra_small;
        let knob_size = context.theme().layout.range_knob_size;
        let track_height = context.theme().layout.range_track_height;
        let hit_bounds = self.hit_bounds(
            bounds,
            label_height,
            label_spacing,
            context.theme().layout.range_hit_padding,
        );

        match event {
            ViewEvent::PointerMoved { position } => {
                let (state_changed, dragging, drag_offset_x) = {
                    let mut inner = self.interaction.inner.borrow_mut();

                    let hovered = hit_bounds.contains(*position);

                    let state_changed = inner.hovered != hovered;

                    inner.hovered = hovered;

                    (state_changed, inner.dragging, inner.drag_offset_x)
                };

                let value_changed = if dragging {
                    self.update_from_pointer(
                        bounds,
                        position.x,
                        drag_offset_x,
                        label_height,
                        label_spacing,
                        knob_size,
                        track_height,
                    )
                } else {
                    false
                };

                if state_changed || value_changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                if dragging {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !hit_bounds.contains(*position) {
                    return EventResult::Ignored;
                }

                let knob_bounds =
                    self.knob_bounds(bounds, label_height, label_spacing, knob_size, track_height);

                let pressed_inside_knob = knob_bounds.contains(*position);

                let drag_offset_x = if pressed_inside_knob {
                    position.x
                        - self.knob_center_x(
                            bounds,
                            label_height,
                            label_spacing,
                            knob_size,
                            track_height,
                        )
                } else {
                    0.0
                };

                {
                    let mut inner = self.interaction.inner.borrow_mut();

                    inner.hovered = true;
                    inner.dragging = true;
                    inner.drag_offset_x = drag_offset_x;
                }

                if !pressed_inside_knob {
                    self.update_from_pointer(
                        bounds,
                        position.x,
                        0.0,
                        label_height,
                        label_spacing,
                        knob_size,
                        track_height,
                    );
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let (was_dragging, drag_offset_x) = {
                    let mut inner = self.interaction.inner.borrow_mut();

                    let was_dragging = inner.dragging;

                    let drag_offset_x = inner.drag_offset_x;

                    inner.dragging = false;
                    inner.drag_offset_x = 0.0;
                    inner.hovered = hit_bounds.contains(*position);

                    (was_dragging, drag_offset_x)
                };

                if !was_dragging {
                    return EventResult::Ignored;
                }

                self.update_from_pointer(
                    bounds,
                    position.x,
                    drag_offset_x,
                    label_height,
                    label_spacing,
                    knob_size,
                    track_height,
                );

                self.value.commit();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                let changed = {
                    let mut inner = self.interaction.inner.borrow_mut();

                    let changed = inner.hovered;

                    inner.hovered = false;

                    changed
                };

                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }

                EventResult::Ignored
            }

            ViewEvent::FocusChanged { focused: false } => {
                let was_dragging = {
                    let mut inner = self.interaction.inner.borrow_mut();

                    let was_dragging = inner.dragging;

                    inner.hovered = false;
                    inner.dragging = false;

                    was_dragging
                };

                if was_dragging {
                    self.value.commit();
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Ignored
            }

            _ => EventResult::Ignored,
        }
    }
}

fn values_equal(first: f32, second: f32) -> bool {
    let scale = first.abs().max(second.abs()).max(1.0);

    (first - second).abs() <= f32::EPSILON * scale * 4.0
}

fn with_opacity(color: Color, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };

    color.with_alpha((color.alpha as f32 * opacity).round() as u8)
}

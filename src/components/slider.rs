mod level;

use std::cell::RefCell;
use std::ops::RangeInclusive;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::omochi::moving::PressResponse;
use crate::omochi::{Material, OmochiSurface, button_contour};
use crate::platform::PointerButton;
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, ShadowStyle};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::time::Instant;

use super::{Rectangle, RectangleColor, Text};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct SliderInteractionInner {
    hovered: bool,
    dragging: bool,
    focused: bool,
    enabled: bool,
    drag_offset_x: f32,
}

#[derive(Clone, Copy)]
struct SliderMetrics {
    label_height: f32,
    label_spacing: f32,
    thumb_width: f32,
    thumb_height: f32,
    track_height: f32,
    hit_padding: f32,
}
#[derive(Default)]
struct SliderSurface {
    surface: OmochiSurface,
    press: PressResponse,
}
#[derive(Clone)]
pub struct SliderInteractionState {
    surface: Rc<RefCell<SliderSurface>>,
    inner: Rc<RefCell<SliderInteractionInner>>,
}

impl Default for SliderInteractionState {
    fn default() -> Self {
        Self {
            surface: Rc::new(RefCell::new(SliderSurface::default())),
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
        *self.surface.borrow_mut() = SliderSurface::default();
        let mut inner = self.inner.borrow_mut();

        inner.hovered = false;
        inner.dragging = false;
        inner.focused = false;
        inner.drag_offset_x = 0.0;
    }

    fn set_enabled(&self, enabled: bool) -> bool {
        let mut inner = self.inner.borrow_mut();

        let changed = inner.enabled != enabled;

        inner.enabled = enabled;

        if !enabled {
            *self.surface.borrow_mut() = SliderSurface::default();
            inner.hovered = false;
            inner.dragging = false;
            inner.focused = false;
            inner.drag_offset_x = 0.0;
        }

        changed
    }
}

/// Explicit slider presentations. Standard sliders keep their existing layout and colors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SliderStyle {
    #[default]
    Standard,
    VerticalLevel,
}

pub struct Slider {
    style: SliderStyle,
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
        Self::with_interaction(value, SliderInteractionState::new())
    }

    pub fn with_interaction(value: Binding<f32>, interaction: SliderInteractionState) -> Self {
        Self {
            value,
            style: SliderStyle::Standard,

            minimum: 0.0,
            maximum: 1.0,
            step: None,

            label: None,
            enabled: true,

            interaction,
        }
    }

    pub fn style(mut self, style: SliderStyle) -> Self {
        self.style = style;
        self
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

    fn track_bounds(&self, bounds: Rect, thumb_width: f32, track_height: f32) -> Rect {
        let thumb_radius = thumb_width / 2.0;

        let width = (bounds.size.width - thumb_width).max(0.0);

        Rect::new(
            bounds.origin.x + thumb_radius,
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
        thumb_width: f32,
        track_height: f32,
    ) -> f32 {
        let track = self.track_bounds(
            self.slider_bounds(bounds, label_height, label_spacing),
            thumb_width,
            track_height,
        );

        track.origin.x + track.size.width * self.progress()
    }

    fn knob_bounds(
        &self,
        bounds: Rect,
        label_height: f32,
        label_spacing: f32,
        thumb_width: f32,
        thumb_height: f32,
        track_height: f32,
    ) -> Rect {
        let slider_bounds = self.slider_bounds(bounds, label_height, label_spacing);

        let center_x = self.knob_center_x(
            bounds,
            label_height,
            label_spacing,
            thumb_width,
            track_height,
        );

        Rect::new(
            center_x - thumb_width / 2.0,
            slider_bounds.origin.y + (slider_bounds.size.height - thumb_height) / 2.0,
            thumb_width,
            thumb_height,
        )
    }

    fn value_from_pointer(&self, bounds: Rect, pointer_x: f32, metrics: SliderMetrics) -> f32 {
        let track = self.track_bounds(
            self.slider_bounds(bounds, metrics.label_height, metrics.label_spacing),
            metrics.thumb_width,
            metrics.track_height,
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
        metrics: SliderMetrics,
    ) -> bool {
        let value = self.value_from_pointer(bounds, pointer_x - drag_offset_x, metrics);

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
        if self.style == SliderStyle::VerticalLevel {
            return constraints.constrain(Size::new(60.0, 130.0));
        }
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
        if self.style == SliderStyle::VerticalLevel {
            self.paint_level(bounds, context);
            return;
        }
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        self.interaction.set_enabled(self.enabled);

        let mut accessibility = AccessibilityNode::new(AccessibilityRole::Slider, bounds);
        accessibility.label = self.label.clone();
        accessibility.numeric_value = Some(self.current_value());
        accessibility.numeric_minimum = Some(self.minimum);
        accessibility.numeric_maximum = Some(self.maximum);
        accessibility.enabled = self.enabled;
        accessibility.focusable = true;
        accessibility.focused = self.interaction.inner.borrow().focused;
        context.record_accessibility(accessibility);

        if let Some(label) = self.label.as_ref() {
            Text::label(label.as_str())
                .accessibility_hidden(true)
                .color(if self.enabled {
                    context.theme.slider.label
                } else {
                    context.theme.slider.disabled_label
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
        let thumb = context.theme.layout.control_thumb_size(false);
        let track_height = context.theme.layout.range_track_height;
        let slider_bounds = self.slider_bounds(bounds, label_height, label_spacing);

        let track_bounds = self.track_bounds(slider_bounds, thumb.width, track_height);

        let progress = self.progress();

        let hovered = self.interaction.is_hovered();

        let dragging = self.interaction.is_dragging();

        let background_color = if self.enabled {
            context.theme.slider.track
        } else {
            with_opacity(
                context.theme.slider.track,
                context.theme.slider.disabled_opacity,
            )
        };

        let accent_color = if !self.enabled {
            with_opacity(
                context.theme.slider.fill,
                context.theme.slider.disabled_opacity,
            )
        } else if dragging {
            context.theme.slider.pressed_fill
        } else if hovered {
            context.theme.slider.hovered_fill
        } else {
            context.theme.slider.fill
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

        let now = Instant::now();
        let mut motion = context.theme.motion.omochi;
        motion.enabled &= self.enabled;
        motion.material = Material::SLIDER.for_height(thumb.height * 30.0 / 20.0);
        let mut material_state = self.interaction.surface.borrow_mut();
        let mut sample = material_state.surface.sample(motion, now);
        sample.anchor = Point::new(thumb.width * 0.5, thumb.height * 0.5);
        let (pressure, press_redraw) = material_state.press.sample(dragging, motion.enabled, now);
        let press_scale = 1.0 + pressure * 0.20;
        let knob_bounds = Rect::new(
            knob_center_x - thumb.width * 0.5,
            slider_bounds.origin.y + (slider_bounds.size.height - thumb.height) * 0.5,
            thumb.width,
            thumb.height,
        );
        if let Some(next) = sample.next_redraw_at.into_iter().chain(press_redraw).min() {
            context.request_redraw_in_at(
                bounds.expanded(motion.material.max_pull + thumb.width * 0.2 + 6.0),
                next,
            );
        }
        drop(material_state);

        let knob_color = if !self.enabled {
            context.theme.slider.disabled_knob
        } else if dragging || hovered {
            context.theme.slider.hovered_knob
        } else {
            context.theme.slider.knob
        };

        let knob_shadow = ShadowStyle::None;
        let knob_radius = context.theme.slider.knob_radius.resolve(
            &context.theme.radius,
            knob_bounds.size.width,
            knob_bounds.size.height,
        );

        let interaction = self.interaction.inner.borrow();
        if interaction.focused && interaction.hovered {
            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.slider.focus_ring))
                .radius(CornerRadius::Custom(
                    knob_radius + context.theme.slider.focus_ring_width,
                ))
                .shadow(ShadowStyle::None)
                .paint(
                    knob_bounds.expanded(context.theme.slider.focus_ring_width),
                    context,
                );
        }
        drop(interaction);

        if pressure > 0.001 || sample.pull.x.hypot(sample.pull.y) > 0.001 {
            let center = Point::new(knob_center_x, knob_bounds.origin.y + thumb.height * 0.5);
            let points = button_contour(knob_bounds, knob_radius, sample, motion.material)
                .into_iter()
                .map(|p| {
                    Point::new(
                        center.x + (p.x - center.x) * press_scale,
                        center.y + (p.y - center.y) * press_scale,
                    )
                })
                .collect();
            context.display_list.push(DrawCommand::DrawContour {
                points,
                fill: knob_color,
                stroke: if hovered {
                    context.theme.slider.hovered_knob_border
                } else {
                    context.theme.slider.knob_border
                },
                stroke_width: context.theme.slider.stroke_width,
            });
        } else {
            Rectangle::new()
                .color(RectangleColor::Custom(knob_color))
                .radius(context.theme.slider.knob_radius)
                .border(super::BorderStyle::custom(
                    if hovered {
                        context.theme.slider.hovered_knob_border
                    } else {
                        context.theme.slider.knob_border
                    },
                    context.theme.slider.stroke_width,
                ))
                .shadow(knob_shadow)
                .paint(knob_bounds, context);
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if self.style == SliderStyle::VerticalLevel {
            return self.handle_level_event(bounds, event, context);
        }
        let enabled_changed = self.interaction.set_enabled(self.enabled);

        if enabled_changed {
            context.request_redraw_in(bounds.expanded(16.0));
        }

        if !self.enabled {
            return EventResult::Ignored;
        }

        let thumb = context.theme().layout.control_thumb_size(false);
        let metrics = SliderMetrics {
            label_height: context.typography().label.line_height,
            label_spacing: context.theme().spacing.extra_small,
            thumb_width: thumb.width,
            thumb_height: thumb.height,
            track_height: context.theme().layout.range_track_height,
            hit_padding: context.theme().layout.range_hit_padding,
        };
        let hit_bounds = self.hit_bounds(
            bounds,
            metrics.label_height,
            metrics.label_spacing,
            metrics.hit_padding,
        );

        let mut motion = context.theme().motion.omochi;
        motion.material = Material::SLIDER.for_height(thumb.height * 30.0 / 20.0);
        let now = Instant::now();
        if motion.enabled {
            let mut state = self.interaction.surface.borrow_mut();
            match event {
                ViewEvent::PointerPressed {
                    position,
                    button: PointerButton::Primary,
                } if hit_bounds.contains(*position) => {
                    state
                        .surface
                        .begin_velocity(*position, Point::new(-1.0, -0.12), now)
                }
                ViewEvent::PointerMoved { position } => state.surface.move_to(*position, now),
                ViewEvent::PointerReleased {
                    button: PointerButton::Primary,
                    ..
                }
                | ViewEvent::FocusChanged { focused: false } => state.surface.end(motion, now),
                _ => {}
            }
        }

        match event {
            ViewEvent::KeyboardFocusRequested { bounds: target } => {
                let focused = target.is_some_and(|target| target == bounds);
                let mut inner = self.interaction.inner.borrow_mut();
                let changed = inner.focused != focused;
                inner.focused = focused;
                drop(inner);
                if changed {
                    context.request_redraw_in(bounds.expanded(16.0));
                }
                EventResult::Ignored
            }

            ViewEvent::ArrowLeft => self.adjust_from_keyboard(-1.0, bounds, context),
            ViewEvent::ArrowRight => self.adjust_from_keyboard(1.0, bounds, context),

            ViewEvent::KeyPressed { key, .. } => match key {
                crate::platform::Key::ArrowLeft | crate::platform::Key::ArrowDown => {
                    self.adjust_from_keyboard(-1.0, bounds, context)
                }
                crate::platform::Key::ArrowRight | crate::platform::Key::ArrowUp => {
                    self.adjust_from_keyboard(1.0, bounds, context)
                }
                _ => EventResult::Ignored,
            },

            ViewEvent::PointerMoved { position } => {
                let (state_changed, dragging, drag_offset_x) = {
                    let mut inner = self.interaction.inner.borrow_mut();

                    let hovered = hit_bounds.contains(*position);

                    let state_changed = inner.hovered != hovered;

                    inner.hovered = hovered;

                    (state_changed, inner.dragging, inner.drag_offset_x)
                };

                let value_changed = if dragging {
                    self.update_from_pointer(bounds, position.x, drag_offset_x, metrics)
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

                let knob_bounds = self.knob_bounds(
                    bounds,
                    metrics.label_height,
                    metrics.label_spacing,
                    metrics.thumb_width,
                    metrics.thumb_height,
                    metrics.track_height,
                );

                let pressed_inside_knob = knob_bounds.contains(*position);

                let drag_offset_x = if pressed_inside_knob {
                    position.x
                        - self.knob_center_x(
                            bounds,
                            metrics.label_height,
                            metrics.label_spacing,
                            metrics.thumb_width,
                            metrics.track_height,
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
                    self.update_from_pointer(bounds, position.x, 0.0, metrics);
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

                self.update_from_pointer(bounds, position.x, drag_offset_x, metrics);

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
                    inner.focused = false;

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

impl Slider {
    fn adjust_from_keyboard(
        &self,
        direction: f32,
        bounds: Rect,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !self.interaction.inner.borrow().focused {
            return EventResult::Ignored;
        }
        let increment = self
            .step
            .unwrap_or_else(|| ((self.maximum - self.minimum) / 100.0).max(f32::EPSILON));
        let value = self.sanitize_value(self.current_value() + increment * direction);
        if !values_equal(value, self.current_value()) {
            self.value.set_without_notification(value);
            self.value.commit();
            context.request_redraw_in(bounds.expanded(16.0));
        }
        EventResult::Consumed
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

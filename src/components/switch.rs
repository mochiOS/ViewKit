use super::omochi_shape::{OmochiPreset, OmochiShape};
use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};
use crate::accessibility::AccessibilityRole;
use crate::animation::{Animation, Transition, interpolate};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::platform::PointerButton;
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, Motion, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Copy)]
struct SwitchMetrics {
    track_width: f32,
    track_height: f32,
    knob_width: f32,
    knob_height: f32,
    pressed_knob_width: f32,
    knob_inset: f32,
    drag_threshold: f32,
    hit_padding: f32,
}

impl SwitchMetrics {
    fn from_theme(theme: &Theme) -> Self {
        let layout = theme.layout;
        let maximum_width = (layout.switch_track_width - layout.switch_knob_inset * 2.0).max(1.0);
        let maximum_height = (layout.switch_track_height - layout.switch_knob_inset * 2.0).max(1.0);
        let knob_height = layout.switch_knob_height.min(maximum_height);
        let knob_width = layout.switch_knob_width.min(maximum_width);
        let pressed_knob_width = layout
            .switch_pressed_knob_width
            .max(knob_width)
            .min(maximum_width);
        Self {
            track_width: layout.switch_track_width,
            track_height: layout.switch_track_height,
            knob_width,
            knob_height,
            pressed_knob_width,
            knob_inset: layout.switch_knob_inset,
            drag_threshold: layout.switch_drag_threshold,
            hit_padding: layout.switch_hit_padding,
        }
    }
}

#[derive(Clone, Copy)]
struct SwitchPositionAnimation {
    from: f32,
    to: f32,
    started_at: Instant,
}

#[derive(Default)]
struct SwitchDragInner {
    mark_bounds: Option<Rect>,

    tracking: bool,
    drag_candidate: bool,
    dragging: bool,

    press_x: f32,
    drag_offset_x: f32,

    drag_position: Option<f32>,
    settle_animation: Option<SwitchPositionAnimation>,
}

#[derive(Clone, Default)]
struct SwitchDragState {
    inner: Rc<RefCell<SwitchDragInner>>,
}

#[derive(Clone, Copy)]
struct KnobWidthAnimation {
    from: f32,
    to: f32,
    started_at: Instant,
}

#[derive(Default)]
struct KnobWidthAnimationState {
    pressed: bool,
    animation: Option<KnobWidthAnimation>,
}

#[derive(Clone)]
pub struct SwitchInteractionState {
    button: ButtonInteractionState,
    knob_width_animation: Arc<Mutex<KnobWidthAnimationState>>,
    drag: SwitchDragState,
    thumb_shape: OmochiShape,
}

impl SwitchInteractionState {
    pub fn new() -> Self {
        Self {
            button: ButtonInteractionState::new(),
            knob_width_animation: Arc::new(Mutex::new(KnobWidthAnimationState::default())),
            drag: SwitchDragState::default(),
            thumb_shape: OmochiShape::velocity(OmochiPreset::Thumb),
        }
    }
}

impl Default for SwitchInteractionState {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Switch {
    checked: Binding<bool>,
    label: Option<String>,
    enabled: bool,
    interaction_state: SwitchInteractionState,
}

impl Switch {
    pub fn new(checked: Binding<bool>) -> Self {
        Self::with_interaction(checked, SwitchInteractionState::new())
    }

    pub fn with_interaction(
        checked: Binding<bool>,
        interaction_state: SwitchInteractionState,
    ) -> Self {
        Self {
            checked,
            label: None,
            enabled: true,
            interaction_state,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn is_checked(&self) -> bool {
        self.checked.get()
    }

    pub fn interaction(&self) -> &ButtonInteractionState {
        &self.interaction_state.button
    }

    fn button(&self, theme: &Theme) -> Button {
        let metrics = SwitchMetrics::from_theme(theme);
        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small);

        if let Some(label) = self.label.as_ref() {
            content = content.child(
                Text::label(label.clone())
                    .accessibility_hidden(true)
                    .color(if self.enabled {
                        theme.switch.foreground
                    } else {
                        theme.switch.disabled_foreground
                    })
                    .layout()
                    .flex_shrink(0.0),
            );
        }

        content = content.child(
            SwitchMark {
                checked: self.checked.get(),
                transition: self.checked.transition(),
                enabled: self.enabled,
                interaction: self.interaction_state.button.clone(),
                knob_width_animation: self.interaction_state.knob_width_animation.clone(),
                drag: self.interaction_state.drag.clone(),
                checked_binding: self.checked.clone(),
                thumb_shape: self.interaction_state.thumb_shape.clone(),
            }
            .frame(metrics.track_width, metrics.track_height)
            .flex_shrink(0.0),
        );

        Button::with_interaction(self.interaction_state.button.clone())
            .style(ButtonStyle::Custom {
                background: theme.switch.interaction_background,
                hovered_background: theme.switch.interaction_background,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.switch.foreground,
            })
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::all(theme.switch.content_padding).content(content))
            .accessibility_role(AccessibilityRole::Switch)
            .accessibility_checked(self.checked.get())
            .accessibility_label_option(self.label.clone())
    }

    fn handle_switch_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if !self.enabled {
            return EventResult::Ignored;
        }

        let metrics = SwitchMetrics::from_theme(context.theme());

        match event {
            ViewEvent::KeyPressed {
                key: crate::platform::Key::Enter | crate::platform::Key::Space,
                ..
            } if self.interaction_state.button.is_focused() => {
                self.start_toggle();
                context.request_redraw_in(bounds.expanded(16.0));
                EventResult::Consumed
            }

            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !bounds.contains(*position) {
                    return EventResult::Ignored;
                }

                let checked_position = bool_position(self.checked.get());

                let mut drag = self.interaction_state.drag.inner.borrow_mut();

                let mark_bounds = drag.mark_bounds;

                let drag_candidate = mark_bounds
                    .map(|mark_bounds| {
                        mark_bounds
                            .expanded(metrics.hit_padding)
                            .contains(*position)
                    })
                    .unwrap_or(false);

                let drag_offset_x = mark_bounds
                    .filter(|mark_bounds| {
                        knob_bounds_at(*mark_bounds, metrics.knob_width, checked_position, metrics)
                            .expanded(metrics.hit_padding)
                            .contains(*position)
                    })
                    .map(|mark_bounds| {
                        position.x
                            - knob_center_x(
                                mark_bounds,
                                metrics.knob_width,
                                checked_position,
                                metrics,
                            )
                    })
                    .unwrap_or(0.0);

                drag.tracking = true;
                drag.drag_candidate = drag_candidate;
                drag.dragging = false;
                drag.press_x = position.x;
                drag.drag_offset_x = drag_offset_x;
                drag.drag_position = Some(checked_position);
                drag.settle_animation = None;
                drop(drag);
                self.update_thumb_shape_for_drag(checked_position, *position);

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerMoved { position } => {
                let tracking = self.interaction_state.drag.inner.borrow().tracking;

                if !tracking {
                    return EventResult::Ignored;
                }

                let drag_position = {
                    let mut drag = self.interaction_state.drag.inner.borrow_mut();

                    if !drag.dragging {
                        let moved = (position.x - drag.press_x).abs();

                        if drag.drag_candidate && moved >= metrics.drag_threshold {
                            drag.dragging = true;
                        }
                    }

                    if drag.dragging
                        && let Some(mark_bounds) = drag.mark_bounds
                    {
                        drag.drag_position = Some(drag_progress_from_pointer(
                            mark_bounds,
                            position.x,
                            drag.drag_offset_x,
                            metrics,
                        ));
                    }
                    drag.dragging.then_some(drag.drag_position).flatten()
                };

                if let Some(drag_position) = drag_position {
                    self.update_thumb_shape_for_drag(drag_position, *position);
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let release = {
                    let mut drag = self.interaction_state.drag.inner.borrow_mut();

                    if !drag.tracking {
                        None
                    } else {
                        let was_dragging = drag.dragging;

                        let final_position = if was_dragging {
                            drag.mark_bounds
                                .map(|mark_bounds| {
                                    drag_progress_from_pointer(
                                        mark_bounds,
                                        position.x,
                                        drag.drag_offset_x,
                                        metrics,
                                    )
                                })
                                .or(drag.drag_position)
                                .unwrap_or_else(|| bool_position(self.checked.get()))
                        } else {
                            bool_position(self.checked.get())
                        };

                        drag.tracking = false;
                        drag.drag_candidate = false;
                        drag.dragging = false;
                        drag.press_x = 0.0;
                        drag.drag_offset_x = 0.0;
                        drag.drag_position = None;

                        Some((was_dragging, final_position))
                    }
                };

                let Some((was_dragging, final_position)) = release else {
                    return EventResult::Ignored;
                };

                if was_dragging {
                    self.update_thumb_shape_for_drag(final_position, *position);
                }
                if self.interaction_state.thumb_shape.is_active() {
                    self.interaction_state.thumb_shape.end();
                }

                if was_dragging {
                    self.start_settle(final_position, final_position >= 0.5);
                } else if bounds.contains(*position) {
                    self.start_toggle();
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                if self.interaction_state.drag.inner.borrow().tracking {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::FocusChanged { focused: false } => {
                let _final_position = {
                    let mut drag = self.interaction_state.drag.inner.borrow_mut();

                    if !drag.tracking {
                        return EventResult::Ignored;
                    }

                    let position = drag
                        .drag_position
                        .unwrap_or_else(|| bool_position(self.checked.get()));

                    let target = if position >= 0.5 { 1.0 } else { 0.0 };

                    drag.tracking = false;
                    drag.drag_candidate = false;
                    drag.dragging = false;
                    drag.drag_position = None;
                    drag.drag_offset_x = 0.0;

                    drag.settle_animation = Some(SwitchPositionAnimation {
                        from: position,
                        to: target,
                        started_at: Instant::now(),
                    });

                    self.checked.set_without_notification(target >= 0.5);

                    position
                };

                self.interaction_state.thumb_shape.end();

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            _ => EventResult::Ignored,
        }
    }
}

impl Switch {
    fn start_toggle(&self) {
        let from = bool_position(self.checked.get());
        self.start_settle(from, from < 0.5);
    }

    fn update_thumb_shape_for_drag(&self, progress: f32, position: Point) {
        let shape = &self.interaction_state.thumb_shape;
        let at_wall = progress <= f32::EPSILON || progress >= 1.0 - f32::EPSILON;
        if at_wall {
            if shape.is_active() {
                shape.end();
            }
        } else if shape.is_active() {
            shape.moved(position);
        } else {
            // Begin at the first unconstrained position. Pointer travel while
            // pinned to either end must never become a velocity impulse.
            shape.begin(position);
        }
    }

    fn start_settle(&self, from: f32, target: bool) {
        self.checked.set_without_notification(target);
        self.interaction_state
            .drag
            .inner
            .borrow_mut()
            .settle_animation = Some(SwitchPositionAnimation {
            from,
            to: bool_position(target),
            started_at: Instant::now(),
        });
    }
}

impl View for Switch {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.button(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if !self.enabled && self.interaction_state.thumb_shape.is_animating() {
            self.interaction_state.thumb_shape.reset();
        }
        self.button(context.theme).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let button_result = self
            .button(context.theme)
            .handle_event(bounds, event, context);

        let switch_result = self.handle_switch_event(bounds, event, context);

        match switch_result {
            EventResult::Consumed => EventResult::Consumed,
            EventResult::Ignored => button_result,
        }
    }
}

struct SwitchMark {
    checked: bool,
    transition: Option<Transition<bool>>,
    enabled: bool,
    interaction: ButtonInteractionState,
    knob_width_animation: Arc<Mutex<KnobWidthAnimationState>>,
    drag: SwitchDragState,
    checked_binding: Binding<bool>,
    thumb_shape: OmochiShape,
}

impl View for SwitchMark {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let metrics = SwitchMetrics::from_theme(context.theme);
        constraints.constrain(Size::new(metrics.track_width, metrics.track_height))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.drag.inner.borrow_mut().mark_bounds = Some(bounds);

        let dragging = self.drag.inner.borrow().dragging;
        let hovered = self.interaction.is_hovered();
        let pressed = self.interaction.is_pressed() || dragging;

        let now = Instant::now();
        let motion = context.theme.motion.toggle;
        let metrics = SwitchMetrics::from_theme(context.theme);

        let (position, position_redraw) = self.visual_position(now, motion);

        let (knob_width, width_redraw) = self.animated_knob_width(now, motion, pressed, metrics);

        if let Some(next_redraw) = position_redraw.into_iter().chain(width_redraw).min() {
            context.request_redraw_in_at(bounds.expanded(16.0), next_redraw);
        }

        let track_color = self.track_color(context.theme, hovered, pressed, position);

        Rectangle::new()
            .color(RectangleColor::Custom(track_color))
            .radius(CornerRadius::Full)
            .paint(bounds, context);

        let knob_left = bounds.origin.x + metrics.knob_inset;

        let knob_right = bounds.origin.x + bounds.size.width - metrics.knob_inset - knob_width;

        let knob_x = interpolate(knob_left, knob_right, position);
        let knob_y = bounds.origin.y + (bounds.size.height - metrics.knob_height) / 2.0;

        let knob_bounds = Rect::new(knob_x, knob_y, knob_width, metrics.knob_height);

        let knob_color = if self.enabled {
            context.theme.switch.knob
        } else {
            context.theme.switch.disabled_knob
        };

        let knob_shadow = if self.enabled {
            context.theme.switch.knob_shadow
        } else {
            ShadowStyle::None
        };

        if self.thumb_shape.is_animating() {
            self.thumb_shape.paint_styled(
                knob_bounds,
                knob_bounds.size.height / 2.0,
                knob_color,
                context.theme.switch.knob_border,
                context.theme.switch.stroke_width,
                knob_shadow,
                context,
            );
        } else {
            Rectangle::new()
                .color(RectangleColor::Custom(knob_color))
                .radius(CornerRadius::Full)
                .border(super::BorderStyle::custom(
                    context.theme.switch.knob_border,
                    context.theme.switch.stroke_width,
                ))
                .shadow(knob_shadow)
                .paint(knob_bounds, context);
        }
    }
}

impl SwitchMark {
    fn track_color(&self, theme: &Theme, hovered: bool, pressed: bool, position: f32) -> Color {
        let off_color = if pressed {
            theme.switch.off_pressed
        } else if hovered {
            theme.switch.off_hovered
        } else {
            theme.switch.off
        };

        let on_color = if pressed {
            theme.switch.on_pressed
        } else if hovered {
            theme.switch.on_hovered
        } else {
            theme.switch.on
        };

        let color = interpolate(off_color, on_color, position);

        if self.enabled {
            color
        } else {
            with_opacity(color, theme.switch.disabled_opacity)
        }
    }

    fn visual_position(&self, now: Instant, motion: Motion) -> (f32, Option<Instant>) {
        let mut completed_target = None;

        let animation_result = {
            let mut drag = self.drag.inner.borrow_mut();

            if let Some(position) = drag.drag_position {
                return (position, None);
            }

            if let Some(animation) = drag.settle_animation {
                let sample = Animation::new(animation.started_at, motion.duration)
                    .easing(motion.easing)
                    .sample(now);

                let position = interpolate(animation.from, animation.to, sample.progress);

                let next_redraw = sample.next_redraw_at;

                if next_redraw.is_none() {
                    drag.settle_animation = None;
                    completed_target = Some(animation.to >= 0.5);
                }

                Some((position, next_redraw))
            } else {
                None
            }
        };

        if let Some(target) = completed_target {
            self.checked_binding.set_without_notification(target);
            self.checked_binding.commit();
        }

        if let Some(result) = animation_result {
            return result;
        }

        self.animation_position(now, motion)
    }

    fn animation_position(&self, now: Instant, motion: Motion) -> (f32, Option<Instant>) {
        let target = bool_position(self.checked);

        let Some(transition) = self.transition else {
            return (target, None);
        };

        if transition.to != self.checked || transition.from == transition.to {
            return (target, None);
        }

        let sample = Animation::new(transition.started_at, motion.duration)
            .easing(motion.easing)
            .sample(now);

        let position = interpolate(
            bool_position(transition.from),
            bool_position(transition.to),
            sample.progress,
        );

        (position, sample.next_redraw_at)
    }

    fn animated_knob_width(
        &self,
        now: Instant,
        motion: Motion,
        pressed: bool,
        metrics: SwitchMetrics,
    ) -> (f32, Option<Instant>) {
        let mut state = self
            .knob_width_animation
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        let current_width = match state.animation {
            Some(animation) => {
                let sample = Animation::new(animation.started_at, motion.duration)
                    .easing(motion.easing)
                    .sample(now);

                interpolate(animation.from, animation.to, sample.progress)
            }
            None => knob_width_for_pressed(state.pressed, metrics),
        };

        if state.pressed != pressed {
            state.pressed = pressed;
            state.animation = Some(KnobWidthAnimation {
                from: current_width,
                to: knob_width_for_pressed(pressed, metrics),
                started_at: now,
            });
        }

        let Some(animation) = state.animation else {
            return (knob_width_for_pressed(pressed, metrics), None);
        };

        let sample = Animation::new(animation.started_at, motion.duration)
            .easing(motion.easing)
            .sample(now);

        let width = interpolate(animation.from, animation.to, sample.progress);
        let next_redraw = sample.next_redraw_at;

        if next_redraw.is_none() {
            state.animation = None;
        }

        (width, next_redraw)
    }
}

fn with_opacity(color: Color, opacity: f32) -> Color {
    let opacity = if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };

    color.with_alpha((color.alpha as f32 * opacity).round() as u8)
}

fn bool_position(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

fn knob_width_for_pressed(pressed: bool, metrics: SwitchMetrics) -> f32 {
    if pressed {
        metrics.pressed_knob_width
    } else {
        metrics.knob_width
    }
}

fn knob_center_x(bounds: Rect, knob_width: f32, progress: f32, metrics: SwitchMetrics) -> f32 {
    let left = bounds.origin.x + metrics.knob_inset + knob_width / 2.0;

    let right = bounds.origin.x + bounds.size.width - metrics.knob_inset - knob_width / 2.0;

    interpolate(left, right, progress.clamp(0.0, 1.0))
}

fn knob_bounds_at(bounds: Rect, knob_width: f32, progress: f32, metrics: SwitchMetrics) -> Rect {
    let center_x = knob_center_x(bounds, knob_width, progress, metrics);

    Rect::new(
        center_x - knob_width / 2.0,
        bounds.origin.y + (bounds.size.height - metrics.knob_height) / 2.0,
        knob_width,
        metrics.knob_height,
    )
}

fn drag_progress_from_pointer(
    bounds: Rect,
    pointer_x: f32,
    drag_offset_x: f32,
    metrics: SwitchMetrics,
) -> f32 {
    let knob_width = metrics.pressed_knob_width;

    let left = bounds.origin.x + metrics.knob_inset + knob_width / 2.0;

    let right = bounds.origin.x + bounds.size.width - metrics.knob_inset - knob_width / 2.0;

    let width = right - left;

    if width <= 0.0 {
        return 0.0;
    }

    ((pointer_x - drag_offset_x - left) / width).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{Switch, SwitchInteractionState, SwitchMetrics};
    use crate::geometry::Point;
    use crate::state::State;
    use crate::theme::Theme;

    #[test]
    fn default_thumb_is_slightly_wider_than_tall() {
        let metrics = SwitchMetrics::from_theme(&Theme::LIGHT);
        assert!(metrics.knob_width > metrics.knob_height);
    }

    #[test]
    fn thumb_uses_switch_specific_geometry_tokens() {
        let mut theme = Theme::LIGHT;
        theme.layout.switch_knob_height = 16.0;
        theme.layout.switch_knob_width = 20.0;
        theme.layout.switch_pressed_knob_width = 24.0;

        let metrics = SwitchMetrics::from_theme(&theme);
        assert_eq!(metrics.knob_height, 16.0);
        assert_eq!(metrics.knob_width, 20.0);
        assert_eq!(metrics.pressed_knob_width, 24.0);
    }

    #[test]
    fn end_stop_releases_velocity_without_snapping_the_thumb() {
        let checked = State::new(false);
        let interaction = SwitchInteractionState::new();
        let switch = Switch::with_interaction(checked.binding(), interaction.clone());
        interaction.thumb_shape.begin(Point::new(10.0, 0.0));
        interaction.thumb_shape.moved(Point::new(20.0, 0.0));

        switch.update_thumb_shape_for_drag(1.0, Point::new(80.0, 0.0));

        assert!(!interaction.thumb_shape.is_active());
        assert!(interaction.thumb_shape.is_animating());
    }
}

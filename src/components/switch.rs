use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};
use crate::accessibility::AccessibilityRole;
use crate::animation::{Animation, Transition, interpolate};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
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
    knob_size: f32,
    pressed_knob_width: f32,
    knob_inset: f32,
    drag_threshold: f32,
    hit_padding: f32,
}

impl SwitchMetrics {
    fn from_theme(theme: &Theme) -> Self {
        let layout = theme.layout;
        Self {
            track_width: layout.switch_track_width,
            track_height: layout.switch_track_height,
            knob_size: layout.switch_knob_size,
            pressed_knob_width: layout.switch_pressed_knob_width,
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

pub struct Switch {
    checked: Binding<bool>,
    label: Option<String>,
    enabled: bool,
    interaction: ButtonInteractionState,
    knob_width_animation: Arc<Mutex<KnobWidthAnimationState>>,
    drag: SwitchDragState,
}

impl Switch {
    pub fn new(checked: Binding<bool>) -> Self {
        Self {
            checked,
            label: None,
            enabled: true,
            interaction: ButtonInteractionState::new(),
            knob_width_animation: Arc::new(Mutex::new(KnobWidthAnimationState::default())),
            drag: SwitchDragState::default(),
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
        &self.interaction
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
                interaction: self.interaction.clone(),
                knob_width_animation: self.knob_width_animation.clone(),
                drag: self.drag.clone(),
                checked_binding: self.checked.clone(),
            }
            .frame(metrics.track_width, metrics.track_height)
            .flex_shrink(0.0),
        );

        Button::with_interaction(self.interaction.clone())
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
            } if self.interaction.is_focused() => {
                self.checked.set(!self.checked.get());
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

                let mut drag = self.drag.inner.borrow_mut();

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
                        knob_bounds_at(*mark_bounds, metrics.knob_size, checked_position, metrics)
                            .expanded(metrics.hit_padding)
                            .contains(*position)
                    })
                    .map(|mark_bounds| {
                        position.x
                            - knob_center_x(
                                mark_bounds,
                                metrics.knob_size,
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

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerMoved { position } => {
                let tracking = self.drag.inner.borrow().tracking;

                if !tracking {
                    return EventResult::Ignored;
                }

                {
                    let mut drag = self.drag.inner.borrow_mut();

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
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => {
                let release = {
                    let mut drag = self.drag.inner.borrow_mut();

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

                        if was_dragging {
                            let target = if final_position >= 0.5 { 1.0 } else { 0.0 };

                            drag.settle_animation = Some(SwitchPositionAnimation {
                                from: final_position,
                                to: target,
                                started_at: Instant::now(),
                            });
                        } else {
                            drag.settle_animation = None;
                        }

                        Some((was_dragging, final_position >= 0.5))
                    }
                };

                let Some((was_dragging, _drag_checked)) = release else {
                    return EventResult::Ignored;
                };

                if !was_dragging && bounds.contains(*position) {
                    self.checked.set(!self.checked.get());
                }

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            ViewEvent::PointerLeft => {
                if self.drag.inner.borrow().tracking {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }

            ViewEvent::FocusChanged { focused: false } => {
                let _final_position = {
                    let mut drag = self.drag.inner.borrow_mut();

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

                    position
                };

                context.request_redraw_in(bounds.expanded(16.0));

                EventResult::Consumed
            }

            _ => EventResult::Ignored,
        }
    }
}

impl View for Switch {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.button(context.theme).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
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
        let knob_y = bounds.origin.y + (bounds.size.height - metrics.knob_size) / 2.0;

        let knob_bounds = Rect::new(knob_x, knob_y, knob_width, metrics.knob_size);

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

        Rectangle::new()
            .color(RectangleColor::Custom(knob_color))
            .radius(CornerRadius::Full)
            .shadow(knob_shadow)
            .paint(knob_bounds, context);
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
        metrics.knob_size
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
        bounds.origin.y + (bounds.size.height - metrics.knob_size) / 2.0,
        knob_width,
        metrics.knob_size,
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

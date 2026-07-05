use super::{
    Button, ButtonInteractionState, ButtonStyle, HStack, Padding, Rectangle, RectangleColor, Text,
    ZStackAlignment,
};
use crate::animation::{Animation, Transition, interpolate};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{StackAlignment, StackGap, ViewExt};
use crate::platform::PointerButton;
use crate::state::Binding;
use crate::theme::{Color, CornerRadius, Motion, Shadow, ShadowSet, ShadowStyle, Theme};
use crate::view::{Constraints, MeasureContext, PaintContext, View};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

const TRACK_WIDTH: f32 = 44.0;
const TRACK_HEIGHT: f32 = 26.0;
const KNOB_SIZE: f32 = 18.0;
const PRESSED_KNOB_WIDTH: f32 = 24.0;
const KNOB_INSET: f32 = 2.0;
const DRAG_THRESHOLD: f32 = 3.0;
const DRAG_HIT_PADDING: f32 = 6.0;

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

struct KnobWidthAnimationState {
    pressed: bool,
    animation: Option<KnobWidthAnimation>,
}

impl Default for KnobWidthAnimationState {
    fn default() -> Self {
        Self {
            pressed: false,
            animation: None,
        }
    }
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
        let mut content = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small);

        if let Some(label) = self.label.as_ref() {
            content = content.child(
                Text::new(label.clone())
                    .font_size(12.0)
                    .line_height(20.0)
                    .weight(500)
                    .color(if self.enabled {
                        theme.colors.text_primary
                    } else {
                        theme.colors.text_disabled
                    })
                    .height(20.0)
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
            .frame(TRACK_WIDTH, TRACK_HEIGHT)
            .flex_shrink(0.0),
        );

        Button::with_interaction(self.interaction.clone())
            .style(ButtonStyle::Custom {
                background: Color::TRANSPARENT,
                hovered_background: Color::TRANSPARENT,
                border: Color::TRANSPARENT,
                hovered_border: Color::TRANSPARENT,
                foreground: theme.colors.text_primary,
            })
            .shadow(ShadowStyle::None)
            .alignment(ZStackAlignment::Leading)
            .enabled(self.enabled)
            .content(Padding::symmetric(6.0, 4.0).content(content))
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

        match event {
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
                    .map(|mark_bounds| mark_bounds.expanded(DRAG_HIT_PADDING).contains(*position))
                    .unwrap_or(false);

                let drag_offset_x = mark_bounds
                    .filter(|mark_bounds| {
                        knob_bounds_at(*mark_bounds, KNOB_SIZE, checked_position)
                            .expanded(DRAG_HIT_PADDING)
                            .contains(*position)
                    })
                    .map(|mark_bounds| {
                        position.x - knob_center_x(mark_bounds, KNOB_SIZE, checked_position)
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

                        if drag.drag_candidate && moved >= DRAG_THRESHOLD {
                            drag.dragging = true;
                        }
                    }

                    if drag.dragging {
                        if let Some(mark_bounds) = drag.mark_bounds {
                            drag.drag_position = Some(drag_progress_from_pointer(
                                mark_bounds,
                                position.x,
                                drag.drag_offset_x,
                            ));
                        }
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

                let Some((was_dragging, drag_checked)) = release else {
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
                let final_position = {
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
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(TRACK_WIDTH, TRACK_HEIGHT))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.drag.inner.borrow_mut().mark_bounds = Some(bounds);

        let dragging = self.drag.inner.borrow().dragging;
        let hovered = self.interaction.is_hovered();
        let pressed = self.interaction.is_pressed() || dragging;

        let now = Instant::now();
        let motion = context.theme.motion.toggle;

        let (position, position_redraw) = self.visual_position(now, motion);

        let (knob_width, width_redraw) = self.animated_knob_width(now, motion, pressed);

        if let Some(next_redraw) = position_redraw.into_iter().chain(width_redraw).min() {
            context.request_redraw_at(next_redraw);
        }

        let track_color = self.track_color(context.theme, hovered, pressed, position);

        Rectangle::new()
            .color(RectangleColor::Custom(track_color))
            .radius(CornerRadius::Full)
            .paint(bounds, context);

        let knob_left = bounds.origin.x + KNOB_INSET;

        let knob_right = bounds.origin.x + bounds.size.width - KNOB_INSET - knob_width;

        let knob_x = interpolate(knob_left, knob_right, position);
        let knob_y = bounds.origin.y + (bounds.size.height - KNOB_SIZE) / 2.0;

        let knob_bounds = Rect::new(knob_x, knob_y, knob_width, KNOB_SIZE);

        let knob_color = if self.enabled {
            Color::WHITE
        } else {
            Color::rgba(255, 255, 255, 170)
        };

        let knob_shadow = if self.enabled {
            ShadowStyle::Custom(ShadowSet::double(
                Shadow::new(Color::rgba(0, 0, 0, 28), 0.0, 1.0, 2.0, 0.0),
                Shadow::new(Color::rgba(0, 0, 0, 14), 0.0, 2.0, 4.0, 0.0),
            ))
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
            Color::from_rgb_hex(0xc7c7cc)
        } else if hovered {
            Color::from_rgb_hex(0xd1d1d6)
        } else {
            Color::from_rgb_hex(0xe5e5ea)
        };

        let on_color = if pressed {
            theme.colors.accent_pressed
        } else if hovered {
            theme.colors.accent_hovered
        } else {
            theme.colors.accent
        };

        let color = interpolate(off_color, on_color, position);

        if self.enabled {
            color
        } else {
            with_opacity(color, 0.45)
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
            None => knob_width_for_pressed(state.pressed),
        };

        if state.pressed != pressed {
            state.pressed = pressed;
            state.animation = Some(KnobWidthAnimation {
                from: current_width,
                to: knob_width_for_pressed(pressed),
                started_at: now,
            });
        }

        let Some(animation) = state.animation else {
            return (knob_width_for_pressed(pressed), None);
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

fn knob_width_for_pressed(pressed: bool) -> f32 {
    if pressed {
        PRESSED_KNOB_WIDTH
    } else {
        KNOB_SIZE
    }
}

fn knob_center_x(bounds: Rect, knob_width: f32, progress: f32) -> f32 {
    let left = bounds.origin.x + KNOB_INSET + knob_width / 2.0;

    let right = bounds.origin.x + bounds.size.width - KNOB_INSET - knob_width / 2.0;

    interpolate(left, right, progress.clamp(0.0, 1.0))
}

fn knob_bounds_at(bounds: Rect, knob_width: f32, progress: f32) -> Rect {
    let center_x = knob_center_x(bounds, knob_width, progress);

    Rect::new(
        center_x - knob_width / 2.0,
        bounds.origin.y + (bounds.size.height - KNOB_SIZE) / 2.0,
        knob_width,
        KNOB_SIZE,
    )
}

fn drag_progress_from_pointer(bounds: Rect, pointer_x: f32, drag_offset_x: f32) -> f32 {
    let knob_width = PRESSED_KNOB_WIDTH;

    let left = bounds.origin.x + KNOB_INSET + knob_width / 2.0;

    let right = bounds.origin.x + bounds.size.width - KNOB_INSET - knob_width / 2.0;

    let width = right - left;

    if width <= 0.0 {
        return 0.0;
    }

    ((pointer_x - drag_offset_x - left) / width).clamp(0.0, 1.0)
}

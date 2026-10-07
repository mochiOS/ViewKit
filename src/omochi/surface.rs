use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use crate::geometry::Point;

use super::burgers::{normalized_creep_gain, recovery_envelope};
use super::deform::{clamp_vector, deform_points};
use super::material::{BurgersParameters, OmochiMaterial};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PullMode {
    Displacement,
    Velocity { direction: f32, axis_scale: Point },
}

#[derive(Clone, Debug, PartialEq)]
pub struct OmochiSample {
    pub points: Vec<Point>,
    pub animating: bool,
}

#[derive(Clone)]
pub struct OmochiSurface {
    inner: Rc<RefCell<OmochiSurfaceInner>>,
}

struct OmochiSurfaceInner {
    material: OmochiMaterial,
    burgers: BurgersParameters,
    pull_mode: PullMode,

    active: bool,
    anchor: Point,
    pointer: Point,

    press_started_at: Option<Instant>,
    released_at: Option<Instant>,
    last_pointer_at: Option<Instant>,
    last_sample_at: Option<Instant>,

    loaded_for: f32,

    release_pull: Point,
    release_press_depth: f32,

    current_pull: Point,
    target_pull: Point,
    current_press_depth: f32,

    pointer_velocity: Point,
}

impl OmochiSurface {
    pub fn new(material: OmochiMaterial, burgers: BurgersParameters, pull_mode: PullMode) -> Self {
        Self {
            inner: Rc::new(RefCell::new(OmochiSurfaceInner {
                material,
                burgers,
                pull_mode,
                active: false,
                anchor: Point::new(0.0, 0.0),
                pointer: Point::new(0.0, 0.0),
                press_started_at: None,
                released_at: None,
                last_pointer_at: None,
                last_sample_at: None,
                loaded_for: 0.0,
                release_pull: Point::new(0.0, 0.0),
                release_press_depth: 0.0,
                current_pull: Point::new(0.0, 0.0),
                target_pull: Point::new(0.0, 0.0),
                current_press_depth: 0.0,
                pointer_velocity: Point::new(0.0, 0.0),
            })),
        }
    }

    pub fn begin_press(&self, point: Point, now: Instant) {
        let mut inner = self.inner.borrow_mut();

        inner.active = true;
        inner.anchor = point;
        inner.pointer = point;
        inner.press_started_at = Some(now);
        inner.released_at = None;
        inner.last_pointer_at = Some(now);
        inner.target_pull = Point::new(0.0, 0.0);
        inner.pointer_velocity = Point::new(0.0, 0.0);
        inner.current_press_depth = inner.material.press_depth * 0.78;
    }

    pub fn move_press(&self, point: Point, now: Instant) {
        let mut inner = self.inner.borrow_mut();

        if !inner.active {
            return;
        }

        let delta_time = inner
            .last_pointer_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0)
            .max(0.001);

        let instant_velocity = Point::new(
            (point.x - inner.pointer.x) / delta_time,
            (point.y - inner.pointer.y) / delta_time,
        );

        let velocity_blend = 1.0 - (-delta_time / 0.035).exp();

        inner.pointer_velocity.x +=
            (instant_velocity.x - inner.pointer_velocity.x) * velocity_blend;
        inner.pointer_velocity.y +=
            (instant_velocity.y - inner.pointer_velocity.y) * velocity_blend;

        inner.pointer = point;
        inner.last_pointer_at = Some(now);
    }

    pub fn end_press(&self, now: Instant) {
        let mut inner = self.inner.borrow_mut();

        if !inner.active {
            return;
        }

        update_active(&mut inner, now, 1.0 / 120.0);

        inner.loaded_for = inner
            .press_started_at
            .map(|started| now.saturating_duration_since(started).as_secs_f32())
            .unwrap_or(0.0)
            .max(0.001);

        inner.release_pull = inner.current_pull;
        inner.release_press_depth = inner.current_press_depth;
        inner.active = false;
        inner.released_at = Some(now);
        inner.target_pull = Point::new(0.0, 0.0);
        inner.pointer_velocity = Point::new(0.0, 0.0);
    }

    pub fn sample(&self, base_points: &[Point], now: Instant) -> OmochiSample {
        let mut inner = self.inner.borrow_mut();

        let delta_time = inner
            .last_sample_at
            .map(|last| now.saturating_duration_since(last).as_secs_f32())
            .unwrap_or(0.0)
            .clamp(0.0, 0.033);

        inner.last_sample_at = Some(now);

        if inner.active {
            update_active(&mut inner, now, delta_time);
        } else if inner.released_at.is_some() {
            update_recovery(&mut inner, now);
        }

        let anchor = match inner.pull_mode {
            PullMode::Displacement => inner.anchor,
            PullMode::Velocity { .. } => Point::new(0.0, 0.0),
        };

        let points = deform_points(
            base_points,
            inner.current_pull,
            inner.current_press_depth,
            anchor,
            inner.material,
        );

        OmochiSample {
            points,
            animating: inner.active || inner.released_at.is_some(),
        }
    }

    pub fn is_active(&self) -> bool {
        self.inner.borrow().active
    }

    pub fn is_animating(&self) -> bool {
        let inner = self.inner.borrow();

        inner.active || inner.released_at.is_some()
    }

    pub fn reset(&self) {
        let mut inner = self.inner.borrow_mut();

        inner.active = false;
        inner.anchor = Point::new(0.0, 0.0);
        inner.pointer = Point::new(0.0, 0.0);
        inner.press_started_at = None;
        inner.released_at = None;
        inner.last_pointer_at = None;
        inner.last_sample_at = None;
        inner.loaded_for = 0.0;
        inner.release_pull = Point::new(0.0, 0.0);
        inner.release_press_depth = 0.0;
        inner.current_pull = Point::new(0.0, 0.0);
        inner.target_pull = Point::new(0.0, 0.0);
        inner.current_press_depth = 0.0;
        inner.pointer_velocity = Point::new(0.0, 0.0);
    }
}

fn update_active(inner: &mut OmochiSurfaceInner, now: Instant, delta_time: f32) {
    let held_for = inner
        .press_started_at
        .map(|started| now.saturating_duration_since(started).as_secs_f32())
        .unwrap_or(0.0);

    let gain = normalized_creep_gain(held_for, inner.material, inner.burgers);

    inner.target_pull = match inner.pull_mode {
        PullMode::Displacement => {
            let raw_x = inner.pointer.x - inner.anchor.x;
            let raw_y = inner.pointer.y - inner.anchor.y;

            let drag = clamp_vector(
                raw_x * inner.material.drag_gain,
                raw_y * inner.material.drag_gain,
                inner.material.max_pull,
            );

            clamp_vector(drag.x * gain, drag.y * gain, inner.material.max_pull)
        }

        PullMode::Velocity {
            direction,
            axis_scale,
        } => {
            let idle_time = inner
                .last_pointer_at
                .map(|last| now.saturating_duration_since(last).as_secs_f32())
                .unwrap_or(0.0);

            let idle_decay = (-idle_time / inner.material.velocity_idle_decay.max(0.001)).exp();

            clamp_vector(
                inner.pointer_velocity.x
                    * axis_scale.x
                    * idle_decay
                    * inner.material.velocity_pull_scale
                    * direction
                    * gain,
                inner.pointer_velocity.y
                    * axis_scale.y
                    * idle_decay
                    * inner.material.velocity_pull_scale
                    * direction
                    * gain,
                inner.material.max_pull,
            )
        }
    };

    let speed = inner.pointer_velocity.x.hypot(inner.pointer_velocity.y);

    let speed_range = (inner.material.max_speed - inner.material.fast_speed).max(1.0);
    let speed_mix = ((speed - inner.material.fast_speed) / speed_range).clamp(0.0, 1.0);

    let follow_time = inner.material.viscous_follow_time
        + (inner.material.fast_viscous_follow_time - inner.material.viscous_follow_time)
            * speed_mix;

    let pull_alpha = 1.0 - (-delta_time.max(0.0) / follow_time.max(0.001)).exp();
    inner.current_pull.x += (inner.target_pull.x - inner.current_pull.x) * pull_alpha;
    inner.current_pull.y += (inner.target_pull.y - inner.current_pull.y) * pull_alpha;
    let press_target = inner.material.press_depth * (0.78 + 0.22 * gain);
    let press_alpha = 1.0 - (-delta_time.max(1.0 / 240.0) / 0.018).exp();
    inner.current_press_depth += (press_target - inner.current_press_depth) * press_alpha;
}

fn update_recovery(inner: &mut OmochiSurfaceInner, now: Instant) {
    let Some(released_at) = inner.released_at else {
        return;
    };

    let elapsed = now.saturating_duration_since(released_at).as_secs_f32();

    let envelope = recovery_envelope(elapsed, inner.loaded_for, inner.material, inner.burgers);

    inner.current_pull = Point::new(
        inner.release_pull.x * envelope,
        inner.release_pull.y * envelope,
    );

    let press_recovery_time = (inner.material.spring_recovery_time * 1.6).max(0.016);
    let press_envelope = (-elapsed / press_recovery_time).exp();
    inner.current_press_depth = inner.release_press_depth * press_envelope;
    let pull_remaining = inner.current_pull.x.hypot(inner.current_pull.y);
    if pull_remaining < 0.001 && inner.current_press_depth.abs() < 0.001 {
        inner.current_pull = Point::new(0.0, 0.0);
        inner.current_press_depth = 0.0;
        inner.released_at = None;
    }
}

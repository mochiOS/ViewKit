//! `../omochi/playground/omochi.js`のBurgers粘弾性モデルを使う操作面。
//! 座標は操作面内の論理px。描画・操作範囲・文字配置とは独立して保持します。
use crate::geometry::{Point, Rect};
use std::time::{Duration, Instant};
pub(crate) mod moving;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BurgersModel {
    pub e1: f32,
    pub e2: f32,
    pub eta1: f32,
    pub eta2: f32,
}
impl BurgersModel {
    pub const DEFAULT: Self = Self {
        e1: 3.10,
        e2: 0.92,
        eta1: 14.0,
        eta2: 0.46,
    };
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub drag_gain: f32,
    pub max_pull: f32,
    pub press_depth: f32,
    pub press_radius: f32,
    pub drag_radius: f32,
    pub reference_creep_time: f32,
    pub flow_recovery_time: f32,
    pub spring_recovery_time: f32,
    pub tan_delta: f32,
    pub drag_follow: f32,
    pub center_lock: f32,
    pub neck_ratio: f32,
    pub neck_width_ratio: f32,
    pub neck_backshift_ratio: f32,
    pub tip_stretch: f32,
    pub tip_long_radius: f32,
    pub tip_cross_radius: f32,
    pub viscous_follow_time: f32,
    pub fast_viscous_follow_time: f32,
    pub fast_speed: f32,
    pub max_speed: f32,
    pub press_release_start: f32,
    pub press_release_end: f32,
    pub velocity_pull_scale: f32,
    pub velocity_idle_decay: f32,
}
impl Material {
    pub const DEFAULT: Self = Self {
        drag_gain: 0.34,
        max_pull: 46.0,
        press_depth: 9.5,
        press_radius: 92.0,
        drag_radius: 118.0,
        reference_creep_time: 0.68,
        flow_recovery_time: 0.94,
        spring_recovery_time: 0.032,
        tan_delta: 0.46,
        drag_follow: 0.34,
        center_lock: 1.0,
        neck_ratio: 0.052,
        neck_width_ratio: 0.30,
        neck_backshift_ratio: 0.24,
        tip_stretch: 0.30,
        tip_long_radius: 34.0,
        tip_cross_radius: 58.0,
        viscous_follow_time: 0.050,
        fast_viscous_follow_time: 0.115,
        fast_speed: 1100.0,
        max_speed: 3000.0,
        press_release_start: 1.5,
        press_release_end: 12.0,
        velocity_pull_scale: 0.014,
        velocity_idle_decay: 0.055,
    };
    /// button/index.htmlの素材設定。その他の値は共通素材から継承します。
    pub const BUTTON: Self = Self {
        max_pull: 16.0,
        press_depth: 3.4,
        press_radius: 33.0,
        drag_radius: 42.0,
        neck_backshift_ratio: 0.22,
        tip_long_radius: 12.0,
        tip_cross_radius: 21.0,
        press_release_start: 0.6,
        press_release_end: 4.5,
        ..Self::DEFAULT
    };
    /// slider/index.html: velocity-driven thumb, opposite to pointer motion.
    pub const SLIDER: Self = Self {
        press_depth: 0.0,
        max_pull: 18.0,
        velocity_pull_scale: 0.018,
        velocity_idle_decay: 0.080,
        viscous_follow_time: 0.040,
        fast_viscous_follow_time: 0.085,
        drag_follow: 0.46,
        neck_ratio: 0.080,
        neck_width_ratio: 0.34,
        neck_backshift_ratio: 0.20,
        tip_stretch: 0.62,
        tip_long_radius: 12.0,
        tip_cross_radius: 18.0,
        ..Self::DEFAULT
    };
    /// segmented-control/index.html: selected surface pulled from its leading edge.
    pub const SELECTION: Self = Self {
        press_depth: 0.0,
        max_pull: 28.0,
        drag_gain: 0.82,
        viscous_follow_time: 0.044,
        fast_viscous_follow_time: 0.082,
        drag_radius: 72.0,
        drag_follow: 0.38,
        neck_ratio: 0.052,
        neck_width_ratio: 0.28,
        neck_backshift_ratio: 0.26,
        tip_stretch: 0.58,
        tip_long_radius: 20.0,
        tip_cross_radius: 27.0,
        press_release_start: 0.0,
        press_release_end: 1.0,
        ..Self::DEFAULT
    };
    pub fn for_height(self, height: f32) -> Self {
        let scale = (height / 30.0).clamp(0.1, 8.0);
        Self {
            max_pull: self.max_pull * scale,
            press_depth: self.press_depth * scale,
            press_radius: self.press_radius * scale,
            drag_radius: self.drag_radius * scale,
            tip_long_radius: self.tip_long_radius * scale,
            tip_cross_radius: self.tip_cross_radius * scale,
            press_release_start: self.press_release_start * scale,
            press_release_end: self.press_release_end * scale,
            velocity_pull_scale: self.velocity_pull_scale * scale,
            ..self
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OmochiMotion {
    pub enabled: bool,
    pub material: Material,
    pub burgers: BurgersModel,
}
impl OmochiMotion {
    pub const DEFAULT: Self = Self {
        enabled: true,
        material: Material::BUTTON,
        burgers: BurgersModel::DEFAULT,
    };
}

pub fn creep_compliance(time: f32, model: BurgersModel) -> f32 {
    let time = time.max(0.0);
    let tau = model.eta2 / model.e2;
    1.0 / model.e1 + 1.0 / model.e2 * (1.0 - (-time / tau).exp()) + time / model.eta1
}
pub fn normalized_creep_gain(time: f32, motion: OmochiMotion) -> f32 {
    (creep_compliance(time, motion.burgers)
        / creep_compliance(motion.material.reference_creep_time, motion.burgers))
    .clamp(0.0, 1.12)
}
pub fn recovery_envelope(time: f32, held: f32, motion: OmochiMotion) -> f32 {
    let m = motion.material;
    let b = motion.burgers;
    let held = held.max(0.001);
    let time = time.max(0.0);
    let tau = b.eta2 / b.e2;
    let elastic = 1.0 / b.e1;
    let delayed = 1.0 / b.e2 * (1.0 - (-held / tau).exp());
    let viscous = held / b.eta1;
    let total = elastic + delayed + viscous;
    let loss = std::f32::consts::PI * m.tan_delta / (2.0 + std::f32::consts::PI * m.tan_delta);
    elastic / total * (-time / (m.spring_recovery_time * (1.0 + loss * 0.35))).exp()
        + delayed / total * (-time / (tau * (1.0 + loss * 0.55))).exp()
        + viscous / total * (-time / (m.flow_recovery_time * (1.0 + loss * 0.80))).exp()
}
fn clamp_vector(p: Point, max: f32) -> Point {
    let length = p.x.hypot(p.y);
    if length <= max || length < 0.0001 {
        p
    } else {
        Point::new(p.x * max / length, p.y * max / length)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SurfaceSample {
    pub anchor: Point,
    pub pull: Point,
    pub press_depth: f32,
    pub next_redraw_at: Option<Instant>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OmochiSurface {
    velocity_driven: bool,
    velocity_axis: Point,
    active: bool,
    anchor: Point,
    pointer: Point,
    velocity: Point,
    last_pointer: Option<(Point, Instant)>,
    started: Option<Instant>,
    released: Option<Instant>,
    updated: Option<Instant>,
    held: f32,
    pull: Point,
    depth: f32,
    release_pull: Point,
    release_depth: f32,
}
impl OmochiSurface {
    pub fn is_active(&self) -> bool {
        self.active
    }
    pub fn begin(&mut self, point: Point, now: Instant) {
        self.velocity_driven = false;
        self.active = true;
        self.anchor = point;
        self.pointer = point;
        self.velocity = Point::default();
        self.last_pointer = Some((point, now));
        self.started = Some(now);
        self.released = None;
        self.updated = Some(now);
    }
    /// Velocity-to-pull mapping per axis, including direction.
    pub fn begin_velocity(&mut self, point: Point, axis: Point, now: Instant) {
        self.begin(point, now);
        self.velocity_driven = true;
        self.velocity_axis = axis;
    }
    pub fn move_to(&mut self, point: Point, now: Instant) {
        if !self.active {
            return;
        }
        if let Some((last, time)) = self.last_pointer {
            let dt = now.saturating_duration_since(time).as_secs_f32().max(0.001);
            let blend = 1.0 - (-dt / 0.035).exp();
            self.velocity.x += ((point.x - last.x) / dt - self.velocity.x) * blend;
            self.velocity.y += ((point.y - last.y) / dt - self.velocity.y) * blend;
        }
        self.pointer = point;
        self.last_pointer = Some((point, now));
    }
    pub fn end(&mut self, motion: OmochiMotion, now: Instant) {
        if !self.active {
            return;
        }
        self.update_active(motion, now, 1.0 / 120.0);
        self.held = now
            .saturating_duration_since(self.started.unwrap_or(now))
            .as_secs_f32()
            .max(0.001);
        self.release_pull = self.pull;
        self.release_depth = self.depth;
        self.active = false;
        self.released = Some(now);
        self.updated = Some(now);
    }
    fn update_active(&mut self, motion: OmochiMotion, now: Instant, dt: f32) -> bool {
        let gain = normalized_creep_gain(
            now.saturating_duration_since(self.started.unwrap_or(now))
                .as_secs_f32(),
            motion,
        );
        let m = motion.material;
        let raw = if self.velocity_driven {
            let idle = now
                .saturating_duration_since(self.last_pointer.map_or(now, |(_, time)| time))
                .as_secs_f32();
            let decay = (-idle / m.velocity_idle_decay.max(0.001)).exp();
            Point::new(
                self.velocity.x * self.velocity_axis.x * decay * m.velocity_pull_scale,
                self.velocity.y * self.velocity_axis.y * decay * m.velocity_pull_scale,
            )
        } else {
            clamp_vector(
                Point::new(
                    (self.pointer.x - self.anchor.x) * m.drag_gain,
                    (self.pointer.y - self.anchor.y) * m.drag_gain,
                ),
                m.max_pull,
            )
        };
        let target = clamp_vector(Point::new(raw.x * gain, raw.y * gain), m.max_pull);
        let speed = self.velocity.x.hypot(self.velocity.y);
        let mix = ((speed - m.fast_speed) / (m.max_speed - m.fast_speed).max(1.0)).clamp(0.0, 1.0);
        let follow =
            m.viscous_follow_time + (m.fast_viscous_follow_time - m.viscous_follow_time) * mix;
        let alpha = 1.0 - (-dt.max(0.0) / follow.max(0.001)).exp();
        self.pull.x += (target.x - self.pull.x) * alpha;
        self.pull.y += (target.y - self.pull.y) * alpha;
        self.depth = m.press_depth * gain;
        (!self.velocity_driven && gain < 1.12)
            || (self.velocity_driven && self.pull.x.hypot(self.pull.y) > 0.005)
            || (self.pull.x - target.x).hypot(self.pull.y - target.y) > 0.005
    }
    pub fn sample(&mut self, motion: OmochiMotion, now: Instant) -> SurfaceSample {
        if !motion.enabled {
            *self = Self::default();
            return SurfaceSample::default();
        }
        let dt = self.updated.map_or(0.0, |time| {
            now.saturating_duration_since(time).as_secs_f32().min(0.033)
        });
        self.updated = Some(now);
        let moving = if self.active {
            self.update_active(motion, now, dt)
        } else if let Some(released) = self.released {
            let envelope = recovery_envelope(
                now.saturating_duration_since(released).as_secs_f32(),
                self.held,
                motion,
            );
            self.pull = Point::new(
                self.release_pull.x * envelope,
                self.release_pull.y * envelope,
            );
            self.depth = self.release_depth * envelope;
            if envelope < 0.0008 || (self.pull.x.hypot(self.pull.y) < 0.005 && self.depth < 0.005) {
                self.pull = Point::default();
                self.depth = 0.0;
                self.released = None;
                false
            } else {
                true
            }
        } else {
            false
        };
        SurfaceSample {
            anchor: self.anchor,
            pull: self.pull,
            press_depth: self.depth,
            next_redraw_at: moving.then_some(now + Duration::from_millis(16)),
        }
    }
}

fn gaussian(distance: f32, radius: f32) -> f32 {
    (-distance / (2.0 * radius * radius)).exp()
}
fn smoothstep(value: f32) -> f32 {
    let x = value.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// `deformPoints`の局所押下・中心固定・先端伸張・首の収縮を共通実装します。
pub fn deform_points(
    base: &[Point],
    center: Point,
    sample: SurfaceSample,
    m: Material,
) -> Vec<Point> {
    if base.is_empty() {
        return Vec::new();
    }
    let pull = sample.pull;
    let length = pull.x.hypot(pull.y);
    let pulling = length > 0.001;
    let direction = if pulling {
        Point::new(pull.x / length, pull.y / length)
    } else {
        Point::default()
    };
    let perpendicular = Point::new(-direction.y, direction.x);
    let minx = base.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
    let maxx = base.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
    let miny = base.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
    let maxy = base.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
    let width = (maxx - minx).max(1.0);
    let height = (maxy - miny).max(1.0);
    let axis = (direction.x.abs() * width * 0.5 + direction.y.abs() * height * 0.5).max(1.0);
    let cross =
        (perpendicular.x.abs() * width * 0.5 + perpendicular.y.abs() * height * 0.5).max(1.0);
    let transition = (axis * 0.42).max(18.0);
    let along =
        |p: Point| (p.x - sample.anchor.x) * direction.x + (p.y - sample.anchor.y) * direction.y;
    let surface = base
        .iter()
        .map(|&p| along(p))
        .fold(f32::NEG_INFINITY, f32::max);
    let offsets: Vec<Point> = base
        .iter()
        .map(|p| {
            let relative = Point::new(p.x - sample.anchor.x, p.y - sample.anchor.y);
            let follow = if pulling {
                m.drag_follow
                    * (0.5 + 0.5 * (along(*p) / transition).tanh())
                    * (0.72
                        + 0.28
                            * gaussian(
                                relative.x * relative.x + relative.y * relative.y,
                                m.drag_radius,
                            ))
            } else {
                0.0
            };
            Point::new(pull.x * follow, pull.y * follow)
        })
        .collect();
    let mean = Point::new(
        offsets.iter().map(|p| p.x).sum::<f32>() / base.len() as f32,
        offsets.iter().map(|p| p.y).sum::<f32>() / base.len() as f32,
    );
    let press = sample.press_depth
        * (1.0
            - smoothstep(
                (length - m.press_release_start)
                    / (m.press_release_end - m.press_release_start).max(0.001),
            ));
    let _ = center; // Center locking uses the mean drag offset, as in the reference.
    base.iter()
        .zip(offsets)
        .map(|(&point, offset)| {
            let to = Point::new(sample.anchor.x - point.x, sample.anchor.y - point.y);
            let distance = to.x.hypot(to.y);
            let mut p = point;
            if distance > 0.001 && press > 0.001 {
                let influence = press * gaussian(distance * distance, m.press_radius) / distance;
                p.x += to.x * influence;
                p.y += to.y * influence;
            }
            if pulling {
                p.x += offset.x - mean.x * m.center_lock;
                p.y += offset.y - mean.y * m.center_lock;
                let along = along(point);
                let across = (point.x - sample.anchor.x) * perpendicular.x
                    + (point.y - sample.anchor.y) * perpendicular.y;
                let behind = surface - along;
                let tip = gaussian(behind * behind, m.tip_long_radius)
                    * gaussian(across * across, m.tip_cross_radius);
                p.x += pull.x * m.tip_stretch * tip;
                p.y += pull.y * m.tip_stretch * tip;
                let neck_width = (axis * m.neck_width_ratio).max(14.0);
                let neck_center = surface - axis * m.neck_backshift_ratio;
                let contraction = length
                    * m.neck_ratio
                    * gaussian((along - neck_center).powi(2), neck_width)
                    * (across.abs() / cross).clamp(0.0, 1.0).powf(0.76);
                p.x -= perpendicular.x * across.signum() * contraction;
                p.y -= perpendicular.y * across.signum() * contraction;
            }
            p
        })
        .collect()
}

/// Same line/arc sampling and Catmull–Rom tension (0.68) as button/index.html.
pub fn button_contour(
    bounds: Rect,
    radius: f32,
    sample: SurfaceSample,
    material: Material,
) -> Vec<Point> {
    let w = bounds.size.width;
    let h = bounds.size.height;
    let r = radius.max(0.0).min(w.min(h) * 0.5);
    let mut base = vec![Point::new(r, 0.0)];
    let mut line = |a: Point, b: Point, count: usize| {
        for i in 1..=count {
            let t = i as f32 / count as f32;
            base.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    };
    line(Point::new(r, 0.0), Point::new(w - r, 0.0), 18);
    // Build arcs/edges in one loop to avoid duplicating corner endpoints.
    for corner in 0..4 {
        let (cx, cy, start) = match corner {
            0 => (w - r, r, -90.0_f32),
            1 => (w - r, h - r, 0.0),
            2 => (r, h - r, 90.0),
            _ => (r, r, 180.0),
        };
        for i in 1..=12 {
            let angle = (start + i as f32 * 90.0 / 12.0).to_radians();
            base.push(Point::new(cx + r * angle.cos(), cy + r * angle.sin()));
        }
        let (a, b, count) = match corner {
            0 => (Point::new(w, r), Point::new(w, h - r), 10),
            1 => (Point::new(w - r, h), Point::new(r, h), 18),
            2 => (Point::new(0.0, h - r), Point::new(0.0, r), 10),
            _ => (Point::new(r, 0.0), Point::new(r, 0.0), 0),
        };
        for i in 1..=count {
            let t = i as f32 / count as f32;
            base.push(Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    let deformed = deform_points(&base, Point::new(w * 0.5, h * 0.5), sample, material);
    // The reference repeats the first point at the end. Remove that duplicate
    // before smoothing, otherwise its zero-length cubic creates a tiny loop.
    let deformed = crate::contour::clean(&deformed);
    let n = deformed.len();
    let mut points = Vec::with_capacity(n * 3);
    for i in 0..n {
        let previous = deformed[(i + n - 1) % n];
        let current = deformed[i];
        let next = deformed[(i + 1) % n];
        let after = deformed[(i + 2) % n];
        let c1 = Point::new(
            current.x + (next.x - previous.x) * 0.68 / 6.0,
            current.y + (next.y - previous.y) * 0.68 / 6.0,
        );
        let c2 = Point::new(
            next.x - (after.x - current.x) * 0.68 / 6.0,
            next.y - (after.y - current.y) * 0.68 / 6.0,
        );
        let dx = next.x - current.x;
        let dy = next.y - current.y;
        let chord = dx.hypot(dy).max(0.0001);
        let deviation = |p: Point| ((p.x - current.x) * dy - (p.y - current.y) * dx).abs() / chord;
        let steps = ((deviation(c1).max(deviation(c2)) / 0.05).sqrt().ceil() as usize).clamp(1, 8);
        for j in 0..steps {
            let t = j as f32 / steps as f32;
            let u = 1.0 - t;
            points.push(Point::new(
                bounds.origin.x
                    + u.powi(3) * current.x
                    + 3.0 * u * u * t * c1.x
                    + 3.0 * u * t * t * c2.x
                    + t.powi(3) * next.x,
                bounds.origin.y
                    + u.powi(3) * current.y
                    + 3.0 * u * u * t * c1.y
                    + 3.0 * u * t * t * c2.y
                    + t.powi(3) * next.y,
            ));
        }
    }
    crate::contour::clean(&points)
}

#[cfg(test)]
mod tests;

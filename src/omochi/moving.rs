//! Native adaptations of the switch and selected-indicator reference drivers.
//! Position, deformation and bound values are independent; idle drivers stop repainting.
use super::*;
use crate::geometry::Size;

fn follow(current: f32, target: f32, dt: f32, tau: f32) -> f32 {
    current + (target - current) * (1.0 - (-dt / tau.max(0.001)).exp())
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SwitchThumb {
    pub center: f32,
    pub stretch: f32,
    pub press: f32,
    direction: f32,
    updated: Option<Instant>,
}
impl SwitchThumb {
    pub fn initialize(&mut self, center: f32, now: Instant) {
        if self.updated.is_none() {
            self.center = center;
            self.updated = Some(now);
        }
    }

    pub fn sample(
        &mut self,
        target: f32,
        pressed: bool,
        dragging: bool,
        scale: f32,
        motion: OmochiMotion,
        now: Instant,
    ) -> Option<Instant> {
        let Some(updated) = self.updated else {
            self.center = target;
            self.updated = Some(now);
            return (pressed && motion.enabled).then_some(now + Duration::from_millis(16));
        };
        let dt = now
            .saturating_duration_since(updated)
            .as_secs_f32()
            .min(0.033);
        self.updated = Some(now);
        if !motion.enabled {
            self.center = target;
            self.press = 0.0;
            self.stretch = 0.0;
            return None;
        }
        let loss = std::f32::consts::PI * motion.material.tan_delta
            / (2.0 + std::f32::consts::PI * motion.material.tan_delta);
        let tau = if dragging {
            0.052 + loss * 0.014
        } else {
            0.070 + loss * 0.020
        };
        let previous = self.center;
        self.center = follow(self.center, target, dt, tau);
        let velocity = if dt > 0.0001 {
            (self.center - previous) / dt
        } else {
            0.0
        };
        if velocity.abs() > 1.0 {
            self.direction = velocity.signum();
        }
        let stretch = (velocity.abs() * 0.0058).min(3.2 * scale);
        self.stretch = follow(
            self.stretch,
            stretch,
            dt,
            if stretch > self.stretch {
                0.060 + loss * 0.025
            } else {
                0.135 + loss * 0.090
            },
        );
        let press = if pressed { 1.0 } else { 0.0 };
        self.press = follow(self.press, press, dt, if pressed { 0.045 } else { 0.115 });
        if (target - self.center).abs() < 0.01
            && self.stretch < 0.005
            && (press - self.press).abs() < 0.001
        {
            self.center = target;
            self.stretch = 0.0;
            self.press = press;
            None
        } else {
            Some(now + Duration::from_millis(16))
        }
    }
    pub fn contour(&self, y: f32, width: f32, height: f32) -> Vec<Point> {
        let scale = height / 18.0;
        let widen = self.press * 0.55 * scale;
        let (back, front) = (self.stretch * 0.28, self.stretch * 0.72);
        let (left, right) = if self.direction >= 0.0 {
            (back, front)
        } else {
            (front, back)
        };
        let radius = (height * 0.5 - self.press * 0.35 * scale).max(0.1);
        let bounds = Rect::new(
            self.center - width * 0.5 - left - widen,
            y - radius,
            width + left + right + widen * 2.0,
            radius * 2.0,
        );
        let neck = (self.stretch * 0.18).min(3.1 * scale);
        let sigma = (bounds.size.width * 0.22).max(8.0 * scale);
        let center = bounds.origin.x + bounds.size.width * 0.5;
        button_contour(bounds, radius, SurfaceSample::default(), Material::BUTTON)
            .into_iter()
            .map(|mut p| {
                let influence = (-((p.x - center) / sigma).powi(2) * 0.5).exp();
                p.y = y + (p.y - y) * (1.0 - neck / radius * influence);
                p
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SelectionSurface {
    pub center: f32,
    updated: Option<Instant>,
    direction: f32,
    surface: OmochiSurface,
}
impl SelectionSurface {
    pub fn initialize(&mut self, center: f32, now: Instant) {
        if self.updated.is_none() {
            self.center = center;
            self.updated = Some(now);
        }
    }
    pub fn sample(
        &mut self,
        target: f32,
        size: Size,
        motion: OmochiMotion,
        now: Instant,
    ) -> SurfaceSample {
        let dt = self.updated.map_or(0.0, |last| {
            now.saturating_duration_since(last).as_secs_f32().min(0.033)
        });
        self.initialize(target, now);
        self.updated = Some(now);
        if !motion.enabled {
            self.center = target;
            self.surface = OmochiSurface::default();
            return SurfaceSample::default();
        }
        let error = target - self.center;
        let previous = self.center;
        self.center = follow(self.center, target, dt, 0.072);
        let velocity = if dt > 0.0001 {
            (self.center - previous) / dt
        } else {
            0.0
        };
        let moving = error.abs() >= 0.06 || velocity.abs() >= 0.8;
        if moving {
            let direction = if error.abs() > 0.35 {
                error.signum()
            } else {
                self.direction
            };
            let edge = Point::new(
                if direction >= 0.0 { size.width } else { 0.0 },
                size.height * 0.5,
            );
            if !self.surface.is_active() || direction != self.direction {
                self.surface.end(motion, now);
                self.surface.begin(edge, now);
            }
            self.direction = direction;
            let scale = size.height / 38.0;
            let distance =
                (error.abs() * 0.34 + (velocity.abs() * 0.010).min(8.0 * scale)).min(34.0 * scale);
            self.surface
                .move_to(Point::new(edge.x + direction * distance, edge.y), now);
        } else {
            self.center = target;
            self.surface.end(motion, now);
        }
        let mut sample = self.surface.sample(motion, now);
        if moving {
            sample.next_redraw_at = Some(now + Duration::from_millis(16));
        }
        sample
    }
}

/// The slider reference's continuous press-in/press-out scaling driver.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PressResponse {
    value: f32,
    updated: Option<Instant>,
}
impl PressResponse {
    pub fn sample(&mut self, pressed: bool, enabled: bool, now: Instant) -> (f32, Option<Instant>) {
        let dt = self.updated.map_or(0.0, |last| {
            now.saturating_duration_since(last).as_secs_f32().min(0.033)
        });
        self.updated = Some(now);
        if !enabled {
            self.value = 0.0;
            return (0.0, None);
        }
        let target = if pressed { 1.0 } else { 0.0 };
        self.value = follow(self.value, target, dt, if pressed { 0.040 } else { 0.115 });
        if (target - self.value).abs() < 0.001 {
            self.value = target;
            (self.value, None)
        } else {
            (self.value, Some(now + Duration::from_millis(16)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn switch_stretches_in_travel_direction_reverses_and_settles() {
        let now = Instant::now();
        let mut thumb = SwitchThumb::default();
        let motion = OmochiMotion::DEFAULT;
        thumb.sample(13.5, false, false, 1.0, motion, now);
        for i in 1..=8 {
            thumb.sample(
                26.5,
                true,
                false,
                1.0,
                motion,
                now + Duration::from_millis(i * 16),
            );
        }
        assert!(thumb.center > 13.5 && thumb.center < 26.5);
        assert!(thumb.stretch > 0.0 && thumb.press > 0.0);
        let points = thumb.contour(11.0, 23.0, 18.0);
        let left = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
        let right = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
        assert!(right - thumb.center > thumb.center - left);
        assert!(!crate::contour::tessellate(&points).is_empty());
        let previous = thumb.center;
        let mut next = None;
        for i in 9..=500 {
            next = thumb.sample(
                13.5,
                false,
                false,
                1.0,
                motion,
                now + Duration::from_millis(i * 16),
            );
            assert!(thumb.center >= 13.5 && thumb.center <= previous);
        }
        assert_eq!(thumb.center, 13.5);
        assert_eq!(thumb.press, 0.0);
        assert_eq!(thumb.stretch, 0.0);
        assert!(next.is_none());
        assert!(
            thumb
                .sample(
                    26.5,
                    true,
                    true,
                    1.0,
                    OmochiMotion {
                        enabled: false,
                        ..motion
                    },
                    now + Duration::from_secs(9)
                )
                .is_none()
        );
        assert_eq!(thumb.center, 26.5);
        assert_eq!(thumb.press, 0.0);
    }
    #[test]
    fn selected_surface_can_reverse_midflight_without_overshooting_and_stops_redrawing() {
        let now = Instant::now();
        let motion = OmochiMotion {
            material: Material::SELECTION,
            ..OmochiMotion::DEFAULT
        };
        let mut surface = SelectionSurface::default();
        surface.initialize(46.0, now);
        let size = Size::new(92.0, 38.0);
        for i in 1..=5 {
            let sample = surface.sample(246.0, size, motion, now + Duration::from_millis(i * 16));
            assert!(surface.center >= 46.0 && surface.center <= 246.0);
            assert!(sample.next_redraw_at.is_some());
        }
        let reversal = surface.center;
        let mut final_sample = SurfaceSample::default();
        for i in 6..=600 {
            final_sample = surface.sample(46.0, size, motion, now + Duration::from_millis(i * 16));
            assert!(surface.center >= 46.0 && surface.center <= reversal);
            let points = button_contour(
                Rect::new(0.0, 0.0, 92.0, 38.0),
                9.0,
                final_sample,
                motion.material,
            );
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        }
        assert_eq!(surface.center, 46.0);
        assert!(final_sample.next_redraw_at.is_none());
        let reduced = surface.sample(
            246.0,
            size,
            OmochiMotion {
                enabled: false,
                ..motion
            },
            now + Duration::from_secs(10),
        );
        assert_eq!(surface.center, 246.0);
        assert_eq!(reduced, SurfaceSample::default());
    }
    #[test]
    fn slider_press_releases_continuously_and_reduced_motion_clears_it() {
        let now = Instant::now();
        let mut press = PressResponse::default();
        press.sample(false, true, now);
        let (pressed, _) = press.sample(true, true, now + Duration::from_millis(16));
        assert!((pressed - (1.0 - (-0.016_f32 / 0.040).exp())).abs() < 0.00001);
        let (released, next) = press.sample(false, true, now + Duration::from_millis(32));
        assert!(released > 0.0 && released < pressed && next.is_some());
        assert_eq!(
            press.sample(true, false, now + Duration::from_millis(48)),
            (0.0, None)
        );
    }
}

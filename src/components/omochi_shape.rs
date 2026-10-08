use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::animation::{Animation, interpolate};
use crate::draw_command::DrawCommand;
use crate::geometry::{Point, Rect};
use crate::omochi::{
    DEFAULT_BURGERS, DEFAULT_MATERIAL, OmochiMaterial, OmochiSurface, PullMode, rounded_rect_points,
};
use crate::theme::Color;
use crate::theme::Motion;
use crate::view::PaintContext;

#[derive(Clone, Copy)]
pub(super) enum OmochiPreset {
    CompactControl,
    Thumb,
    SelectionIndicator,
}

#[derive(Clone, Copy)]
struct SelectionTransition {
    from: f32,
    to: f32,
    started_at: Instant,
    launched: bool,
    completed: bool,
}

#[derive(Clone, Default)]
pub(super) struct SelectionMotion {
    transition: Rc<RefCell<Option<SelectionTransition>>>,
}

impl SelectionMotion {
    pub(super) fn start(&self, from: usize, to: usize) {
        if from == to {
            return;
        }
        *self.transition.borrow_mut() = Some(SelectionTransition {
            from: from as f32,
            to: to as f32,
            started_at: Instant::now(),
            launched: false,
            completed: false,
        });
    }

    pub(super) fn sample(&self, fallback: f32, motion: Motion) -> (f32, Option<Instant>) {
        let mut transition = self.transition.borrow_mut();
        let Some(active) = transition.as_mut() else {
            return (fallback, None);
        };
        if active.completed {
            return (active.to, None);
        }
        let sample = Animation::new(active.started_at, motion.duration)
            .easing(motion.easing)
            .sample(Instant::now());
        active.completed = sample.next_redraw_at.is_none();
        (
            interpolate(active.from, active.to, sample.progress),
            sample.next_redraw_at,
        )
    }

    pub(super) fn take_launch(&self) -> Option<(f32, f32)> {
        let mut transition = self.transition.borrow_mut();
        let active = transition.as_mut()?;
        if active.launched {
            return None;
        }
        active.launched = true;
        Some((active.from, active.to))
    }

    pub(super) fn ready_to_commit(&self) -> bool {
        self.transition
            .borrow()
            .is_some_and(|transition| transition.completed)
    }

    pub(super) fn finish(&self) {
        *self.transition.borrow_mut() = None;
    }
}

impl OmochiPreset {
    fn material(self) -> OmochiMaterial {
        let mut material = DEFAULT_MATERIAL;
        match self {
            Self::CompactControl => {
                material.max_pull = 16.0;
                material.press_depth = 3.4;
                material.press_radius = 33.0;
                material.drag_radius = 42.0;
                material.tip_long_radius = 12.0;
                material.tip_cross_radius = 21.0;
            }
            Self::Thumb => {
                material.max_pull = 14.0;
                material.press_depth = 3.2;
                material.press_radius = 22.0;
                material.drag_radius = 32.0;
                material.tip_long_radius = 9.0;
                material.tip_cross_radius = 16.0;
                material.velocity_pull_scale = 0.016;
            }
            Self::SelectionIndicator => {
                material.max_pull = 18.0;
                material.press_depth = 3.0;
                material.press_radius = 54.0;
                material.drag_radius = 72.0;
                material.tip_long_radius = 18.0;
                material.tip_cross_radius = 28.0;
                material.velocity_pull_scale = 0.014;
            }
        }
        material.press_release_start = 0.6;
        material.press_release_end = 4.5;
        material
    }
}

#[derive(Clone)]
pub(super) struct OmochiShape {
    surface: OmochiSurface,
}

impl OmochiShape {
    pub(super) fn velocity(preset: OmochiPreset) -> Self {
        Self {
            surface: OmochiSurface::new(
                preset.material(),
                DEFAULT_BURGERS,
                PullMode::Velocity {
                    direction: 1.0,
                    axis_scale: Point::new(1.0, 0.55),
                },
            ),
        }
    }

    pub(super) fn displacement(preset: OmochiPreset) -> Self {
        Self {
            surface: OmochiSurface::new(preset.material(), DEFAULT_BURGERS, PullMode::Displacement),
        }
    }

    pub(super) fn begin(&self, point: Point) {
        self.surface.begin_press(point, Instant::now());
    }

    pub(super) fn moved(&self, point: Point) {
        self.surface.move_press(point, Instant::now());
    }

    pub(super) fn end(&self) {
        self.surface.end_press(Instant::now());
    }

    pub(super) fn reset(&self) {
        self.surface.reset();
    }

    pub(super) fn is_active(&self) -> bool {
        self.surface.is_active()
    }

    pub(super) fn is_animating(&self) -> bool {
        self.surface.is_animating()
    }

    pub(super) fn paint(
        &self,
        bounds: Rect,
        radius: f32,
        color: Color,
        context: &mut PaintContext<'_>,
    ) {
        let now = Instant::now();
        let base = rounded_rect_points(bounds.size.width, bounds.size.height, radius, 10);
        let sample = self.surface.sample(&base, now);
        let center = Point::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );
        context.display_list.push(DrawCommand::FillPolygon {
            points: sample
                .points
                .into_iter()
                .map(|point| Point::new(center.x + point.x, center.y + point.y))
                .collect(),
            color,
        });
        if sample.animating {
            context.request_redraw_in_at(bounds.expanded(20.0), now + Duration::from_millis(8));
        }
    }
}

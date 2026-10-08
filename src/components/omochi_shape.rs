use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::animation::{Animation, interpolate};
use crate::draw_command::DrawCommand;
use crate::geometry::{Point, Rect};
use crate::omochi::{
    DEFAULT_BURGERS, DEFAULT_MATERIAL, OmochiMaterial, OmochiSurface, PullMode, rounded_rect_points,
};
use crate::theme::{Color, Motion, Shadow, ShadowStyle};
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
                material.neck_backshift_ratio = 0.22;
                material.tip_long_radius = 12.0;
                material.tip_cross_radius = 21.0;
            }
            Self::Thumb => {
                material.press_depth = 0.0;
                material.max_pull = 18.0;
                material.drag_follow = 0.46;
                material.neck_ratio = 0.080;
                material.neck_width_ratio = 0.34;
                material.neck_backshift_ratio = 0.20;
                material.tip_stretch = 0.62;
                material.tip_long_radius = 12.0;
                material.tip_cross_radius = 18.0;
                material.velocity_pull_scale = 0.018;
                material.velocity_idle_decay = 0.080;
                material.viscous_follow_time = 0.040;
                material.fast_viscous_follow_time = 0.085;
            }
            Self::SelectionIndicator => {
                material.press_depth = 0.0;
                material.max_pull = 28.0;
                material.drag_gain = 0.82;
                material.viscous_follow_time = 0.044;
                material.fast_viscous_follow_time = 0.082;
                material.drag_radius = 72.0;
                material.drag_follow = 0.38;
                material.neck_ratio = 0.052;
                material.neck_width_ratio = 0.28;
                material.neck_backshift_ratio = 0.26;
                material.tip_stretch = 0.58;
                material.tip_long_radius = 20.0;
                material.tip_cross_radius = 27.0;
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
        let (direction, axis_scale) = match preset {
            OmochiPreset::Thumb => (-1.0, Point::new(1.0, 0.12)),
            _ => (1.0, Point::new(1.0, 0.55)),
        };
        Self {
            surface: OmochiSurface::new(
                preset.material(),
                DEFAULT_BURGERS,
                PullMode::Velocity {
                    direction,
                    axis_scale,
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

    pub(super) fn paint_points(
        &self,
        base_points: &[Point],
        center: Point,
        color: Color,
        redraw_bounds: Rect,
        context: &mut PaintContext<'_>,
    ) {
        let now = Instant::now();
        let sample = self.surface.sample(base_points, now);
        context.display_list.push(DrawCommand::FillPolygon {
            points: sample
                .points
                .into_iter()
                .map(|point| Point::new(center.x + point.x, center.y + point.y))
                .collect(),
            color,
        });
        if sample.animating {
            context
                .request_redraw_in_at(redraw_bounds.expanded(20.0), now + Duration::from_millis(8));
        }
    }

    pub(super) fn paint_styled(
        &self,
        bounds: Rect,
        radius: f32,
        fill: Color,
        border: Color,
        border_width: f32,
        shadow: ShadowStyle,
        context: &mut PaintContext<'_>,
    ) {
        let now = Instant::now();
        let base = rounded_rect_points(bounds.size.width, bounds.size.height, radius, 10);
        let sample = self.surface.sample(&base, now);
        let center = Point::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        );

        if let Some(shadows) = shadow.resolve(&context.theme.shadows) {
            for shadow in shadows.layers.iter().rev().flatten() {
                paint_polygon_shadow(&sample.points, center, bounds, *shadow, context);
            }
        }

        if border.alpha > 0 && border_width > 0.0 {
            context.display_list.push(DrawCommand::FillPolygon {
                points: expanded_points(&sample.points, center, bounds, border_width, 0.0, 0.0),
                color: border,
            });
        }
        context.display_list.push(DrawCommand::FillPolygon {
            points: expanded_points(&sample.points, center, bounds, 0.0, 0.0, 0.0),
            color: fill,
        });

        if sample.animating {
            context.request_redraw_in_at(bounds.expanded(20.0), now + Duration::from_millis(8));
        }
    }
}

fn expanded_points(
    points: &[Point],
    center: Point,
    bounds: Rect,
    expansion: f32,
    offset_x: f32,
    offset_y: f32,
) -> Vec<Point> {
    let scale_x = 1.0 + expansion * 2.0 / bounds.size.width.max(1.0);
    let scale_y = 1.0 + expansion * 2.0 / bounds.size.height.max(1.0);
    points
        .iter()
        .map(|point| {
            Point::new(
                center.x + point.x * scale_x + offset_x,
                center.y + point.y * scale_y + offset_y,
            )
        })
        .collect()
}

fn paint_polygon_shadow(
    points: &[Point],
    center: Point,
    bounds: Rect,
    shadow: Shadow,
    context: &mut PaintContext<'_>,
) {
    if shadow.color.alpha == 0 {
        return;
    }
    let blur = shadow.blur_radius.max(0.0);
    let layers = if blur > 0.0 { 5 } else { 1 };
    let layer_alpha = (u16::from(shadow.color.alpha) / layers as u16).max(1) as u8;
    for layer in (0..layers).rev() {
        let progress = if layers == 1 {
            0.0
        } else {
            layer as f32 / (layers - 1) as f32
        };
        context.display_list.push(DrawCommand::FillPolygon {
            points: expanded_points(
                points,
                center,
                bounds,
                shadow.spread.max(0.0) + blur * progress,
                shadow.offset_x,
                shadow.offset_y,
            ),
            color: shadow.color.with_alpha(layer_alpha),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn pressed_shape_emits_a_deformed_polygon() {
        let shape = OmochiShape::displacement(OmochiPreset::CompactControl);
        shape.begin(Point::new(8.0, 0.0));

        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        let bounds = Rect::new(20.0, 30.0, 28.0, 28.0);
        shape.paint(bounds, 6.0, Color::WHITE, &mut context);

        let points = display_list.commands().iter().find_map(|command| {
            if let DrawCommand::FillPolygon { points, .. } = command {
                Some(points)
            } else {
                None
            }
        });
        let points = points.expect("omochi paint must use a polygon contour");
        let base = rounded_rect_points(28.0, 28.0, 6.0, 10);
        let center = Point::new(34.0, 44.0);

        assert_eq!(points.len(), base.len());
        assert!(points.iter().zip(base).any(|(painted, base)| {
            (painted.x - (center.x + base.x)).abs() > 0.05
                || (painted.y - (center.y + base.y)).abs() > 0.05
        }));
    }

    #[test]
    fn styled_shape_keeps_shadow_border_and_white_fill_while_animating() {
        let shape = OmochiShape::velocity(OmochiPreset::Thumb);
        shape.begin(Point::new(0.0, 0.0));
        shape.moved(Point::new(12.0, 0.0));

        let theme = Theme::LIGHT;
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &theme,
            &Typography::DEFAULT,
            &mut text_measurer,
        );
        shape.paint_styled(
            Rect::new(20.0, 30.0, 26.0, 20.0),
            10.0,
            Color::WHITE,
            theme.slider.knob_border,
            theme.slider.stroke_width,
            theme.slider.knob_shadow,
            &mut context,
        );

        let polygon_colors = display_list.commands().iter().filter_map(|command| {
            if let DrawCommand::FillPolygon { color, .. } = command {
                Some(*color)
            } else {
                None
            }
        });
        let polygon_colors = polygon_colors.collect::<Vec<_>>();
        assert!(
            polygon_colors.len() > 2,
            "shadow, border, and fill must be layered"
        );
        assert!(polygon_colors.contains(&theme.slider.knob_border));
        assert_eq!(polygon_colors.last(), Some(&Color::WHITE));
    }
}

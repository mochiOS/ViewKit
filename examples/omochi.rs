use std::time::{Duration, Instant};

use viewkit::draw_command::DrawCommand;
use viewkit::event::{EventContext, EventResult, ViewEvent};
use viewkit::geometry::{Point, Rect, Size};
use viewkit::omochi::{
    DEFAULT_BURGERS, DEFAULT_MATERIAL, OmochiMaterial, OmochiSurface, PullMode, rounded_rect_points,
};
use viewkit::platform::PointerButton;
use viewkit::prelude::{
    App, Color, StackAlignment, StackDistribution, VStack, ViewContext, ViewExt, ViewKitError,
    WindowOptions,
};
use viewkit::view::{Constraints, MeasureContext, PaintContext, View};

const SHAPE_WIDTH: f32 = 96.0;
const SHAPE_HEIGHT: f32 = 30.0;
const SHAPE_RADIUS: f32 = 9.0;

fn button_material() -> OmochiMaterial {
    let mut material = DEFAULT_MATERIAL;

    material.max_pull = 16.0;
    material.press_depth = 3.4;
    material.press_radius = 33.0;
    material.drag_radius = 42.0;
    material.neck_backshift_ratio = 0.22;
    material.tip_long_radius = 12.0;
    material.tip_cross_radius = 21.0;
    material.press_release_start = 0.6;
    material.press_release_end = 4.5;

    material
}

struct OmochiPreview {
    surface: OmochiSurface,
    base_points: Vec<Point>,
}

impl OmochiPreview {
    fn new() -> Self {
        Self {
            surface: OmochiSurface::new(button_material(), DEFAULT_BURGERS, PullMode::Displacement),
            base_points: rounded_rect_points(SHAPE_WIDTH, SHAPE_HEIGHT, SHAPE_RADIUS, 12),
        }
    }

    fn shape_bounds(bounds: Rect) -> Rect {
        Rect::new(
            bounds.origin.x + (bounds.size.width - SHAPE_WIDTH) / 2.0,
            bounds.origin.y + (bounds.size.height - SHAPE_HEIGHT) / 2.0,
            SHAPE_WIDTH,
            SHAPE_HEIGHT,
        )
    }

    fn center(bounds: Rect) -> Point {
        Point::new(
            bounds.origin.x + bounds.size.width / 2.0,
            bounds.origin.y + bounds.size.height / 2.0,
        )
    }

    fn local_point(bounds: Rect, point: Point) -> Point {
        let center = Self::center(bounds);

        Point::new(point.x - center.x, point.y - center.y)
    }
}

impl View for OmochiPreview {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(240.0, 140.0))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let now = Instant::now();
        let sample = self.surface.sample(&self.base_points, now);
        let center = Self::center(bounds);

        let points = sample
            .points
            .into_iter()
            .map(|point| Point::new(center.x + point.x, center.y + point.y))
            .collect();

        context.display_list.push(DrawCommand::FillPolygon {
            points,
            color: Color::from_rgb_hex(0x3970DD),
        });

        if sample.animating {
            context.request_redraw_in_at(bounds.expanded(48.0), now + Duration::from_millis(8));
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        match event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => {
                if !Self::shape_bounds(bounds).contains(*position) {
                    return EventResult::Ignored;
                }

                self.surface
                    .begin_press(Self::local_point(bounds, *position), Instant::now());

                context.request_redraw_in(bounds.expanded(48.0));

                EventResult::Consumed
            }

            ViewEvent::PointerMoved { position } if self.surface.is_active() => {
                self.surface
                    .move_press(Self::local_point(bounds, *position), Instant::now());

                context.request_redraw_in(bounds.expanded(48.0));

                EventResult::Consumed
            }

            ViewEvent::PointerReleased {
                button: PointerButton::Primary,
                ..
            } if self.surface.is_active() => {
                self.surface.end_press(Instant::now());

                context.request_redraw_in(bounds.expanded(48.0));

                EventResult::Consumed
            }

            _ => EventResult::Ignored,
        }
    }
}

struct OmochiExample;

impl App for OmochiExample {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("Omochi Surface")
            .size(480.0, 320.0)
            .resizable(true)
    }

    fn body(&self, _context: &ViewContext) -> Self::Body {
        Box::new(
            VStack::new()
                .alignment(StackAlignment::Center)
                .distribution(StackDistribution::Center)
                .child(OmochiPreview::new().frame(240.0, 140.0)),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<OmochiExample>()
}

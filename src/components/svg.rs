//! SVGを表示するコンポーネント

use crate::draw_command::{DrawCommand, SvgCommand};
use crate::geometry::{Rect, Size};
use crate::svg::SvgData;
use crate::theme::{Color, CornerRadius};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgContentMode {
    #[default]
    Fit,
    Fill,
    Stretch,
}

#[derive(Clone, Debug)]
pub struct Svg {
    svg: SvgData,

    content_mode: SvgContentMode,
    radius: CornerRadius,

    opacity: f32,
    tint: Option<Color>,
}

impl Svg {
    pub fn new(svg: SvgData) -> Self {
        Self {
            svg,

            content_mode: SvgContentMode::Fit,

            radius: CornerRadius::None,

            opacity: 1.0,

            tint: None,
        }
    }

    pub fn svg(&self) -> &SvgData {
        &self.svg
    }

    pub fn content_mode(mut self, content_mode: SvgContentMode) -> Self {
        self.content_mode = content_mode;

        self
    }

    pub fn radius(mut self, radius: CornerRadius) -> Self {
        self.radius = radius;

        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = sanitize_opacity(opacity);

        self
    }

    pub fn tint(mut self, tint: Color) -> Self {
        self.tint = Some(tint);

        self
    }

    pub fn optional_tint(mut self, tint: Option<Color>) -> Self {
        self.tint = tint;

        self
    }

    pub fn original_colors(mut self) -> Self {
        self.tint = None;

        self
    }
}

impl View for Svg {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        let intrinsic_size = Size::new(self.svg.width(), self.svg.height());

        constraints.constrain(intrinsic_size)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if !is_valid_bounds(bounds) {
            return;
        }

        let svg_width = self.svg.width();

        let svg_height = self.svg.height();

        if !svg_width.is_finite()
            || !svg_height.is_finite()
            || svg_width <= 0.0
            || svg_height <= 0.0
        {
            return;
        }

        let svg_bounds = resolve_svg_bounds(bounds, svg_width, svg_height, self.content_mode);

        let radius =
            self.radius
                .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);

        /*
         * FillではSVGがboundsより大きくなるため、
         * radiusが0でも矩形クリップが必要です。
         */
        let needs_clip = radius > 0.0 || self.content_mode == SvgContentMode::Fill;

        if needs_clip {
            if radius > 0.0 {
                context.display_list.push(DrawCommand::PushRoundedClip {
                    rect: bounds,
                    radius,
                });
            } else {
                context
                    .display_list
                    .push(DrawCommand::PushClip { rect: bounds });
            }
        }

        context.display_list.push(DrawCommand::DrawSvg {
            command: SvgCommand {
                svg: self.svg.clone(),

                bounds: svg_bounds,

                opacity: self.opacity,

                tint: self.tint,
            },
        });

        if needs_clip {
            context.display_list.push(DrawCommand::PopClip);
        }
    }
}

fn resolve_svg_bounds(
    bounds: Rect,
    svg_width: f32,
    svg_height: f32,
    content_mode: SvgContentMode,
) -> Rect {
    if content_mode == SvgContentMode::Stretch {
        return bounds;
    }

    let scale_x = bounds.size.width / svg_width;

    let scale_y = bounds.size.height / svg_height;

    let scale = match content_mode {
        SvgContentMode::Fit => scale_x.min(scale_y),

        SvgContentMode::Fill => scale_x.max(scale_y),

        SvgContentMode::Stretch => {
            unreachable!()
        }
    };

    let width = svg_width * scale;

    let height = svg_height * scale;

    Rect::new(
        bounds.origin.x + (bounds.size.width - width) / 2.0,
        bounds.origin.y + (bounds.size.height - height) / 2.0,
        width,
        height,
    )
}

fn is_valid_bounds(bounds: Rect) -> bool {
    bounds.origin.x.is_finite()
        && bounds.origin.y.is_finite()
        && bounds.size.width.is_finite()
        && bounds.size.height.is_finite()
        && bounds.size.width > 0.0
        && bounds.size.height > 0.0
}

fn sanitize_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

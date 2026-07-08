//! 画像を表示するImageコンポーネント

use crate::draw_command::{DrawCommand, ImageCommand, ImageSampling};
use crate::geometry::{Rect, Size};
use crate::image::ImageData;
use crate::theme::CornerRadius;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ImageContentMode {
    /*
     * アスペクト比を維持し、
     * 全体が収まる最大サイズにします。
     */
    #[default]
    Fit,

    /*
     * アスペクト比を維持し、
     * 領域全体を覆うサイズにします。
     *
     * はみ出した部分はクリップされます。
     */
    Fill,

    /*
     * アスペクト比を維持せず、
     * 指定領域全体へ引き伸ばします。
     */
    Stretch,
}

#[derive(Clone, Debug)]
pub struct Image {
    image: ImageData,

    content_mode: ImageContentMode,
    radius: CornerRadius,

    opacity: f32,
    sampling: ImageSampling,
}

impl Image {
    pub fn new(image: ImageData) -> Self {
        Self {
            image,
            content_mode: ImageContentMode::Fit,
            radius: CornerRadius::None,
            opacity: 1.0,
            sampling: ImageSampling::Bicubic,
        }
    }

    pub fn image(&self) -> &ImageData {
        &self.image
    }

    pub fn content_mode(mut self, content_mode: ImageContentMode) -> Self {
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

    pub fn sampling(mut self, sampling: ImageSampling) -> Self {
        self.sampling = sampling;

        self
    }
}

impl View for Image {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        let intrinsic_size = Size::new(self.image.width() as f32, self.image.height() as f32);

        constraints.constrain(intrinsic_size)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if !is_valid_bounds(bounds) {
            return;
        }

        let image_width = self.image.width() as f32;

        let image_height = self.image.height() as f32;

        if image_width <= 0.0 || image_height <= 0.0 {
            return;
        }

        let image_bounds =
            resolve_image_bounds(bounds, image_width, image_height, self.content_mode);

        let radius =
            self.radius
                .resolve(&context.theme.radius, bounds.size.width, bounds.size.height);

        /*
         * Fillでは画像がboundsより大きくなるため、
         * radiusが0でも矩形クリップが必要です。
         */
        let needs_clip = radius > 0.0 || self.content_mode == ImageContentMode::Fill;

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

        context.display_list.push(DrawCommand::DrawImage {
            command: ImageCommand {
                image: self.image.clone(),

                bounds: image_bounds,

                opacity: self.opacity,

                sampling: self.sampling,
            },
        });

        if needs_clip {
            context.display_list.push(DrawCommand::PopClip);
        }
    }
}

fn resolve_image_bounds(
    bounds: Rect,
    image_width: f32,
    image_height: f32,
    content_mode: ImageContentMode,
) -> Rect {
    if content_mode == ImageContentMode::Stretch {
        return bounds;
    }

    let scale_x = bounds.size.width / image_width;

    let scale_y = bounds.size.height / image_height;

    let scale = match content_mode {
        ImageContentMode::Fit => scale_x.min(scale_y),

        ImageContentMode::Fill => scale_x.max(scale_y),

        ImageContentMode::Stretch => {
            unreachable!()
        }
    };

    let width = image_width * scale;
    let height = image_height * scale;

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

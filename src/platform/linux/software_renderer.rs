//! softbufferとtiny-skiaを使用したLinux向けソフトウェアレンダラー

use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{
    Context,
    SoftBufferError,
    Surface,
};
use tiny_skia::{
    Color as SkiaColor,
    FillRule,
    Paint,
    Path,
    PathBuilder,
    Pixmap,
    Rect as SkiaRect,
    Stroke,
    Transform,
};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use crate::draw_command::{
    DisplayList,
    DrawCommand,
};
use crate::geometry::Rect;
use crate::renderer::{
    Renderer,
    Viewport,
};
use crate::theme::Color;

#[derive(Debug, thiserror::Error)]
pub enum SoftwareRendererError {
    #[error("softbufferの処理に失敗しました: {0}")]
    SoftBuffer(#[from] SoftBufferError),

    #[error(
        "描画バッファを確保できませんでした: {width}x{height}"
    )]
    PixmapAllocation {
        width: u32,
        height: u32,
    },
}

pub struct SoftwareRenderer {
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    viewport: Viewport,
    pixmap: Option<Pixmap>,
}

impl SoftwareRenderer {
    pub fn new(
        context: &Context<OwnedDisplayHandle>,
        window: Rc<Window>,
        viewport: Viewport,
    ) -> Result<Self, SoftwareRendererError> {
        let surface = Surface::new(context, window)?;

        let mut renderer = Self {
            surface,
            viewport,
            pixmap: None,
        };

        renderer.resize_surface(viewport)?;

        Ok(renderer)
    }

    fn resize_surface(
        &mut self,
        viewport: Viewport,
    ) -> Result<(), SoftwareRendererError> {
        self.viewport = viewport;

        if viewport.physical_width == 0
            || viewport.physical_height == 0
        {
            self.pixmap = None;
            return Ok(());
        }

        let width = NonZeroU32::new(viewport.physical_width)
            .expect("幅は0ではない");

        let height = NonZeroU32::new(viewport.physical_height)
            .expect("高さは0ではない");

        self.surface.resize(width, height)?;

        self.pixmap = Some(
            Pixmap::new(
                viewport.physical_width,
                viewport.physical_height,
            )
                .ok_or(
                    SoftwareRendererError::PixmapAllocation {
                        width: viewport.physical_width,
                        height: viewport.physical_height,
                    },
                )?,
        );

        Ok(())
    }
}

impl Renderer for SoftwareRenderer {
    type Error = SoftwareRendererError;

    fn resize(
        &mut self,
        viewport: Viewport,
    ) -> Result<(), Self::Error> {
        self.resize_surface(viewport)
    }

    fn render(
        &mut self,
        display_list: &DisplayList,
    ) -> Result<(), Self::Error> {
        let Some(pixmap) = self.pixmap.as_mut() else {
            return Ok(());
        };

        // 前フレームの内容を残さない
        pixmap.fill(SkiaColor::from_rgba8(0, 0, 0, 0));

        let scale = valid_scale_factor(
            self.viewport.scale_factor,
        );

        let transform = Transform::from_scale(scale, scale);

        for command in display_list.commands() {
            match command {
                DrawCommand::Clear { color } => {
                    pixmap.fill(to_skia_color(*color));
                }

                DrawCommand::FillRect {
                    rect,
                    color,
                } => {
                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let paint = solid_paint(*color);

                    pixmap.fill_rect(
                        rect,
                        &paint,
                        transform,
                        None,
                    );
                }

                DrawCommand::FillRoundedRect {
                    rect,
                    radius,
                    color,
                } => {
                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = rounded_rect_path(
                        rect,
                        *radius,
                    );

                    let paint = solid_paint(*color);

                    pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        transform,
                        None,
                    );
                }

                DrawCommand::StrokeRect {
                    rect,
                    color,
                    width,
                } => {
                    if !width.is_finite() || *width <= 0.0 {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = PathBuilder::from_rect(rect);
                    let paint = solid_paint(*color);

                    let stroke = Stroke {
                        width: *width,
                        ..Stroke::default()
                    };

                    pixmap.stroke_path(
                        &path,
                        &paint,
                        &stroke,
                        transform,
                        None,
                    );
                }

                DrawCommand::StrokeRoundedRect {
                    rect,
                    radius,
                    color,
                    width,
                } => {
                    if !width.is_finite() || *width <= 0.0 {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = rounded_rect_path(
                        rect,
                        *radius,
                    );

                    let paint = solid_paint(*color);

                    let stroke = Stroke {
                        width: *width,
                        ..Stroke::default()
                    };

                    pixmap.stroke_path(
                        &path,
                        &paint,
                        &stroke,
                        transform,
                        None,
                    );
                }

                DrawCommand::DrawText { .. }
                | DrawCommand::PushClip { .. }
                | DrawCommand::PopClip => {
                    // 後で実装する
                }
            }
        }

        copy_pixmap_to_surface(
            pixmap,
            &mut self.surface,
        )?;

        Ok(())
    }
}

fn copy_pixmap_to_surface(
    pixmap: &Pixmap,
    surface: &mut Surface<
        OwnedDisplayHandle,
        Rc<Window>,
    >,
) -> Result<(), SoftBufferError> {
    let mut buffer = surface.buffer_mut()?;

    for (destination, rgba) in buffer
        .iter_mut()
        .zip(pixmap.data().chunks_exact(4))
    {
        let red = u32::from(rgba[0]);
        let green = u32::from(rgba[1]);
        let blue = u32::from(rgba[2]);

        *destination =
            (red << 16) | (green << 8) | blue;
    }

    buffer.present()?;

    Ok(())
}

fn solid_paint(
    color: Color,
) -> Paint<'static> {
    let mut paint = Paint::default();

    paint.set_color_rgba8(
        color.red,
        color.green,
        color.blue,
        color.alpha,
    );

    paint.anti_alias = true;

    paint
}

fn to_skia_color(
    color: Color,
) -> SkiaColor {
    SkiaColor::from_rgba8(
        color.red,
        color.green,
        color.blue,
        color.alpha,
    )
}

fn to_skia_rect(
    rect: Rect,
) -> Option<SkiaRect> {
    let x = rect.origin.x;
    let y = rect.origin.y;
    let width = rect.size.width;
    let height = rect.size.height;

    if !x.is_finite()
        || !y.is_finite()
        || !width.is_finite()
        || !height.is_finite()
        || width < 0.0
        || height < 0.0
    {
        return None;
    }

    SkiaRect::from_xywh(
        x,
        y,
        width,
        height,
    )
}

fn rounded_rect_path(
    rect: SkiaRect,
    radius: f32,
) -> Path {
    let radius = if radius.is_finite() {
        radius.max(0.0).min(
            rect.width().min(rect.height()) / 2.0,
        )
    } else {
        0.0
    };

    if radius == 0.0 {
        return PathBuilder::from_rect(rect);
    }

    let left = rect.left();
    let top = rect.top();
    let right = rect.right();
    let bottom = rect.bottom();

    let mut builder = PathBuilder::new();

    builder.move_to(left + radius, top);
    builder.line_to(right - radius, top);

    builder.quad_to(
        right,
        top,
        right,
        top + radius,
    );

    builder.line_to(right, bottom - radius);

    builder.quad_to(
        right,
        bottom,
        right - radius,
        bottom,
    );

    builder.line_to(left + radius, bottom);

    builder.quad_to(
        left,
        bottom,
        left,
        bottom - radius,
    );

    builder.line_to(left, top + radius);

    builder.quad_to(
        left,
        top,
        left + radius,
        top,
    );

    builder.close();

    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn valid_scale_factor(
    scale_factor: f64,
) -> f32 {
    if scale_factor.is_finite()
        && scale_factor > 0.0
    {
        scale_factor as f32
    } else {
        1.0
    }
}
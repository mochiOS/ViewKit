//! softbufferとtiny-skiaを使用したLinux向けソフトウェアレンダラー

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, SoftBufferError, Surface};
use tiny_skia::{
    Color as SkiaColor, FillRule, FilterQuality, Mask, Paint, Path, PathBuilder, Pixmap,
    PixmapPaint, PixmapRef, Rect as SkiaRect, Stroke, Transform,
};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use crate::draw_command::{
    DisplayList, DrawCommand, ImageCommand, ImageSampling, SvgCommand, TextCommand,
};
use crate::font::create_font_system;
use crate::geometry::Rect;
use crate::renderer::{Renderer, Viewport};
use crate::theme::Color;
use crate::typography::TextAlignment;
use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};

#[derive(Debug, thiserror::Error)]
pub enum SoftwareRendererError {
    #[error("softbufferの処理に失敗しました: {0}")]
    SoftBuffer(#[from] SoftBufferError),

    #[error("描画バッファを確保できませんでした: {width}x{height}")]
    PixmapAllocation { width: u32, height: u32 },

    #[error("クリップマスクを確保できませんでした: {width}x{height}")]
    ClipMaskAllocation { width: u32, height: u32 },

    #[error("対応するPushClipがない状態でPopClipが呼び出されました")]
    ClipStackUnderflow,

    #[error("閉じられていないクリップが残っています: depth={depth}")]
    UnclosedClipStack { depth: usize },

    #[error("SVG描画バッファを確保できませんでした: {width}x{height}")]
    SvgPixmapAllocation { width: u32, height: u32 },

    #[error("画像描画バッファを確保できませんでした: {width}x{height}")]
    ImagePixmapAllocation { width: u32, height: u32 },
}

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;
const SVG_SMALL_RENDER_LIMIT: f32 = 256.0;
const SVG_SMALL_RENDER_SUPERSAMPLE: f32 = 2.0;
const IMAGE_SMALL_RENDER_LIMIT: f32 = 256.0;
const IMAGE_SMALL_RENDER_SUPERSAMPLE: f32 = 3.0;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextLayoutKey {
    text: String,
    font_family: String,

    font_size_bits: u32,
    line_height_bits: u32,

    width_bits: u32,
    height_bits: u32,
    scale_bits: u32,

    weight: u16,
    alignment: u8,
}

impl TextLayoutKey {
    fn new(command: &TextCommand, scale: f32) -> Self {
        Self {
            text: command.text.clone(),
            font_family: command.font_family.clone(),

            font_size_bits: canonical_f32_bits(command.font_size),
            line_height_bits: canonical_f32_bits(command.line_height),

            width_bits: canonical_f32_bits(command.bounds.size.width),
            height_bits: canonical_f32_bits(command.bounds.size.height),
            scale_bits: canonical_f32_bits(scale),

            weight: command.weight.clamp(1, 1000),
            alignment: alignment_key(command.alignment),
        }
    }
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    }
}

const fn alignment_key(alignment: TextAlignment) -> u8 {
    match alignment {
        TextAlignment::Start => 0,
        TextAlignment::Center => 1,
        TextAlignment::End => 2,
        TextAlignment::Justified => 3,
    }
}

pub struct SoftwareRenderer {
    surface: Surface<OwnedDisplayHandle, Rc<Window>>,
    viewport: Viewport,
    pixmap: Option<Pixmap>,
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_layout_cache: HashMap<TextLayoutKey, Buffer>,
    present_pixels: Vec<u32>,
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
            font_system: create_font_system(),
            swash_cache: SwashCache::new(),
            text_layout_cache: HashMap::new(),
            present_pixels: Vec::new(),
        };

        renderer.resize_surface(viewport)?;

        Ok(renderer)
    }

    fn resize_surface(&mut self, viewport: Viewport) -> Result<(), SoftwareRendererError> {
        if self.viewport != viewport {
            self.text_layout_cache.clear();
        }

        self.viewport = viewport;

        if viewport.physical_width == 0 || viewport.physical_height == 0 {
            self.pixmap = None;

            return Ok(());
        }

        let width = NonZeroU32::new(viewport.physical_width).expect("幅は0ではない");

        let height = NonZeroU32::new(viewport.physical_height).expect("高さは0ではない");

        self.surface.resize(width, height)?;

        self.pixmap = Some(
            Pixmap::new(viewport.physical_width, viewport.physical_height).ok_or(
                SoftwareRendererError::PixmapAllocation {
                    width: viewport.physical_width,

                    height: viewport.physical_height,
                },
            )?,
        );

        self.present_pixels =
            vec![0; viewport.physical_width as usize * viewport.physical_height as usize];

        Ok(())
    }
}

impl Renderer for SoftwareRenderer {
    type Error = SoftwareRendererError;

    fn resize(&mut self, viewport: Viewport) -> Result<(), Self::Error> {
        self.resize_surface(viewport)
    }

    fn render(
        &mut self,
        display_list: &DisplayList,
        dirty_bounds: Rect,
    ) -> Result<(), Self::Error> {
        let Some(pixmap) = self.pixmap.as_mut() else {
            return Ok(());
        };

        pixmap.fill(SkiaColor::from_rgba8(0, 0, 0, 0));

        let scale = valid_scale_factor(self.viewport.scale_factor);

        let viewport_bounds = self.viewport.logical_bounds();

        let Some(dirty_bounds) = dirty_bounds.intersection(viewport_bounds) else {
            return Ok(());
        };

        let transform = Transform::from_scale(scale, scale);

        let dirty_mask = create_clip_mask(
            dirty_bounds,
            None,
            self.viewport.physical_width,
            self.viewport.physical_height,
            transform,
        )?;

        let mut clip_stack = vec![dirty_mask];

        for command in display_list.commands() {
            match command {
                DrawCommand::Clear { color } => {
                    pixmap.fill(SkiaColor::from_rgba8(
                        color.red,
                        color.green,
                        color.blue,
                        color.alpha,
                    ));
                }

                DrawCommand::FillRect { rect, color } => {
                    if rect.intersection(dirty_bounds).is_none() {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let paint = solid_paint(*color);

                    pixmap.fill_rect(rect, &paint, transform, clip_stack.last());
                }

                DrawCommand::FillRoundedRect {
                    rect,
                    radius,
                    color,
                } => {
                    if rect.intersection(dirty_bounds).is_none() {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = rounded_rect_path(rect, *radius);

                    let paint = solid_paint(*color);

                    pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        transform,
                        clip_stack.last(),
                    );
                }

                DrawCommand::StrokeRect { rect, color, width } => {
                    if rect
                        .expanded(*width * 0.5 + 1.0)
                        .intersection(dirty_bounds)
                        .is_none()
                    {
                        continue;
                    }

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

                    pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
                }

                DrawCommand::StrokeRoundedRect {
                    rect,
                    radius,
                    color,
                    width,
                } => {
                    if rect
                        .expanded(*width * 0.5 + 1.0)
                        .intersection(dirty_bounds)
                        .is_none()
                    {
                        continue;
                    }

                    if !width.is_finite() || *width <= 0.0 {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = rounded_rect_path(rect, *radius);

                    let paint = solid_paint(*color);

                    let stroke = Stroke {
                        width: *width,

                        ..Stroke::default()
                    };

                    pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
                }

                DrawCommand::FillEllipse { rect, color } => {
                    if rect.intersection(dirty_bounds).is_none() {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = ellipse_path(rect);
                    let paint = solid_paint(*color);

                    pixmap.fill_path(
                        &path,
                        &paint,
                        FillRule::Winding,
                        transform,
                        clip_stack.last(),
                    );
                }

                DrawCommand::StrokeEllipse { rect, color, width } => {
                    if rect
                        .expanded(*width * 0.5 + 1.0)
                        .intersection(dirty_bounds)
                        .is_none()
                    {
                        continue;
                    }

                    if !width.is_finite() || *width <= 0.0 {
                        continue;
                    }

                    let Some(rect) = to_skia_rect(*rect) else {
                        continue;
                    };

                    let path = ellipse_path(rect);
                    let paint = solid_paint(*color);

                    let stroke = Stroke {
                        width: *width,
                        ..Stroke::default()
                    };

                    pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
                }

                DrawCommand::PushClip { rect } => {
                    let mask = create_clip_mask(
                        *rect,
                        clip_stack.last(),
                        self.viewport.physical_width,
                        self.viewport.physical_height,
                        transform,
                    )?;

                    clip_stack.push(mask);
                }

                DrawCommand::PushRoundedClip { rect, radius } => {
                    let mask = create_rounded_clip_mask(
                        *rect,
                        *radius,
                        clip_stack.last(),
                        self.viewport.physical_width,
                        self.viewport.physical_height,
                        transform,
                    )?;

                    clip_stack.push(mask);
                }

                DrawCommand::PopClip => {
                    if clip_stack.len() <= 1 {
                        return Err(SoftwareRendererError::ClipStackUnderflow);
                    }

                    clip_stack.pop();
                }

                DrawCommand::DrawText { command } => {
                    if command.bounds.intersection(dirty_bounds).is_none() {
                        continue;
                    }

                    draw_text_command(
                        pixmap,
                        &mut self.font_system,
                        &mut self.swash_cache,
                        &mut self.text_layout_cache,
                        command,
                        scale,
                        clip_stack.last(),
                    );
                }

                DrawCommand::DrawImage { command } => {
                    if command.bounds.intersection(dirty_bounds).is_none() {
                        continue;
                    }

                    draw_image_command(pixmap, command, scale, clip_stack.last())?;
                }

                DrawCommand::DrawSvg { command } => {
                    if command.bounds.intersection(dirty_bounds).is_none() {
                        continue;
                    }

                    draw_svg_command(pixmap, command, scale, clip_stack.last())?;
                }
            }
        }

        if clip_stack.len() != 1 {
            return Err(SoftwareRendererError::UnclosedClipStack {
                depth: clip_stack.len() - 1,
            });
        }

        copy_pixmap_to_surface(
            pixmap,
            &mut self.present_pixels,
            &mut self.surface,
            dirty_bounds,
            scale,
        )?;

        Ok(())
    }
}

fn draw_image_command(
    target: &mut Pixmap,
    command: &ImageCommand,
    display_scale: f32,
    clip: Option<&Mask>,
) -> Result<(), SoftwareRendererError> {
    let bounds = command.bounds;

    if !is_valid_image_bounds(bounds) {
        return Ok(());
    }

    let image_width = command.image.width();
    let image_height = command.image.height();

    if image_width == 0 || image_height == 0 {
        return Ok(());
    }

    let Some(source) = PixmapRef::from_bytes(
        command.image.premultiplied_rgba8(),
        image_width,
        image_height,
    ) else {
        return Ok(());
    };

    let destination_width = bounds.size.width * display_scale;

    let destination_height = bounds.size.height * display_scale;

    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return Ok(());
    }

    let translate_x = bounds.origin.x * display_scale;

    let translate_y = bounds.origin.y * display_scale;

    if !translate_x.is_finite() || !translate_y.is_finite() {
        return Ok(());
    }

    let quality = image_filter_quality(command.sampling);

    let supersample = if command.sampling == ImageSampling::Nearest {
        1.0
    } else {
        image_supersample_scale(
            destination_width,
            destination_height,
            image_width as f32,
            image_height as f32,
        )
    };

    if supersample <= 1.0 {
        let scale_x = destination_width / image_width as f32;

        let scale_y = destination_height / image_height as f32;

        if !scale_x.is_finite() || !scale_y.is_finite() || scale_x <= 0.0 || scale_y <= 0.0 {
            return Ok(());
        }

        let transform = Transform::from_row(scale_x, 0.0, 0.0, scale_y, translate_x, translate_y);

        let paint = PixmapPaint {
            opacity: sanitize_image_opacity(command.opacity),

            quality,

            ..PixmapPaint::default()
        };

        target.draw_pixmap(0, 0, source, &paint, transform, clip);

        return Ok(());
    }

    let raster_width = (destination_width * supersample).ceil() as u32;

    let raster_height = (destination_height * supersample).ceil() as u32;

    if raster_width == 0 || raster_height == 0 {
        return Ok(());
    }

    let mut raster = Pixmap::new(raster_width, raster_height).ok_or(
        SoftwareRendererError::ImagePixmapAllocation {
            width: raster_width,
            height: raster_height,
        },
    )?;

    let first_scale_x = raster_width as f32 / image_width as f32;

    let first_scale_y = raster_height as f32 / image_height as f32;

    if !first_scale_x.is_finite()
        || !first_scale_y.is_finite()
        || first_scale_x <= 0.0
        || first_scale_y <= 0.0
    {
        return Ok(());
    }

    let first_transform = Transform::from_scale(first_scale_x, first_scale_y);

    let first_paint = PixmapPaint {
        opacity: 1.0,
        quality: FilterQuality::Bicubic,

        ..PixmapPaint::default()
    };

    raster.draw_pixmap(0, 0, source, &first_paint, first_transform, None);

    let second_transform = Transform::from_row(
        destination_width / raster_width as f32,
        0.0,
        0.0,
        destination_height / raster_height as f32,
        translate_x,
        translate_y,
    );

    let second_paint = PixmapPaint {
        opacity: sanitize_image_opacity(command.opacity),

        quality: FilterQuality::Bicubic,

        ..PixmapPaint::default()
    };

    target.draw_pixmap(0, 0, raster.as_ref(), &second_paint, second_transform, clip);

    Ok(())
}

fn image_filter_quality(sampling: ImageSampling) -> FilterQuality {
    match sampling {
        ImageSampling::Nearest => FilterQuality::Nearest,
        ImageSampling::Bilinear => FilterQuality::Bilinear,
        ImageSampling::Bicubic => FilterQuality::Bicubic,
    }
}

fn image_supersample_scale(
    destination_width: f32,
    destination_height: f32,
    source_width: f32,
    source_height: f32,
) -> f32 {
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || !source_width.is_finite()
        || !source_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
        || source_width <= 0.0
        || source_height <= 0.0
    {
        return 1.0;
    }

    if destination_width.max(destination_height) > IMAGE_SMALL_RENDER_LIMIT {
        return 1.0;
    }

    let downscale_x = source_width / destination_width;

    let downscale_y = source_height / destination_height;

    let downscale = downscale_x.max(downscale_y);

    if downscale <= 1.5 {
        return 1.0;
    }

    IMAGE_SMALL_RENDER_SUPERSAMPLE
}

fn draw_svg_command(
    target: &mut Pixmap,
    command: &SvgCommand,
    display_scale: f32,
    clip: Option<&Mask>,
) -> Result<(), SoftwareRendererError> {
    let bounds = command.bounds;

    if !is_valid_image_bounds(bounds) {
        return Ok(());
    }

    let svg_width = command.svg.width();
    let svg_height = command.svg.height();

    if !svg_width.is_finite() || !svg_height.is_finite() || svg_width <= 0.0 || svg_height <= 0.0 {
        return Ok(());
    }

    let destination_width = bounds.size.width * display_scale;

    let destination_height = bounds.size.height * display_scale;

    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return Ok(());
    }

    let supersample = svg_supersample_scale(destination_width, destination_height);

    let raster_width = (destination_width * supersample).ceil() as u32;

    let raster_height = (destination_height * supersample).ceil() as u32;

    if raster_width == 0 || raster_height == 0 {
        return Ok(());
    }

    let mut raster = Pixmap::new(raster_width, raster_height).ok_or(
        SoftwareRendererError::SvgPixmapAllocation {
            width: raster_width,
            height: raster_height,
        },
    )?;

    let render_transform = Transform::from_scale(
        raster_width as f32 / svg_width,
        raster_height as f32 / svg_height,
    );

    resvg::render(command.svg.tree(), render_transform, &mut raster.as_mut());

    if let Some(tint) = command.tint {
        tint_svg_pixmap(&mut raster, tint);
    }

    let translate_x = bounds.origin.x * display_scale;

    let translate_y = bounds.origin.y * display_scale;

    if !translate_x.is_finite() || !translate_y.is_finite() {
        return Ok(());
    }

    let composite_transform = Transform::from_row(
        destination_width / raster_width as f32,
        0.0,
        0.0,
        destination_height / raster_height as f32,
        translate_x,
        translate_y,
    );

    let paint = PixmapPaint {
        opacity: sanitize_image_opacity(command.opacity),

        quality: FilterQuality::Bicubic,

        ..PixmapPaint::default()
    };

    target.draw_pixmap(0, 0, raster.as_ref(), &paint, composite_transform, clip);

    Ok(())
}

fn svg_supersample_scale(destination_width: f32, destination_height: f32) -> f32 {
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return 1.0;
    }

    if destination_width.max(destination_height) <= SVG_SMALL_RENDER_LIMIT {
        SVG_SMALL_RENDER_SUPERSAMPLE
    } else {
        1.0
    }
}

fn is_valid_image_bounds(bounds: Rect) -> bool {
    bounds.origin.x.is_finite()
        && bounds.origin.y.is_finite()
        && bounds.size.width.is_finite()
        && bounds.size.height.is_finite()
        && bounds.size.width > 0.0
        && bounds.size.height > 0.0
}

fn sanitize_image_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn create_clip_mask(
    rect: Rect,
    previous: Option<&Mask>,
    physical_width: u32,
    physical_height: u32,
    transform: Transform,
) -> Result<Mask, SoftwareRendererError> {
    let path = to_skia_rect(rect).map(PathBuilder::from_rect);

    create_path_clip_mask(path, previous, physical_width, physical_height, transform)
}

fn create_rounded_clip_mask(
    rect: Rect,
    radius: f32,
    previous: Option<&Mask>,
    physical_width: u32,
    physical_height: u32,
    transform: Transform,
) -> Result<Mask, SoftwareRendererError> {
    let path = to_skia_rect(rect).map(|rect| rounded_rect_path(rect, radius));

    create_path_clip_mask(path, previous, physical_width, physical_height, transform)
}

fn create_path_clip_mask(
    path: Option<Path>,
    previous: Option<&Mask>,
    physical_width: u32,
    physical_height: u32,
    transform: Transform,
) -> Result<Mask, SoftwareRendererError> {
    let has_previous = previous.is_some();

    let mut mask = match previous {
        Some(previous) => previous.clone(),

        None => Mask::new(physical_width, physical_height).ok_or(
            SoftwareRendererError::ClipMaskAllocation {
                width: physical_width,
                height: physical_height,
            },
        )?,
    };

    let Some(path) = path else {
        mask.clear();

        return Ok(mask);
    };

    if has_previous {
        mask.intersect_path(&path, FillRule::Winding, true, transform);
    } else {
        mask.clear();

        mask.fill_path(&path, FillRule::Winding, true, transform);
    }

    Ok(mask)
}

fn copy_pixmap_to_surface(
    pixmap: &Pixmap,
    present_pixels: &mut Vec<u32>,
    surface: &mut Surface<OwnedDisplayHandle, Rc<Window>>,
    dirty_bounds: Rect,
    scale: f32,
) -> Result<(), SoftBufferError> {
    let physical_width = pixmap.width() as usize;
    let physical_height = pixmap.height() as usize;

    let left = (dirty_bounds.origin.x * scale).floor().max(0.0) as usize;

    let top = (dirty_bounds.origin.y * scale).floor().max(0.0) as usize;

    let right = ((dirty_bounds.origin.x + dirty_bounds.size.width) * scale)
        .ceil()
        .max(0.0) as usize;

    let bottom = ((dirty_bounds.origin.y + dirty_bounds.size.height) * scale)
        .ceil()
        .max(0.0) as usize;

    let right = right.min(physical_width);
    let bottom = bottom.min(physical_height);

    let source = pixmap.data();

    for y in top..bottom {
        let row_start = y * physical_width;

        for x in left..right {
            let pixel_index = row_start + x;
            let source_index = pixel_index * 4;

            let red = u32::from(source[source_index]);
            let green = u32::from(source[source_index + 1]);
            let blue = u32::from(source[source_index + 2]);

            present_pixels[pixel_index] = (red << 16) | (green << 8) | blue;
        }
    }

    let mut buffer = surface.buffer_mut()?;

    buffer.copy_from_slice(present_pixels);
    buffer.present()?;

    Ok(())
}

fn solid_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();

    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);

    paint.anti_alias = true;

    paint
}

fn to_skia_rect(rect: Rect) -> Option<SkiaRect> {
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

    SkiaRect::from_xywh(x, y, width, height)
}

fn rounded_rect_path(rect: SkiaRect, radius: f32) -> Path {
    let radius = if radius.is_finite() {
        radius.max(0.0).min(rect.width().min(rect.height()) / 2.0)
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

    builder.quad_to(right, top, right, top + radius);

    builder.line_to(right, bottom - radius);

    builder.quad_to(right, bottom, right - radius, bottom);

    builder.line_to(left + radius, bottom);

    builder.quad_to(left, bottom, left, bottom - radius);

    builder.line_to(left, top + radius);

    builder.quad_to(left, top, left + radius, top);

    builder.close();

    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn ellipse_path(rect: SkiaRect) -> Path {
    const KAPPA: f32 = 0.552_284_8;

    let center_x = (rect.left() + rect.right()) / 2.0;
    let center_y = (rect.top() + rect.bottom()) / 2.0;

    let radius_x = rect.width() / 2.0;
    let radius_y = rect.height() / 2.0;

    let control_x = radius_x * KAPPA;
    let control_y = radius_y * KAPPA;

    let mut builder = PathBuilder::new();

    builder.move_to(center_x + radius_x, center_y);

    builder.cubic_to(
        center_x + radius_x,
        center_y + control_y,
        center_x + control_x,
        center_y + radius_y,
        center_x,
        center_y + radius_y,
    );

    builder.cubic_to(
        center_x - control_x,
        center_y + radius_y,
        center_x - radius_x,
        center_y + control_y,
        center_x - radius_x,
        center_y,
    );

    builder.cubic_to(
        center_x - radius_x,
        center_y - control_y,
        center_x - control_x,
        center_y - radius_y,
        center_x,
        center_y - radius_y,
    );

    builder.cubic_to(
        center_x + control_x,
        center_y - radius_y,
        center_x + radius_x,
        center_y - control_y,
        center_x + radius_x,
        center_y,
    );

    builder.close();

    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn valid_scale_factor(scale_factor: f64) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor as f32
    } else {
        1.0
    }
}

fn draw_text_command(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    command: &TextCommand,
    scale: f32,
    clip: Option<&Mask>,
) {
    if command.text.is_empty()
        || command.bounds.size.width <= 0.0
        || command.bounds.size.height <= 0.0
    {
        return;
    }

    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    let font_size = (command.font_size * scale).max(1.0);

    let line_height = (command.line_height * scale).max(font_size);

    let width = (command.bounds.size.width * scale).max(0.0);

    let height = (command.bounds.size.height * scale).max(0.0);

    let origin_x = (command.bounds.origin.x * scale).round();

    let origin_y = command.bounds.origin.y * scale;

    let key = TextLayoutKey::new(command, scale);

    if !layout_cache.contains_key(&key) {
        if layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            layout_cache.clear();
        }

        let metrics = Metrics::new(font_size, line_height);

        let mut buffer = Buffer::new(font_system, metrics);

        {
            let mut buffer_with_font_system = buffer.borrow_with(font_system);

            buffer_with_font_system.set_size(Some(width), Some(height));

            let attrs = Attrs::new()
                .family(Family::Name(command.font_family.as_str()))
                .weight(Weight(command.weight.clamp(1, 1000)));

            buffer_with_font_system.set_text(
                command.text.as_str(),
                &attrs,
                Shaping::Advanced,
                command.alignment.to_cosmic(),
            );
        }

        layout_cache.insert(key.clone(), buffer);
    }

    let buffer = layout_cache
        .get_mut(&key)
        .expect("Text layout cache does not exist");

    let mut buffer = buffer.borrow_with(font_system);

    let text_color = CosmicColor::rgba(
        command.color.red,
        command.color.green,
        command.color.blue,
        command.color.alpha,
    );

    let Some(text_clip) = SkiaRect::from_xywh(origin_x, origin_y, width, height) else {
        return;
    };

    let mut physical_glyphs = Vec::new();

    for run in buffer.layout_runs() {
        let baseline_y = (origin_y + run.line_y).round();

        for glyph in run.glyphs {
            physical_glyphs.push(glyph.physical((origin_x, baseline_y), 1.0));
        }
    }

    drop(buffer);

    for physical_glyph in physical_glyphs {
        swash_cache.with_pixels(
            font_system,
            physical_glyph.cache_key,
            text_color,
            |x, y, color| {
                let draw_x = physical_glyph.x + x;

                let draw_y = physical_glyph.y + y;

                let Some(pixel_rect) = SkiaRect::from_xywh(draw_x as f32, draw_y as f32, 1.0, 1.0)
                else {
                    return;
                };

                let Some(rect) = intersect_rect(pixel_rect, text_clip) else {
                    return;
                };

                let (red, green, blue, alpha) = color.as_rgba_tuple();

                if alpha == 0 {
                    return;
                }

                let mut paint = Paint::default();

                paint.set_color_rgba8(red, green, blue, alpha);
                paint.anti_alias = false;

                pixmap.fill_rect(rect, &paint, Transform::identity(), clip);
            },
        );
    }
}

fn intersect_rect(first: SkiaRect, second: SkiaRect) -> Option<SkiaRect> {
    let left = first.left().max(second.left());
    let top = first.top().max(second.top());
    let right = first.right().min(second.right());
    let bottom = first.bottom().min(second.bottom());

    let width = right - left;
    let height = bottom - top;

    if width <= 0.0 || height <= 0.0 {
        return None;
    }

    SkiaRect::from_xywh(left, top, width, height)
}

fn tint_svg_pixmap(pixmap: &mut Pixmap, tint: Color) {
    for pixel in pixmap.data_mut().chunks_exact_mut(4) {
        let alpha = multiply_channel(pixel[3], tint.alpha);

        pixel[0] = multiply_channel(tint.red, alpha);

        pixel[1] = multiply_channel(tint.green, alpha);

        pixel[2] = multiply_channel(tint.blue, alpha);

        pixel[3] = alpha;
    }
}

fn multiply_channel(first: u8, second: u8) -> u8 {
    let value = u16::from(first) * u16::from(second);

    ((value + 127) / 255) as u8
}

//! softbufferとtiny-skiaを使用したLinux向けソフトウェアレンダラー

use std::collections::HashMap;
use std::num::NonZeroU32;
use std::rc::Rc;

use softbuffer::{Context, SoftBufferError, Surface};
use tiny_skia::{
    Color as SkiaColor, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Rect as SkiaRect, Stroke,
    Transform,
};
use winit::event_loop::OwnedDisplayHandle;
use winit::window::Window;

use crate::draw_command::{DisplayList, DrawCommand, TextCommand};
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
}

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;

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
            font_system: FontSystem::new(),
            swash_cache: SwashCache::new(),
            text_layout_cache: HashMap::new(),
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

        Ok(())
    }
}

impl Renderer for SoftwareRenderer {
    type Error = SoftwareRendererError;

    fn resize(&mut self, viewport: Viewport) -> Result<(), Self::Error> {
        self.resize_surface(viewport)
    }

    fn render(&mut self, display_list: &DisplayList) -> Result<(), Self::Error> {
        let Some(pixmap) = self.pixmap.as_mut() else {
            return Ok(());
        };

        pixmap.fill(SkiaColor::from_rgba8(0, 0, 0, 0));

        let scale = valid_scale_factor(self.viewport.scale_factor);

        let transform = Transform::from_scale(scale, scale);

        let mut clip_stack: Vec<Mask> = Vec::new();

        for command in display_list.commands() {
            match command {
                DrawCommand::Clear { color } => {
                    /*
                     * Clearはクリップの影響を受けず、
                     * 描画先全体を初期化します。
                     */
                    pixmap.fill(to_skia_color(*color));
                }

                DrawCommand::FillRect { rect, color } => {
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

                DrawCommand::PopClip => {
                    if clip_stack.pop().is_none() {
                        return Err(SoftwareRendererError::ClipStackUnderflow);
                    }
                }

                DrawCommand::DrawText { command } => {
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
            }
        }

        if !clip_stack.is_empty() {
            return Err(SoftwareRendererError::UnclosedClipStack {
                depth: clip_stack.len(),
            });
        }

        copy_pixmap_to_surface(pixmap, &mut self.surface)?;

        Ok(())
    }
}

fn create_clip_mask(
    rect: Rect,
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

    let Some(rect) = to_skia_rect(rect) else {
        /*
         * 不正なクリップ領域を無視すると、
         * クリップされずに描画されてしまいます。
         *
         * そのため空のマスクにして、
         * 何も描画されない状態にします。
         */
        mask.clear();

        return Ok(mask);
    };

    let path = PathBuilder::from_rect(rect);

    if has_previous {
        mask.intersect_path(&path, FillRule::Winding, false, transform);
    } else {
        mask.clear();

        mask.fill_path(&path, FillRule::Winding, false, transform);
    }

    Ok(mask)
}

fn copy_pixmap_to_surface(
    pixmap: &Pixmap,
    surface: &mut Surface<OwnedDisplayHandle, Rc<Window>>,
) -> Result<(), SoftBufferError> {
    let mut buffer = surface.buffer_mut()?;

    for (destination, rgba) in buffer.iter_mut().zip(pixmap.data().chunks_exact(4)) {
        let red = u32::from(rgba[0]);

        let green = u32::from(rgba[1]);

        let blue = u32::from(rgba[2]);

        *destination = (red << 16) | (green << 8) | blue;
    }

    buffer.present()?;

    Ok(())
}

fn solid_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();

    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);

    paint.anti_alias = true;

    paint
}

fn to_skia_color(color: Color) -> SkiaColor {
    SkiaColor::from_rgba8(color.red, color.green, color.blue, color.alpha)
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

    let origin_x = command.bounds.origin.x * scale;

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

    let text_clip = SkiaRect::from_xywh(
        command.bounds.origin.x * scale,
        command.bounds.origin.y * scale,
        command.bounds.size.width * scale,
        command.bounds.size.height * scale,
    );

    buffer.draw(swash_cache, text_color, |x, y, width, height, color| {
        let draw_x = origin_x + x as f32;

        let draw_y = origin_y + y as f32;

        let Some(glyph_rect) = SkiaRect::from_xywh(draw_x, draw_y, width as f32, height as f32)
        else {
            return;
        };

        let Some(text_clip) = text_clip else {
            return;
        };

        let Some(rect) = intersect_rect(glyph_rect, text_clip) else {
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
    });
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

use super::renderer::valid_scale_factor;
use super::*;
use cosmic_text::{SwashContent, SwashImage};
use std::sync::Arc;

use mochios_viewkit_gpu_protocol::{
    ATLAS_HEIGHT, ATLAS_WIDTH, HEADER_LEN, VERTEX_STRIDE, encode_header,
};
const CURVE_SEGMENTS: usize = 48;
const IMAGE_RASTER_CACHE_CAPACITY: usize = 128;
const SVG_RASTER_CACHE_CAPACITY: usize = 128;

#[derive(Clone, Copy)]
struct Vertex {
    position: [f32; 3],
    uv: [f32; 2],
    color: [f32; 4],
}

#[derive(Clone, Copy)]
struct AtlasRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

struct SvgRasterCacheEntry {
    svg: SvgData,
    width: u32,
    height: u32,
    tint: Option<Color>,
    pixels: Arc<[u8]>,
}

struct ImageRasterCacheEntry {
    image: ImageData,
    pixels: Arc<[u8]>,
}

pub(super) struct GpuSceneRenderer {
    vertices: Vec<Vertex>,
    atlas: Vec<u8>,
    previous_atlas: Vec<u8>,
    previous_atlas_valid: bool,
    atlas_x: u32,
    atlas_y: u32,
    atlas_row_height: u32,
    clips: Vec<Rect>,
    image_raster_cache: Vec<ImageRasterCacheEntry>,
    svg_raster_cache: Vec<SvgRasterCacheEntry>,
    frame_width: u32,
    frame_height: u32,
    frame_valid: bool,
}

impl GpuSceneRenderer {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            atlas: Vec::new(),
            previous_atlas: Vec::new(),
            previous_atlas_valid: false,
            atlas_x: 1,
            atlas_y: 0,
            atlas_row_height: 1,
            clips: Vec::new(),
            image_raster_cache: Vec::new(),
            svg_raster_cache: Vec::new(),
            frame_width: 0,
            frame_height: 0,
            frame_valid: false,
        }
    }

    pub(super) fn render(
        &mut self,
        viewport: Viewport,
        dirty_bounds: Rect,
        display_list: &DisplayList,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        text_layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
        transparent_clear: bool,
        output: &mut Vec<u8>,
    ) -> Result<(), MochiOsBackendError> {
        self.reset()?;
        let scale = valid_scale_factor(viewport.scale_factor);
        let viewport_bounds = viewport.logical_bounds();
        let damage = if self.frame_valid
            && self.frame_width == viewport.physical_width
            && self.frame_height == viewport.physical_height
        {
            dirty_bounds
                .intersection(viewport_bounds)
                .unwrap_or(viewport_bounds)
        } else {
            viewport_bounds
        };
        self.clips.push(damage);
        for command in display_list.commands() {
            match command {
                DrawCommand::Clear { color } => {
                    self.solid_rect(
                        viewport.logical_bounds(),
                        if transparent_clear {
                            Color::TRANSPARENT
                        } else {
                            *color
                        },
                        viewport,
                    );
                }
                DrawCommand::FillRect { rect, color } => self.solid_rect(*rect, *color, viewport),
                DrawCommand::FillRoundedRect {
                    rect,
                    radius,
                    color,
                } => {
                    self.rounded_rect(*rect, *radius, *color, viewport);
                }
                DrawCommand::FillEllipse { rect, color } => {
                    self.ellipse(*rect, *color, viewport);
                }
                DrawCommand::StrokeRect { rect, color, width } => {
                    self.stroke_rect(*rect, *width, *color, viewport);
                }
                DrawCommand::StrokeRoundedRect {
                    rect,
                    radius,
                    color,
                    width,
                } => {
                    self.stroke_rounded_rect(*rect, *radius, *width, *color, viewport);
                }
                DrawCommand::StrokeEllipse { rect, color, width } => {
                    self.stroke_ellipse(*rect, *width, *color, viewport);
                }
                DrawCommand::DrawText { command } => self.text(
                    command,
                    scale,
                    viewport,
                    font_system,
                    swash_cache,
                    text_layout_cache,
                )?,
                DrawCommand::DrawImage { command } => self.image(command, scale, viewport)?,
                DrawCommand::DrawSvg { command } => self.svg(command, scale, viewport)?,
                DrawCommand::PushClip { rect } | DrawCommand::PushRoundedClip { rect, .. } => {
                    let current = self.current_clip();
                    self.clips.push(
                        rect.intersection(current)
                            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
                    );
                }
                DrawCommand::PopClip => {
                    if self.clips.len() > 1 {
                        self.clips.pop();
                    }
                }
            }
        }
        self.encode(viewport, output)?;
        self.frame_width = viewport.physical_width;
        self.frame_height = viewport.physical_height;
        self.frame_valid = true;
        Ok(())
    }

    fn reset(&mut self) -> Result<(), MochiOsBackendError> {
        self.vertices.clear();
        self.clips.clear();
        self.atlas_x = 1;
        self.atlas_y = 0;
        self.atlas_row_height = 1;
        core::mem::swap(&mut self.atlas, &mut self.previous_atlas);
        let atlas_len = (ATLAS_WIDTH as usize)
            .checked_mul(ATLAS_HEIGHT as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        self.atlas.resize(atlas_len, 0);
        self.atlas[..4].copy_from_slice(&[255, 255, 255, 255]);
        Ok(())
    }

    fn encode(
        &mut self,
        viewport: Viewport,
        output: &mut Vec<u8>,
    ) -> Result<(), MochiOsBackendError> {
        let vertex_count = u32::try_from(self.vertices.len())
            .map_err(|_| MochiOsBackendError::ArithmeticOverflow)?;
        let vertex_bytes = self
            .vertices
            .len()
            .checked_mul(VERTEX_STRIDE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let atlas_used_height = self
            .atlas_y
            .saturating_add(self.atlas_row_height)
            .clamp(1, ATLAS_HEIGHT);
        let row_bytes = ATLAS_WIDTH as usize * 4;
        let (atlas_data_y, atlas_data_height) = if self.previous_atlas_valid {
            let changed = (0..atlas_used_height as usize).filter(|row| {
                let start = row * row_bytes;
                let end = start + row_bytes;
                self.atlas[start..end] != self.previous_atlas[start..end]
            });
            let first = changed.clone().next();
            let last = changed.last();
            match (first, last) {
                (Some(first), Some(last)) => (first as u32, (last - first + 1) as u32),
                _ => (0, 0),
            }
        } else {
            (0, atlas_used_height)
        };
        let atlas_bytes = (ATLAS_WIDTH as usize)
            .checked_mul(atlas_data_height as usize)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let total = HEADER_LEN
            .checked_add(vertex_bytes)
            .and_then(|size| size.checked_add(atlas_bytes))
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        output.clear();
        output.resize(total, 0);
        encode_header(
            output,
            viewport.physical_width,
            viewport.physical_height,
            vertex_count,
            ATLAS_WIDTH,
            ATLAS_HEIGHT,
            atlas_data_y,
            atlas_data_height,
        )
        .map_err(|_| MochiOsBackendError::InvalidWindowSize)?;
        let mut offset = HEADER_LEN;
        for vertex in &self.vertices {
            for value in vertex
                .position
                .into_iter()
                .chain(vertex.uv)
                .chain(vertex.color)
            {
                output[offset..offset + 4].copy_from_slice(&value.to_bits().to_le_bytes());
                offset += 4;
            }
        }
        let atlas_start = atlas_data_y as usize * row_bytes;
        output[offset..].copy_from_slice(&self.atlas[atlas_start..atlas_start + atlas_bytes]);
        self.previous_atlas_valid = true;
        Ok(())
    }

    fn current_clip(&self) -> Rect {
        self.clips
            .last()
            .copied()
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
    }

    fn solid_rect(&mut self, rect: Rect, color: Color, viewport: Viewport) {
        let Some(rect) = rect.intersection(self.current_clip()) else {
            return;
        };
        self.quad(
            rect,
            AtlasRect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            color,
            viewport,
        );
    }

    fn rounded_rect(&mut self, rect: Rect, radius: f32, color: Color, viewport: Viewport) {
        self.fan(rect, radius.max(0.0), color, viewport, false);
    }

    fn ellipse(&mut self, rect: Rect, color: Color, viewport: Viewport) {
        self.fan(
            rect,
            rect.size.width.min(rect.size.height) * 0.5,
            color,
            viewport,
            true,
        );
    }

    fn fan(&mut self, rect: Rect, radius: f32, color: Color, viewport: Viewport, ellipse: bool) {
        let Some(clipped) = rect.intersection(self.current_clip()) else {
            return;
        };
        if clipped != rect {
            self.solid_rect(clipped, color, viewport);
            return;
        }
        let inner_rect = inset_rect(rect, 0.5);
        let outer_rect = rect.expanded(0.5);
        let center = (
            inner_rect.origin.x + inner_rect.size.width * 0.5,
            inner_rect.origin.y + inner_rect.size.height * 0.5,
        );
        let points = if ellipse {
            ellipse_points(inner_rect)
        } else {
            rounded_points(inner_rect, (radius - 0.5).max(0.0))
        };
        let outer = if ellipse {
            ellipse_points(outer_rect)
        } else {
            rounded_points(outer_rect, radius + 0.5)
        };
        if points.len() < 3 {
            return;
        }
        let white = AtlasRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        let uv = self.uv_center(white);
        for index in 0..points.len() {
            let next = (index + 1) % points.len();
            self.push_triangle(
                [(center.0, center.1), points[index], points[next]],
                [uv; 3],
                color,
                viewport,
            );
            self.push_triangle_colors(
                [points[index], outer[index], outer[next]],
                [uv; 3],
                [color, transparent(color), transparent(color)],
                viewport,
            );
            self.push_triangle_colors(
                [points[index], outer[next], points[next]],
                [uv; 3],
                [color, transparent(color), color],
                viewport,
            );
        }
    }

    fn stroke_rect(&mut self, rect: Rect, width: f32, color: Color, viewport: Viewport) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let half = width * 0.5;
        self.solid_rect(
            Rect::new(
                rect.origin.x - half,
                rect.origin.y - half,
                rect.size.width + width,
                width,
            ),
            color,
            viewport,
        );
        self.solid_rect(
            Rect::new(
                rect.origin.x - half,
                rect.origin.y + rect.size.height - half,
                rect.size.width + width,
                width,
            ),
            color,
            viewport,
        );
        self.solid_rect(
            Rect::new(
                rect.origin.x - half,
                rect.origin.y + half,
                width,
                (rect.size.height - width).max(0.0),
            ),
            color,
            viewport,
        );
        self.solid_rect(
            Rect::new(
                rect.origin.x + rect.size.width - half,
                rect.origin.y + half,
                width,
                (rect.size.height - width).max(0.0),
            ),
            color,
            viewport,
        );
    }

    fn stroke_rounded_rect(
        &mut self,
        rect: Rect,
        radius: f32,
        width: f32,
        color: Color,
        viewport: Viewport,
    ) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let outer = rounded_points(rect.expanded(width * 0.5), radius + width * 0.5);
        let inner_rect = Rect::new(
            rect.origin.x + width * 0.5,
            rect.origin.y + width * 0.5,
            (rect.size.width - width).max(0.0),
            (rect.size.height - width).max(0.0),
        );
        let inner = rounded_points(inner_rect, (radius - width * 0.5).max(0.0));
        self.ring(&outer, &inner, color, viewport);
    }

    fn stroke_ellipse(&mut self, rect: Rect, width: f32, color: Color, viewport: Viewport) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let outer = ellipse_points(rect.expanded(width * 0.5));
        let inner = ellipse_points(Rect::new(
            rect.origin.x + width * 0.5,
            rect.origin.y + width * 0.5,
            (rect.size.width - width).max(0.0),
            (rect.size.height - width).max(0.0),
        ));
        self.ring(&outer, &inner, color, viewport);
    }

    fn ring(
        &mut self,
        outer: &[(f32, f32)],
        inner: &[(f32, f32)],
        color: Color,
        viewport: Viewport,
    ) {
        if outer.len() != inner.len() || outer.len() < 3 {
            return;
        }
        let uv = self.uv_center(AtlasRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        });
        for i in 0..outer.len() {
            let n = (i + 1) % outer.len();
            self.push_triangle([outer[i], outer[n], inner[n]], [uv; 3], color, viewport);
            self.push_triangle([outer[i], inner[n], inner[i]], [uv; 3], color, viewport);
        }
    }

    fn image(
        &mut self,
        command: &ImageCommand,
        scale: f32,
        viewport: Viewport,
    ) -> Result<(), MochiOsBackendError> {
        if command.bounds.intersection(self.current_clip()).is_none() {
            return Ok(());
        }
        let pixels = if let Some(entry) = self
            .image_raster_cache
            .iter()
            .find(|entry| entry.image == command.image)
        {
            Arc::clone(&entry.pixels)
        } else {
            let pixels: Arc<[u8]> =
                Arc::from(rgba_to_bgra(command.image.premultiplied_rgba8()).into_boxed_slice());
            if self.image_raster_cache.len() >= IMAGE_RASTER_CACHE_CAPACITY {
                self.image_raster_cache.clear();
            }
            self.image_raster_cache.push(ImageRasterCacheEntry {
                image: command.image.clone(),
                pixels: Arc::clone(&pixels),
            });
            pixels
        };
        let atlas = self.pack_bgra(command.image.width(), command.image.height(), &pixels)?;
        let rect = scale_rect(command.bounds, scale);
        self.textured_quad(rect, atlas, command.opacity, viewport);
        Ok(())
    }

    fn svg(
        &mut self,
        command: &SvgCommand,
        scale: f32,
        viewport: Viewport,
    ) -> Result<(), MochiOsBackendError> {
        if command.bounds.intersection(self.current_clip()).is_none() {
            return Ok(());
        }
        let width = (command.bounds.size.width * scale).ceil().max(1.0) as u32;
        let height = (command.bounds.size.height * scale).ceil().max(1.0) as u32;
        let pixels = if let Some(entry) = self.svg_raster_cache.iter().find(|entry| {
            entry.svg == command.svg
                && entry.width == width
                && entry.height == height
                && entry.tint == command.tint
        }) {
            Arc::clone(&entry.pixels)
        } else {
            let mut pixmap =
                Pixmap::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?;
            let transform = Transform::from_scale(
                width as f32 / command.svg.width(),
                height as f32 / command.svg.height(),
            );
            resvg::render(command.svg.tree(), transform, &mut pixmap.as_mut());
            if let Some(tint) = command.tint {
                for pixel in pixmap.data_mut().chunks_exact_mut(4) {
                    let alpha = ((u16::from(pixel[3]) * u16::from(tint.alpha) + 127) / 255) as u8;
                    pixel[0] = ((u16::from(tint.red) * u16::from(alpha) + 127) / 255) as u8;
                    pixel[1] = ((u16::from(tint.green) * u16::from(alpha) + 127) / 255) as u8;
                    pixel[2] = ((u16::from(tint.blue) * u16::from(alpha) + 127) / 255) as u8;
                    pixel[3] = alpha;
                }
            }
            let pixels: Arc<[u8]> = Arc::from(rgba_to_bgra(pixmap.data()).into_boxed_slice());
            if self.svg_raster_cache.len() >= SVG_RASTER_CACHE_CAPACITY {
                self.svg_raster_cache.clear();
            }
            self.svg_raster_cache.push(SvgRasterCacheEntry {
                svg: command.svg.clone(),
                width,
                height,
                tint: command.tint,
                pixels: Arc::clone(&pixels),
            });
            pixels
        };
        let atlas = self.pack_bgra(width, height, &pixels)?;
        self.textured_quad(
            scale_rect(command.bounds, scale),
            atlas,
            command.opacity,
            viewport,
        );
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn text(
        &mut self,
        command: &TextCommand,
        scale: f32,
        viewport: Viewport,
        font_system: &mut FontSystem,
        swash_cache: &mut SwashCache,
        layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    ) -> Result<(), MochiOsBackendError> {
        if command.text.is_empty() {
            return Ok(());
        }
        if command.bounds.intersection(self.current_clip()).is_none() {
            return Ok(());
        }
        let key = TextLayoutKey::new(command, scale);
        if !layout_cache.contains_key(&key) {
            if layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
                layout_cache.clear();
            }
            let font_size = (command.font_size * scale).max(1.0);
            let line_height = (command.line_height * scale).max(font_size);
            let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
            let mut borrowed = buffer.borrow_with(font_system);
            borrowed.set_size(
                Some(command.bounds.size.width * scale),
                Some(command.bounds.size.height * scale),
            );
            let attrs = Attrs::new()
                .family(Family::Name(command.font_family.as_str()))
                .weight(Weight(command.weight.clamp(1, 1000)));
            borrowed.set_text(
                command.text.as_str(),
                &attrs,
                Shaping::Advanced,
                command.alignment.to_cosmic(),
            );
            drop(borrowed);
            layout_cache.insert(key.clone(), buffer);
        }
        let Some(buffer) = layout_cache.get_mut(&key) else {
            return Ok(());
        };
        let origin_x = (command.bounds.origin.x * scale).round();
        let origin_y = command.bounds.origin.y * scale;
        let mut borrowed = buffer.borrow_with(font_system);
        let mut glyphs = Vec::new();
        for run in borrowed.layout_runs() {
            let baseline_y = (origin_y + run.line_y).round();
            for glyph in run.glyphs {
                glyphs.push(glyph.physical((origin_x, baseline_y), 1.0));
            }
        }
        drop(borrowed);
        let text_color = CosmicColor::rgba(
            command.color.red,
            command.color.green,
            command.color.blue,
            command.color.alpha,
        );
        let text_clip = scale_rect(command.bounds, scale);
        let current_clip = self.current_clip();
        self.clips.push(
            text_clip
                .intersection(current_clip)
                .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
        );
        for glyph in glyphs {
            let Some(image) = swash_cache.get_image(font_system, glyph.cache_key).as_ref() else {
                continue;
            };
            let atlas = self.pack_glyph(image, text_color)?;
            let rect = Rect::new(
                (glyph.x + image.placement.left) as f32,
                (glyph.y - image.placement.top) as f32,
                image.placement.width as f32,
                image.placement.height as f32,
            );
            self.textured_quad(rect, atlas, 1.0, viewport);
        }
        self.clips.pop();
        Ok(())
    }

    fn textured_quad(&mut self, rect: Rect, atlas: AtlasRect, opacity: f32, viewport: Viewport) {
        let original = rect;
        let Some(rect) = original.intersection(self.current_clip()) else {
            return;
        };
        let (u0, v0, u1, v1) = self.uv_bounds(atlas);
        let left = (rect.origin.x - original.origin.x) / original.size.width;
        let top = (rect.origin.y - original.origin.y) / original.size.height;
        let right = (rect.origin.x + rect.size.width - original.origin.x) / original.size.width;
        let bottom = (rect.origin.y + rect.size.height - original.origin.y) / original.size.height;
        let uv = [
            [u0 + (u1 - u0) * left, v0 + (v1 - v0) * top],
            [u0 + (u1 - u0) * right, v0 + (v1 - v0) * top],
            [u0 + (u1 - u0) * right, v0 + (v1 - v0) * bottom],
            [u0 + (u1 - u0) * left, v0 + (v1 - v0) * bottom],
        ];
        let p = [
            (rect.origin.x, rect.origin.y),
            (rect.origin.x + rect.size.width, rect.origin.y),
            (
                rect.origin.x + rect.size.width,
                rect.origin.y + rect.size.height,
            ),
            (rect.origin.x, rect.origin.y + rect.size.height),
        ];
        let color = Color {
            red: 255,
            green: 255,
            blue: 255,
            alpha: (opacity.clamp(0.0, 1.0) * 255.0).round() as u8,
        };
        self.push_triangle([p[0], p[1], p[2]], [uv[0], uv[1], uv[2]], color, viewport);
        self.push_triangle([p[0], p[2], p[3]], [uv[0], uv[2], uv[3]], color, viewport);
    }

    fn quad(&mut self, rect: Rect, atlas: AtlasRect, color: Color, viewport: Viewport) {
        if rect.size.width <= 0.0 || rect.size.height <= 0.0 {
            return;
        }
        let (u0, v0, u1, v1) = self.uv_bounds(atlas);
        let p = [
            (rect.origin.x, rect.origin.y),
            (rect.origin.x + rect.size.width, rect.origin.y),
            (
                rect.origin.x + rect.size.width,
                rect.origin.y + rect.size.height,
            ),
            (rect.origin.x, rect.origin.y + rect.size.height),
        ];
        self.push_triangle(
            [p[0], p[1], p[2]],
            [[u0, v0], [u1, v0], [u1, v1]],
            color,
            viewport,
        );
        self.push_triangle(
            [p[0], p[2], p[3]],
            [[u0, v0], [u1, v1], [u0, v1]],
            color,
            viewport,
        );
    }

    fn push_triangle(
        &mut self,
        positions: [(f32, f32); 3],
        uv: [[f32; 2]; 3],
        color: Color,
        viewport: Viewport,
    ) {
        self.push_triangle_colors(positions, uv, [color; 3], viewport);
    }

    fn push_triangle_colors(
        &mut self,
        positions: [(f32, f32); 3],
        uv: [[f32; 2]; 3],
        colors: [Color; 3],
        viewport: Viewport,
    ) {
        for index in 0..3 {
            let color = colors[index];
            let alpha = f32::from(color.alpha) / 255.0;
            let rgba = [
                f32::from(color.red) / 255.0 * alpha,
                f32::from(color.green) / 255.0 * alpha,
                f32::from(color.blue) / 255.0 * alpha,
                alpha,
            ];
            let x = positions[index].0 / viewport.physical_width as f32 * 2.0 - 1.0;
            let y = positions[index].1 / viewport.physical_height as f32 * 2.0 - 1.0;
            self.vertices.push(Vertex {
                position: [x, y, 0.0],
                uv: uv[index],
                color: rgba,
            });
        }
    }

    fn pack_bgra(
        &mut self,
        width: u32,
        height: u32,
        bgra: &[u8],
    ) -> Result<AtlasRect, MochiOsBackendError> {
        let expected = width as usize * height as usize * 4;
        if width == 0
            || height == 0
            || bgra.len() < expected
            || width > ATLAS_WIDTH
            || height > ATLAS_HEIGHT
        {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        if self.atlas_x + width > ATLAS_WIDTH {
            self.atlas_x = 0;
            self.atlas_y = self.atlas_y.saturating_add(self.atlas_row_height);
            self.atlas_row_height = 0;
        }
        if self.atlas_y + height > ATLAS_HEIGHT {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let rect = AtlasRect {
            x: self.atlas_x,
            y: self.atlas_y,
            width,
            height,
        };
        for y in 0..height as usize {
            let source = y * width as usize * 4;
            let target = ((rect.y as usize + y) * ATLAS_WIDTH as usize + rect.x as usize) * 4;
            let byte_width = width as usize * 4;
            self.atlas[target..target + byte_width]
                .copy_from_slice(&bgra[source..source + byte_width]);
        }
        self.atlas_x = self.atlas_x.saturating_add(width + 1);
        self.atlas_row_height = self.atlas_row_height.max(height + 1);
        Ok(rect)
    }

    fn pack_glyph(
        &mut self,
        image: &SwashImage,
        color: CosmicColor,
    ) -> Result<AtlasRect, MochiOsBackendError> {
        let width = image.placement.width;
        let height = image.placement.height;
        if width == 0 || height == 0 || width > ATLAS_WIDTH || height > ATLAS_HEIGHT {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        if self.atlas_x + width > ATLAS_WIDTH {
            self.atlas_x = 0;
            self.atlas_y = self.atlas_y.saturating_add(self.atlas_row_height);
            self.atlas_row_height = 0;
        }
        if self.atlas_y + height > ATLAS_HEIGHT {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let rect = AtlasRect {
            x: self.atlas_x,
            y: self.atlas_y,
            width,
            height,
        };
        let pixel_count = width as usize * height as usize;
        let (red, green, blue, base_alpha) = color.as_rgba_tuple();
        match image.content {
            SwashContent::Mask => {
                if image.data.len() < pixel_count {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                }
                for index in 0..pixel_count {
                    let alpha =
                        ((u16::from(image.data[index]) * u16::from(base_alpha) + 127) / 255) as u8;
                    let target_x = index % width as usize;
                    let target_y = index / width as usize;
                    let target = ((rect.y as usize + target_y) * ATLAS_WIDTH as usize
                        + rect.x as usize
                        + target_x)
                        * 4;
                    self.atlas[target..target + 4].copy_from_slice(&[
                        premultiply(blue, alpha),
                        premultiply(green, alpha),
                        premultiply(red, alpha),
                        alpha,
                    ]);
                }
            }
            SwashContent::Color => {
                if image.data.len() < pixel_count.saturating_mul(4) {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                }
                for index in 0..pixel_count {
                    let source = index * 4;
                    let alpha = image.data[source + 3];
                    let target_x = index % width as usize;
                    let target_y = index / width as usize;
                    let target = ((rect.y as usize + target_y) * ATLAS_WIDTH as usize
                        + rect.x as usize
                        + target_x)
                        * 4;
                    self.atlas[target..target + 4].copy_from_slice(&[
                        premultiply(image.data[source + 2], alpha),
                        premultiply(image.data[source + 1], alpha),
                        premultiply(image.data[source], alpha),
                        alpha,
                    ]);
                }
            }
            SwashContent::SubpixelMask => {
                return Err(MochiOsBackendError::InvalidWindowSize);
            }
        }
        self.atlas_x = self.atlas_x.saturating_add(width + 1);
        self.atlas_row_height = self.atlas_row_height.max(height + 1);
        Ok(rect)
    }

    fn uv_center(&self, rect: AtlasRect) -> [f32; 2] {
        [
            (rect.x as f32 + 0.5) / ATLAS_WIDTH as f32,
            (rect.y as f32 + 0.5) / ATLAS_HEIGHT as f32,
        ]
    }

    fn uv_bounds(&self, rect: AtlasRect) -> (f32, f32, f32, f32) {
        (
            rect.x as f32 / ATLAS_WIDTH as f32,
            rect.y as f32 / ATLAS_HEIGHT as f32,
            (rect.x + rect.width) as f32 / ATLAS_WIDTH as f32,
            (rect.y + rect.height) as f32 / ATLAS_HEIGHT as f32,
        )
    }
}

fn scale_rect(rect: Rect, scale: f32) -> Rect {
    Rect::new(
        rect.origin.x * scale,
        rect.origin.y * scale,
        rect.size.width * scale,
        rect.size.height * scale,
    )
}

fn inset_rect(rect: Rect, inset: f32) -> Rect {
    Rect::new(
        rect.origin.x + inset,
        rect.origin.y + inset,
        (rect.size.width - inset * 2.0).max(0.0),
        (rect.size.height - inset * 2.0).max(0.0),
    )
}

fn transparent(color: Color) -> Color {
    Color { alpha: 0, ..color }
}

fn rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut bgra = Vec::with_capacity(rgba.len());
    for pixel in rgba.chunks_exact(4) {
        bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
    }
    bgra
}

fn premultiply(channel: u8, alpha: u8) -> u8 {
    ((u16::from(channel) * u16::from(alpha) + 127) / 255) as u8
}

fn ellipse_points(rect: Rect) -> Vec<(f32, f32)> {
    let center_x = rect.origin.x + rect.size.width * 0.5;
    let center_y = rect.origin.y + rect.size.height * 0.5;
    let rx = rect.size.width * 0.5;
    let ry = rect.size.height * 0.5;
    (0..CURVE_SEGMENTS)
        .map(|index| {
            let angle = core::f32::consts::TAU * index as f32 / CURVE_SEGMENTS as f32;
            (center_x + angle.cos() * rx, center_y + angle.sin() * ry)
        })
        .collect()
}

fn rounded_points(rect: Rect, radius: f32) -> Vec<(f32, f32)> {
    let radius = radius
        .min(rect.size.width * 0.5)
        .min(rect.size.height * 0.5)
        .max(0.0);
    if radius <= 0.0 {
        return vec![
            (rect.origin.x, rect.origin.y),
            (rect.origin.x + rect.size.width, rect.origin.y),
            (
                rect.origin.x + rect.size.width,
                rect.origin.y + rect.size.height,
            ),
            (rect.origin.x, rect.origin.y + rect.size.height),
        ];
    }
    let centers = [
        (
            rect.origin.x + rect.size.width - radius,
            rect.origin.y + radius,
            -core::f32::consts::FRAC_PI_2,
        ),
        (
            rect.origin.x + rect.size.width - radius,
            rect.origin.y + rect.size.height - radius,
            0.0,
        ),
        (
            rect.origin.x + radius,
            rect.origin.y + rect.size.height - radius,
            core::f32::consts::FRAC_PI_2,
        ),
        (
            rect.origin.x + radius,
            rect.origin.y + radius,
            core::f32::consts::PI,
        ),
    ];
    let per_corner = CURVE_SEGMENTS / 4;
    let mut points = Vec::with_capacity(CURVE_SEGMENTS);
    for (cx, cy, start) in centers {
        for step in 0..per_corner {
            let angle =
                start + core::f32::consts::FRAC_PI_2 * step as f32 / (per_corner - 1) as f32;
            points.push((cx + angle.cos() * radius, cy + angle.sin() * radius));
        }
    }
    points
}

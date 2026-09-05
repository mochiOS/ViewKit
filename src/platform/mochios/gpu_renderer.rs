use super::renderer::valid_scale_factor;
use super::*;
use crate::gpu_clip::{ClipRegion, ClipShape, ClipVertex, clip_polygon, premultiplied_color};
use cosmic_text::{CacheKey, SwashContent, SwashImage};

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
    atlas: AtlasRect,
}

struct ImageRasterCacheEntry {
    image: ImageData,
    atlas: AtlasRect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct GlyphAtlasKey {
    cache_key: CacheKey,
    color: u32,
}

#[derive(Clone, Copy)]
struct GlyphAtlasEntry {
    atlas: AtlasRect,
    left: i32,
    top: i32,
}

pub(super) struct GpuSceneRenderer {
    vertices: Vec<Vertex>,
    atlas: Vec<u8>,
    atlas_x: u32,
    atlas_y: u32,
    atlas_row_height: u32,
    atlas_dirty_rows: Option<(u32, u32)>,
    clips: Vec<ClipRegion>,
    glyph_atlas_cache: HashMap<GlyphAtlasKey, GlyphAtlasEntry>,
    image_raster_cache: Vec<ImageRasterCacheEntry>,
    svg_raster_cache: Vec<SvgRasterCacheEntry>,
    frame_width: u32,
    frame_height: u32,
    frame_valid: bool,
    atlas_full: bool,
}

impl GpuSceneRenderer {
    pub(super) fn new() -> Self {
        Self {
            vertices: Vec::new(),
            atlas: Vec::new(),
            atlas_x: 1,
            atlas_y: 0,
            atlas_row_height: 1,
            atlas_dirty_rows: None,
            clips: Vec::new(),
            glyph_atlas_cache: HashMap::new(),
            image_raster_cache: Vec::new(),
            svg_raster_cache: Vec::new(),
            frame_width: 0,
            frame_height: 0,
            frame_valid: false,
            atlas_full: false,
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
        let result = self.render_once(
            viewport,
            dirty_bounds,
            display_list,
            font_system,
            swash_cache,
            text_layout_cache,
            transparent_clear,
            output,
        );
        if result.is_err() && self.atlas_full {
            self.clear_atlas_cache();
            return self.render_once(
                viewport,
                dirty_bounds,
                display_list,
                font_system,
                swash_cache,
                text_layout_cache,
                transparent_clear,
                output,
            );
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn render_once(
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
        self.clips.push(ClipRegion {
            bounds: damage,
            shapes: vec![ClipShape::Rect(damage)],
        });
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
                DrawCommand::PushClip { rect } => {
                    self.push_clip(*rect, None);
                }
                DrawCommand::PushRoundedClip { rect, radius } => {
                    self.push_clip(*rect, Some(*radius));
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
        self.atlas_dirty_rows = None;
        self.atlas_full = false;
        if self.atlas.is_empty() {
            let atlas_len = (ATLAS_WIDTH as usize)
                .checked_mul(ATLAS_HEIGHT as usize)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
            self.atlas.resize(atlas_len, 0);
            self.atlas[..4].copy_from_slice(&[255, 255, 255, 255]);
            self.atlas_dirty_rows = Some((0, 1));
        }
        Ok(())
    }

    fn clear_atlas_cache(&mut self) {
        self.atlas.clear();
        self.atlas_x = 1;
        self.atlas_y = 0;
        self.atlas_row_height = 1;
        self.atlas_dirty_rows = None;
        self.glyph_atlas_cache.clear();
        self.image_raster_cache.clear();
        self.svg_raster_cache.clear();
        self.frame_valid = false;
        self.atlas_full = false;
    }

    fn atlas_capacity_error(&mut self) -> MochiOsBackendError {
        self.atlas_full = true;
        MochiOsBackendError::InvalidWindowSize
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
        let row_bytes = ATLAS_WIDTH as usize * 4;
        let (atlas_data_y, atlas_data_height) = self
            .atlas_dirty_rows
            .map(|(start, end)| (start, end.saturating_sub(start)))
            .unwrap_or((0, 0));
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
        Ok(())
    }

    fn current_clip(&self) -> Rect {
        self.clips
            .last()
            .map(|clip| clip.bounds)
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0))
    }

    fn push_clip(&mut self, rect: Rect, radius: Option<f32>) {
        let Some(current) = self.clips.last() else {
            return;
        };
        let bounds = rect
            .intersection(current.bounds)
            .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let mut shapes = current.shapes.clone();
        if let Some(radius) = radius {
            let radius = radius
                .max(0.0)
                .min(rect.size.width.min(rect.size.height) * 0.5);
            shapes.push(ClipShape::Rounded {
                rect,
                radius,
                polygon: rounded_points(rect, radius),
            });
        } else {
            shapes.push(ClipShape::Rect(rect));
        }
        self.clips.push(ClipRegion { bounds, shapes });
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
        if rect.intersection(self.current_clip()).is_none() {
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
        let half = width * 0.5;
        let opaque_inset = half.min(0.5);
        let outer_transparent = rounded_points_at_offset(rect, radius, half + 0.5);
        let outer_opaque = rounded_points_at_offset(rect, radius, half - opaque_inset);
        let inner_opaque = rounded_points_at_offset(rect, radius, -half + opaque_inset);
        let inner_transparent = rounded_points_at_offset(rect, radius, -half - 0.5);
        self.anti_aliased_ring(
            &outer_transparent,
            &outer_opaque,
            &inner_opaque,
            &inner_transparent,
            color,
            viewport,
        );
    }

    fn stroke_ellipse(&mut self, rect: Rect, width: f32, color: Color, viewport: Viewport) {
        if !width.is_finite() || width <= 0.0 {
            return;
        }
        let half = width * 0.5;
        let opaque_inset = half.min(0.5);
        let outer_transparent = ellipse_points(offset_rect(rect, half + 0.5));
        let outer_opaque = ellipse_points(offset_rect(rect, half - opaque_inset));
        let inner_opaque = ellipse_points(offset_rect(rect, -half + opaque_inset));
        let inner_transparent = ellipse_points(offset_rect(rect, -half - 0.5));
        self.anti_aliased_ring(
            &outer_transparent,
            &outer_opaque,
            &inner_opaque,
            &inner_transparent,
            color,
            viewport,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn anti_aliased_ring(
        &mut self,
        outer_transparent: &[(f32, f32)],
        outer_opaque: &[(f32, f32)],
        inner_opaque: &[(f32, f32)],
        inner_transparent: &[(f32, f32)],
        color: Color,
        viewport: Viewport,
    ) {
        let point_count = outer_transparent.len();
        if point_count < 3
            || outer_opaque.len() != point_count
            || inner_opaque.len() != point_count
            || inner_transparent.len() != point_count
        {
            return;
        }
        let uv = self.uv_center(AtlasRect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        });
        let clear = transparent(color);
        for i in 0..point_count {
            let n = (i + 1) % point_count;
            self.push_triangle_colors(
                [outer_transparent[i], outer_opaque[i], outer_opaque[n]],
                [uv; 3],
                [clear, color, color],
                viewport,
            );
            self.push_triangle_colors(
                [outer_transparent[i], outer_opaque[n], outer_transparent[n]],
                [uv; 3],
                [clear, color, clear],
                viewport,
            );
            if outer_opaque != inner_opaque {
                self.push_triangle(
                    [outer_opaque[i], outer_opaque[n], inner_opaque[n]],
                    [uv; 3],
                    color,
                    viewport,
                );
                self.push_triangle(
                    [outer_opaque[i], inner_opaque[n], inner_opaque[i]],
                    [uv; 3],
                    color,
                    viewport,
                );
            }
            self.push_triangle_colors(
                [inner_opaque[i], inner_opaque[n], inner_transparent[n]],
                [uv; 3],
                [color, color, clear],
                viewport,
            );
            self.push_triangle_colors(
                [inner_opaque[i], inner_transparent[n], inner_transparent[i]],
                [uv; 3],
                [color, clear, clear],
                viewport,
            );
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
        let atlas = if let Some(entry) = self
            .image_raster_cache
            .iter()
            .find(|entry| entry.image == command.image)
        {
            entry.atlas
        } else {
            if self.image_raster_cache.len() >= IMAGE_RASTER_CACHE_CAPACITY {
                return Err(self.atlas_capacity_error());
            }
            let pixels = rgba_to_bgra(command.image.premultiplied_rgba8());
            let atlas = self.pack_bgra(command.image.width(), command.image.height(), &pixels)?;
            self.image_raster_cache.push(ImageRasterCacheEntry {
                image: command.image.clone(),
                atlas,
            });
            atlas
        };
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
        let atlas = if let Some(entry) = self.svg_raster_cache.iter().find(|entry| {
            entry.svg == command.svg
                && entry.width == width
                && entry.height == height
                && entry.tint == command.tint
        }) {
            entry.atlas
        } else {
            if self.svg_raster_cache.len() >= SVG_RASTER_CACHE_CAPACITY {
                return Err(self.atlas_capacity_error());
            }
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
            let pixels = rgba_to_bgra(pixmap.data());
            let atlas = self.pack_bgra(width, height, &pixels)?;
            self.svg_raster_cache.push(SvgRasterCacheEntry {
                svg: command.svg.clone(),
                width,
                height,
                tint: command.tint,
                atlas,
            });
            atlas
        };
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
        let create_buffer = |font_system: &mut FontSystem| {
            let font_size = (command.font_size * scale).max(1.0);
            let line_height = (command.line_height * scale).max(font_size);
            let mut buffer = Buffer::new(font_system, Metrics::new(font_size, line_height));
            let mut borrowed = buffer.borrow_with(font_system);
            borrowed.set_size(
                Some(command.bounds.size.width * scale),
                Some(command.bounds.size.height * scale),
            );
            let attrs = Attrs::new()
                .family(resolve_font_family(command.font_family.as_str()))
                .weight(Weight(command.weight.clamp(1, 1000)));
            borrowed.set_text(
                command.text.as_str(),
                &attrs,
                Shaping::Advanced,
                command.alignment.to_cosmic(),
            );
            drop(borrowed);
            buffer
        };
        let key = TextLayoutKey::new(command, scale);
        if !layout_cache.contains_key(&key) {
            if layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
                layout_cache.clear();
            }
            layout_cache.insert(key.clone(), create_buffer(font_system));
        }
        let Some(buffer) = layout_cache.get_mut(&key) else {
            return Ok(());
        };
        if !command.cache_layout {
            let mut borrowed = buffer.borrow_with(font_system);
            borrowed.set_size(
                Some(command.bounds.size.width * scale),
                Some(command.bounds.size.height * scale),
            );
            let attrs = Attrs::new()
                .family(resolve_font_family(command.font_family.as_str()))
                .weight(Weight(command.weight.clamp(1, 1000)));
            borrowed.set_text(
                command.text.as_str(),
                &attrs,
                Shaping::Advanced,
                command.alignment.to_cosmic(),
            );
        }
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
        self.push_clip(text_clip, None);
        for glyph in glyphs {
            let atlas_key = GlyphAtlasKey {
                cache_key: glyph.cache_key,
                color: u32::from_be_bytes([
                    command.color.red,
                    command.color.green,
                    command.color.blue,
                    command.color.alpha,
                ]),
            };
            let entry = if let Some(entry) = self.glyph_atlas_cache.get(&atlas_key) {
                *entry
            } else {
                let Some(image) = swash_cache.get_image(font_system, glyph.cache_key).as_ref()
                else {
                    continue;
                };
                let atlas = self.pack_glyph(image, text_color)?;
                let entry = GlyphAtlasEntry {
                    atlas,
                    left: image.placement.left,
                    top: image.placement.top,
                };
                self.glyph_atlas_cache.insert(atlas_key, entry);
                entry
            };
            let rect = Rect::new(
                (glyph.x + entry.left) as f32,
                (glyph.y - entry.top) as f32,
                entry.atlas.width as f32,
                entry.atlas.height as f32,
            );
            self.textured_quad(rect, entry.atlas, 1.0, viewport);
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
        let mut polygon = positions
            .into_iter()
            .zip(uv)
            .zip(colors)
            .map(|((position, uv), color)| ClipVertex {
                position,
                uv,
                color: premultiplied_color(color),
            })
            .collect::<Vec<_>>();
        let Some(clip) = self.clips.last() else {
            return;
        };
        if !clip
            .shapes
            .iter()
            .all(|shape| polygon.iter().all(|vertex| shape.contains(vertex.position)))
        {
            for shape in &clip.shapes {
                polygon = clip_polygon(polygon, shape.polygon());
                if polygon.len() < 3 {
                    return;
                }
            }
        }
        for index in 1..polygon.len() - 1 {
            self.push_clipped_vertex(polygon[0], viewport);
            self.push_clipped_vertex(polygon[index], viewport);
            self.push_clipped_vertex(polygon[index + 1], viewport);
        }
    }

    fn push_clipped_vertex(&mut self, vertex: ClipVertex, viewport: Viewport) {
        let x = vertex.position.0 / viewport.physical_width as f32 * 2.0 - 1.0;
        let y = vertex.position.1 / viewport.physical_height as f32 * 2.0 - 1.0;
        self.vertices.push(Vertex {
            position: [x, y, 0.0],
            uv: vertex.uv,
            color: vertex.color,
        });
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
            return Err(self.atlas_capacity_error());
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
        self.mark_atlas_dirty(rect.y, rect.height);
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
            return Err(self.atlas_capacity_error());
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
                if image.data.len() < pixel_count.saturating_mul(4) {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                }
                for index in 0..pixel_count {
                    let source = index * 4;
                    let red_alpha = combine_alpha(image.data[source], base_alpha);
                    let green_alpha = combine_alpha(image.data[source + 1], base_alpha);
                    let blue_alpha = combine_alpha(image.data[source + 2], base_alpha);
                    let alpha = red_alpha.max(green_alpha).max(blue_alpha);
                    let target_x = index % width as usize;
                    let target_y = index / width as usize;
                    let target = ((rect.y as usize + target_y) * ATLAS_WIDTH as usize
                        + rect.x as usize
                        + target_x)
                        * 4;
                    self.atlas[target..target + 4].copy_from_slice(&[
                        premultiply(blue, blue_alpha),
                        premultiply(green, green_alpha),
                        premultiply(red, red_alpha),
                        alpha,
                    ]);
                }
            }
        }
        self.atlas_x = self.atlas_x.saturating_add(width + 1);
        self.atlas_row_height = self.atlas_row_height.max(height + 1);
        self.mark_atlas_dirty(rect.y, rect.height);
        Ok(rect)
    }

    fn mark_atlas_dirty(&mut self, y: u32, height: u32) {
        let end = y.saturating_add(height).min(ATLAS_HEIGHT);
        if y >= end {
            return;
        }
        self.atlas_dirty_rows = Some(match self.atlas_dirty_rows {
            Some((start, previous_end)) => (start.min(y), previous_end.max(end)),
            None => (y, end),
        });
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

fn offset_rect(rect: Rect, offset: f32) -> Rect {
    if offset >= 0.0 {
        rect.expanded(offset)
    } else {
        inset_rect(rect, -offset)
    }
}

fn rounded_points_at_offset(rect: Rect, radius: f32, offset: f32) -> Vec<(f32, f32)> {
    rounded_points(offset_rect(rect, offset), (radius + offset).max(0.0))
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

fn combine_alpha(mask: u8, alpha: u8) -> u8 {
    ((u16::from(mask) * u16::from(alpha) + 127) / 255) as u8
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

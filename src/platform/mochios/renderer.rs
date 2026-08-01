use super::buffer::flatten_premultiplied_pixel;
use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(super) struct TextLayoutKey {
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
    pub(super) fn new(command: &TextCommand, scale: f32) -> Self {
        Self {
            text: if command.cache_layout {
                command.text.clone()
            } else {
                String::new()
            },
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

const fn alignment_key(alignment: crate::typography::TextAlignment) -> u8 {
    match alignment {
        crate::typography::TextAlignment::Start => 0,
        crate::typography::TextAlignment::Center => 1,
        crate::typography::TextAlignment::End => 2,
        crate::typography::TextAlignment::Justified => 3,
    }
}

pub(super) fn render_display_list(
    viewport: Viewport,
    dirty_bounds: Rect,
    display_list: &DisplayList,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text_layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    pixmap: &mut Option<Pixmap>,
    clip_masks: &mut Vec<Mask>,
    transparent_clear: bool,
) -> Result<Color, MochiOsBackendError> {
    let width = viewport.physical_width;
    let height = viewport.physical_height;
    let pixmap = reusable_pixmap(pixmap, width, height)?;
    let mut clear_color = Color::BLACK;

    let scale = valid_scale_factor(viewport.scale_factor);
    let transform = Transform::from_scale(scale, scale);
    let bounds = viewport.logical_bounds();
    let dirty_bounds = dirty_bounds.intersection(bounds).unwrap_or(bounds);
    configure_clip_mask(clip_masks, 0, dirty_bounds, None, width, height, transform)?;
    let mut clip_depth = 0usize;
    let mut clip_enabled = vec![true];

    for command in display_list.commands() {
        match command {
            DrawCommand::PushClip { rect } => {
                let next_depth = clip_depth.saturating_add(1);
                let enabled = clip_enabled[clip_depth] && rect.intersection(dirty_bounds).is_some();
                if clip_enabled.len() <= next_depth {
                    clip_enabled.resize(next_depth + 1, false);
                }
                clip_enabled[next_depth] = enabled;
                if enabled {
                    configure_clip_mask(
                        clip_masks,
                        next_depth,
                        *rect,
                        Some(clip_depth),
                        width,
                        height,
                        transform,
                    )?;
                }
                clip_depth = next_depth;
            }
            DrawCommand::PushRoundedClip { rect, radius } => {
                let next_depth = clip_depth.saturating_add(1);
                let enabled = clip_enabled[clip_depth] && rect.intersection(dirty_bounds).is_some();
                if clip_enabled.len() <= next_depth {
                    clip_enabled.resize(next_depth + 1, false);
                }
                clip_enabled[next_depth] = enabled;
                if enabled {
                    configure_rounded_clip_mask(
                        clip_masks,
                        next_depth,
                        *rect,
                        *radius,
                        Some(clip_depth),
                        width,
                        height,
                        transform,
                    )?;
                }
                clip_depth = next_depth;
            }
            DrawCommand::PopClip => {
                if clip_depth > 0 {
                    clip_depth -= 1;
                }
            }
            _ if !clip_enabled[clip_depth] => continue,
            DrawCommand::Clear { color } => {
                let color = if transparent_clear {
                    Color::TRANSPARENT
                } else {
                    *color
                };
                clear_color = color;
                if let Some(rect) = to_skia_rect(dirty_bounds) {
                    let mut paint = solid_paint(color);
                    if transparent_clear {
                        paint.blend_mode = BlendMode::Source;
                    }
                    pixmap.fill_rect(rect, &paint, transform, clip_masks.get(clip_depth));
                }
            }
            DrawCommand::FillRect { rect, color } => {
                if rect.intersection(dirty_bounds).is_none() {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let paint = solid_paint(*color);
                pixmap.fill_rect(rect, &paint, transform, clip_masks.get(clip_depth));
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
                    clip_masks.get(clip_depth),
                );
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
                    clip_masks.get(clip_depth),
                );
            }
            DrawCommand::StrokeRect {
                rect,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                if rect
                    .expanded(*stroke_width * 0.5 + 1.0)
                    .intersection(dirty_bounds)
                    .is_none()
                {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = PathBuilder::from_rect(rect);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    transform,
                    clip_masks.get(clip_depth),
                );
            }
            DrawCommand::StrokeRoundedRect {
                rect,
                radius,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                if rect
                    .expanded(*stroke_width * 0.5 + 1.0)
                    .intersection(dirty_bounds)
                    .is_none()
                {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = rounded_rect_path(rect, *radius);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    transform,
                    clip_masks.get(clip_depth),
                );
            }
            DrawCommand::StrokeEllipse {
                rect,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                if rect
                    .expanded(*stroke_width * 0.5 + 1.0)
                    .intersection(dirty_bounds)
                    .is_none()
                {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = ellipse_path(rect);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    transform,
                    clip_masks.get(clip_depth),
                );
            }
            DrawCommand::DrawText { command } => {
                if command.bounds.intersection(dirty_bounds).is_none() {
                    continue;
                }
                draw_text_command(
                    &mut *pixmap,
                    font_system,
                    swash_cache,
                    text_layout_cache,
                    command,
                    scale,
                    clip_masks.get(clip_depth),
                );
            }
            DrawCommand::DrawSvg { command } => {
                if command.bounds.intersection(dirty_bounds).is_none() {
                    continue;
                }
                draw_svg_command(pixmap, command, scale, clip_masks.get(clip_depth))?;
            }
            DrawCommand::DrawImage { command } => {
                if command.bounds.intersection(dirty_bounds).is_none() {
                    continue;
                }
                draw_image_command(
                    pixmap,
                    command,
                    scale,
                    dirty_bounds,
                    clip_masks.get(clip_depth),
                )?;
            }
        }
    }

    Ok(clear_color)
}

pub fn render_offscreen_xrgb(
    display_list: &DisplayList,
    width: u32,
    height: u32,
) -> Result<Vec<u32>, MochiOsBackendError> {
    let viewport = Viewport::new(Size::new(width as f32, height as f32), width, height, 1.0);
    let mut font_system = create_font_system();
    let mut swash_cache = SwashCache::new();
    let mut text_layout_cache = HashMap::new();
    let mut pixmap = None;
    let mut clip_masks = Vec::new();
    let bounds = viewport.logical_bounds();
    let background = render_display_list(
        viewport,
        bounds,
        display_list,
        &mut font_system,
        &mut swash_cache,
        &mut text_layout_cache,
        &mut pixmap,
        &mut clip_masks,
        false,
    )?;
    let pixmap = pixmap.ok_or(MochiOsBackendError::InvalidWindowSize)?;
    Ok(pixmap
        .data()
        .chunks_exact(4)
        .map(|pixel| flatten_premultiplied_pixel(pixel, background))
        .collect())
}

fn reusable_pixmap(
    pixmap: &mut Option<Pixmap>,
    width: u32,
    height: u32,
) -> Result<&mut Pixmap, MochiOsBackendError> {
    let needs_allocate = pixmap
        .as_ref()
        .is_none_or(|current| current.width() != width || current.height() != height);
    if needs_allocate {
        *pixmap = Some(Pixmap::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?);
    }
    pixmap
        .as_mut()
        .ok_or(MochiOsBackendError::InvalidWindowSize)
}

pub(super) fn valid_scale_factor(scale_factor: f64) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor as f32
    } else {
        1.0
    }
}

fn draw_image_command(
    target: &mut Pixmap,
    command: &ImageCommand,
    display_scale: f32,
    dirty_bounds: Rect,
    clip: Option<&Mask>,
) -> Result<(), MochiOsBackendError> {
    let bounds = command.bounds;
    if !is_valid_image_bounds(bounds) {
        return Ok(());
    }

    let image_width = command.image.width();
    let image_height = command.image.height();
    if image_width == 0 || image_height == 0 {
        return Ok(());
    }

    let destination_width = bounds.size.width * display_scale;
    let destination_height = bounds.size.height * display_scale;
    let translate_x = bounds.origin.x * display_scale;
    let translate_y = bounds.origin.y * display_scale;
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || !translate_x.is_finite()
        || !translate_y.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return Ok(());
    }

    blit_image(
        target,
        command.image.premultiplied_rgba8(),
        image_width as usize,
        image_height as usize,
        translate_x,
        translate_y,
        destination_width,
        destination_height,
        dirty_bounds,
        display_scale,
        sanitize_image_opacity(command.opacity),
        command.sampling,
        clip,
    );
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn blit_image(
    target: &mut Pixmap,
    source: &[u8],
    source_width: usize,
    source_height: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    dirty_bounds: Rect,
    display_scale: f32,
    opacity: f32,
    sampling: ImageSampling,
    clip: Option<&Mask>,
) {
    let target_width = target.width() as usize;
    let target_height = target.height() as usize;
    let damage_left = (dirty_bounds.origin.x * display_scale).floor();
    let damage_top = (dirty_bounds.origin.y * display_scale).floor();
    let damage_right = ((dirty_bounds.origin.x + dirty_bounds.size.width) * display_scale).ceil();
    let damage_bottom = ((dirty_bounds.origin.y + dirty_bounds.size.height) * display_scale).ceil();
    let left = x.floor().max(damage_left).max(0.0) as usize;
    let top = y.floor().max(damage_top).max(0.0) as usize;
    let right = (x + width)
        .ceil()
        .min(damage_right)
        .max(0.0)
        .min(target_width as f32) as usize;
    let bottom = (y + height)
        .ceil()
        .min(damage_bottom)
        .max(0.0)
        .min(target_height as f32) as usize;
    if left >= right || top >= bottom {
        return;
    }

    let opacity = (opacity * 255.0).round().clamp(0.0, 255.0) as u32;
    let clip_data = clip.map(Mask::data);
    let target_data = target.data_mut();
    for target_y in top..bottom {
        let source_y = ((target_y as f32 + 0.5 - y) * source_height as f32 / height - 0.5)
            .clamp(0.0, source_height.saturating_sub(1) as f32);
        for target_x in left..right {
            let source_x = ((target_x as f32 + 0.5 - x) * source_width as f32 / width - 0.5)
                .clamp(0.0, source_width.saturating_sub(1) as f32);
            let pixel = match sampling {
                ImageSampling::Nearest => {
                    sample_nearest(source, source_width, source_height, source_x, source_y)
                }
                ImageSampling::Bilinear | ImageSampling::Bicubic => {
                    sample_bilinear(source, source_width, source_height, source_x, source_y)
                }
            };
            let target_index = (target_y * target_width + target_x) * 4;
            let mask = clip_data
                .and_then(|data| data.get(target_y * target_width + target_x))
                .copied()
                .unwrap_or(255) as u32;
            let factor = (opacity * mask + 127) / 255;
            blend_premultiplied(
                &mut target_data[target_index..target_index + 4],
                pixel,
                factor,
            );
        }
    }
}

fn sample_nearest(source: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
    let x = (x.round() as usize).min(width.saturating_sub(1));
    let y = (y.round() as usize).min(height.saturating_sub(1));
    source_pixel(source, width, x, y)
}

fn sample_bilinear(source: &[u8], width: usize, height: usize, x: f32, y: f32) -> [u8; 4] {
    let x0 = (x.floor() as usize).min(width.saturating_sub(1));
    let y0 = (y.floor() as usize).min(height.saturating_sub(1));
    let x1 = x0.saturating_add(1).min(width.saturating_sub(1));
    let y1 = y0.saturating_add(1).min(height.saturating_sub(1));
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;
    let pixels = [
        source_pixel(source, width, x0, y0),
        source_pixel(source, width, x1, y0),
        source_pixel(source, width, x0, y1),
        source_pixel(source, width, x1, y1),
    ];
    let weights = [
        (1.0 - fx) * (1.0 - fy),
        fx * (1.0 - fy),
        (1.0 - fx) * fy,
        fx * fy,
    ];
    let mut output = [0u8; 4];
    for channel in 0..4 {
        output[channel] = pixels
            .iter()
            .zip(weights)
            .map(|(pixel, weight)| pixel[channel] as f32 * weight)
            .sum::<f32>()
            .round()
            .clamp(0.0, 255.0) as u8;
    }
    output
}

fn source_pixel(source: &[u8], width: usize, x: usize, y: usize) -> [u8; 4] {
    let index = (y * width + x) * 4;
    source
        .get(index..index + 4)
        .and_then(|pixel| pixel.try_into().ok())
        .unwrap_or([0; 4])
}

fn blend_premultiplied(target: &mut [u8], source: [u8; 4], factor: u32) {
    let source_alpha = (source[3] as u32 * factor + 127) / 255;
    let inverse_alpha = 255 - source_alpha;
    for channel in 0..3 {
        let source_channel = (source[channel] as u32 * factor + 127) / 255;
        target[channel] =
            (source_channel + (target[channel] as u32 * inverse_alpha + 127) / 255).min(255) as u8;
    }
    target[3] = (source_alpha + (target[3] as u32 * inverse_alpha + 127) / 255).min(255) as u8;
}

fn draw_svg_command(
    target: &mut Pixmap,
    command: &SvgCommand,
    display_scale: f32,
    clip: Option<&Mask>,
) -> Result<(), MochiOsBackendError> {
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

    let raster_width = destination_width.ceil() as u32;
    let raster_height = destination_height.ceil() as u32;
    if raster_width == 0 || raster_height == 0 {
        return Ok(());
    }

    let mut raster =
        Pixmap::new(raster_width, raster_height).ok_or(MochiOsBackendError::InvalidWindowSize)?;
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

    let paint = PixmapPaint {
        opacity: sanitize_image_opacity(command.opacity),
        quality: FilterQuality::Bicubic,
        ..PixmapPaint::default()
    };
    target.draw_pixmap(
        translate_x.round() as i32,
        translate_y.round() as i32,
        raster.as_ref(),
        &paint,
        Transform::identity(),
        clip,
    );

    Ok(())
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
    let create_buffer = |font_system: &mut FontSystem| {
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
        return;
    };
    if !command.cache_layout {
        let mut buffer = buffer.borrow_with(font_system);
        buffer.set_size(Some(width), Some(height));
        let attrs = Attrs::new()
            .family(Family::Name(command.font_family.as_str()))
            .weight(Weight(command.weight.clamp(1, 1000)));
        buffer.set_text(
            command.text.as_str(),
            &attrs,
            Shaping::Advanced,
            command.alignment.to_cosmic(),
        );
    }
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
                blend_text_pixel(pixmap, clip, text_clip, draw_x, draw_y, color);
            },
        );
    }
}

fn blend_text_pixel(
    pixmap: &mut Pixmap,
    clip: Option<&Mask>,
    text_clip: SkiaRect,
    x: i32,
    y: i32,
    color: CosmicColor,
) {
    let Ok(x) = usize::try_from(x) else {
        return;
    };
    let Ok(y) = usize::try_from(y) else {
        return;
    };
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    if x >= width || y >= height {
        return;
    }

    let pixel_left = x as f32;
    let pixel_top = y as f32;
    let overlap_width =
        (pixel_left + 1.0).min(text_clip.right()) - pixel_left.max(text_clip.left());
    let overlap_height = (pixel_top + 1.0).min(text_clip.bottom()) - pixel_top.max(text_clip.top());
    if overlap_width <= 0.0 || overlap_height <= 0.0 {
        return;
    }

    let Some(pixel_index) = y.checked_mul(width).and_then(|index| index.checked_add(x)) else {
        return;
    };
    let clip_alpha = clip
        .and_then(|mask| mask.data().get(pixel_index))
        .copied()
        .unwrap_or(255);
    let coverage = (overlap_width * overlap_height).clamp(0.0, 1.0);
    let factor = (f32::from(clip_alpha) * coverage).round() as u32;
    if factor == 0 {
        return;
    }

    let (red, green, blue, alpha) = color.as_rgba_tuple();
    if alpha == 0 {
        return;
    }
    let source = [
        multiply_channel(red, alpha),
        multiply_channel(green, alpha),
        multiply_channel(blue, alpha),
        alpha,
    ];
    let Some((byte_index, byte_end)) = pixel_index
        .checked_mul(4)
        .and_then(|index| index.checked_add(4).map(|end| (index, end)))
    else {
        return;
    };
    let Some(pixel) = pixmap.data_mut().get_mut(byte_index..byte_end) else {
        return;
    };
    blend_premultiplied(pixel, source, factor);
}

fn to_skia_rect(rect: Rect) -> Option<SkiaRect> {
    if !rect.origin.x.is_finite()
        || !rect.origin.y.is_finite()
        || !rect.size.width.is_finite()
        || !rect.size.height.is_finite()
        || rect.size.width < 0.0
        || rect.size.height < 0.0
    {
        return None;
    }
    SkiaRect::from_xywh(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    )
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

fn configure_clip_mask(
    masks: &mut Vec<Mask>,
    index: usize,
    rect: Rect,
    previous: Option<usize>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<(), MochiOsBackendError> {
    let path = to_skia_rect(rect).map(PathBuilder::from_rect);
    configure_path_clip_mask(masks, index, path, previous, width, height, transform)
}

fn configure_rounded_clip_mask(
    masks: &mut Vec<Mask>,
    index: usize,
    rect: Rect,
    radius: f32,
    previous: Option<usize>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<(), MochiOsBackendError> {
    let path = to_skia_rect(rect).map(|rect| rounded_rect_path(rect, radius));
    configure_path_clip_mask(masks, index, path, previous, width, height, transform)
}

fn configure_path_clip_mask(
    masks: &mut Vec<Mask>,
    index: usize,
    path: Option<Path>,
    previous: Option<usize>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<(), MochiOsBackendError> {
    while masks.len() <= index {
        masks.push(Mask::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?);
    }
    if masks[index].width() != width || masks[index].height() != height {
        masks[index] = Mask::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?;
    }

    let has_previous = previous.is_some();
    if let Some(previous) = previous {
        if previous >= index {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let (before, current) = masks.split_at_mut(index);
        current[0]
            .data_mut()
            .copy_from_slice(before[previous].data());
    }
    let mask = &mut masks[index];

    let Some(path) = path else {
        mask.clear();
        return Ok(());
    };

    if has_previous {
        mask.intersect_path(&path, FillRule::Winding, true, transform);
    } else {
        mask.clear();
        mask.fill_path(&path, FillRule::Winding, true, transform);
    }

    Ok(())
}

fn solid_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);
    paint.anti_alias = true;
    paint
}

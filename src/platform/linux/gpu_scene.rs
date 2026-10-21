use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use tiny_skia::{Pixmap, Transform};

use crate::draw_command::{DisplayList, DrawCommand, ImageCommand, SvgCommand, TextCommand};
use crate::font::resolve_font_family;
use crate::geometry::Rect;
use crate::image::ImageData;
use crate::renderer::Viewport;
use crate::svg::SvgData;
use crate::theme::Color;

const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;

#[derive(Debug, thiserror::Error)]
pub enum GpuSceneError {
    #[error("GPU scene arithmetic overflow")]
    ArithmeticOverflow,
    #[error("GPU scene exceeded its supported dimensions or atlas capacity")]
    InvalidWindowSize,
}

type MochiOsBackendError = GpuSceneError;

pub(super) mod renderer {
    use super::*;

    #[derive(Clone, Debug, PartialEq, Eq, Hash)]
    pub(crate) struct TextLayoutKey {
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
                alignment: match command.alignment {
                    crate::typography::TextAlignment::Start => 0,
                    crate::typography::TextAlignment::Center => 1,
                    crate::typography::TextAlignment::End => 2,
                    crate::typography::TextAlignment::Justified => 3,
                },
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

    pub(super) fn valid_scale_factor(scale_factor: f64) -> f32 {
        if scale_factor.is_finite() && scale_factor > 0.0 {
            scale_factor.min(f32::MAX as f64) as f32
        } else {
            1.0
        }
    }
}

#[path = "../mochios/gpu_renderer.rs"]
mod implementation;

pub(super) use implementation::GpuSceneRenderer;
pub(super) use renderer::TextLayoutKey;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::ImageSampling;
    use crate::geometry::Size;

    fn encode(scene: &mut GpuSceneRenderer, list: &DisplayList) -> Vec<u8> {
        let viewport = Viewport::new(Size::new(1280.0, 800.0), 1280, 800, 1.0);
        let mut output = Vec::new();
        scene
            .render(
                viewport,
                viewport.logical_bounds(),
                list,
                &mut crate::font::create_font_system(),
                &mut SwashCache::new(),
                &mut HashMap::new(),
                false,
                &mut output,
            )
            .unwrap();
        output
    }

    #[test]
    fn four_k_wallpaper_keeps_resolution_clipping_and_draw_order() {
        let image = ImageData::from_rgba8(3840, 2160, vec![255; 3840 * 2160 * 4]).unwrap();
        let mut list = DisplayList::new();
        list.push(DrawCommand::Clear {
            color: Color::BLACK,
        });
        list.push(DrawCommand::PushRoundedClip {
            rect: Rect::new(20.0, 20.0, 600.0, 400.0),
            radius: 16.0,
        });
        list.push(DrawCommand::DrawImage {
            command: ImageCommand {
                image: image.clone(),
                bounds: Rect::new(0.0, 0.0, 1280.0, 800.0),
                opacity: 0.75,
                sampling: ImageSampling::Bicubic,
            },
        });
        list.push(DrawCommand::PopClip);
        list.push(DrawCommand::FillRect {
            rect: Rect::new(100.0, 100.0, 80.0, 60.0),
            color: Color::WHITE,
        });
        let mut scene = GpuSceneRenderer::new();
        let bytes = encode(&mut scene, &list);
        let decoded = mochios_viewkit_gpu_protocol::decode(&bytes).unwrap();
        assert_eq!(scene.external_images.len(), 1);
        let draw = &scene.external_images[0];
        assert_eq!(draw.image.dimensions(), (3840, 2160));
        assert_eq!(draw.image, image);
        assert!(draw.vertices.start > 0);
        assert!(draw.vertices.end < decoded.vertex_count());
        for vertex in decoded.vertices
            [draw.vertices.start as usize * 36..draw.vertices.end as usize * 36]
            .chunks_exact(36)
        {
            let x = f32::from_le_bytes(vertex[..4].try_into().unwrap());
            let y = f32::from_le_bytes(vertex[4..8].try_into().unwrap());
            // GPU vertices use normalized device coordinates.
            let x = (x + 1.0) * 640.0;
            let y = (y + 1.0) * 400.0;
            assert!((19.99..=620.01).contains(&x));
            assert!((19.99..=420.01).contains(&y));
        }
        encode(&mut scene, &list);
        assert_eq!(
            scene.external_images.len(),
            1,
            "draw ranges reset each frame"
        );
    }

    #[test]
    fn raster_images_do_not_exhaust_the_glyph_atlas() {
        let mut list = DisplayList::new();
        for index in 0..160 {
            list.push(DrawCommand::DrawImage {
                command: ImageCommand {
                    image: ImageData::from_rgba8(64, 64, vec![255; 64 * 64 * 4]).unwrap(),
                    bounds: Rect::new(
                        (index % 16) as f32 * 64.0,
                        (index / 16) as f32 * 64.0,
                        64.0,
                        64.0,
                    ),
                    opacity: 1.0,
                    sampling: ImageSampling::Bilinear,
                },
            });
        }
        let mut scene = GpuSceneRenderer::new();
        let bytes = encode(&mut scene, &list);
        let decoded = mochios_viewkit_gpu_protocol::decode(&bytes).unwrap();
        assert_eq!(scene.external_images.len(), 160);
        assert_eq!(decoded.atlas_data_height, 1);
        assert!(
            scene
                .external_images
                .windows(2)
                .all(|pair| pair[0].vertices.end <= pair[1].vertices.start)
        );
    }
}

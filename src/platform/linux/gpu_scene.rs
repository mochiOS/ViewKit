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

// Host GPUs can keep a larger glyph/vector cache without changing the native
// mochiOS atlas size or its GPU protocol.
pub(super) const ATLAS_WIDTH: u32 = 2048;
pub(super) const ATLAS_HEIGHT: u32 = 2048;

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

    #[test]
    fn hidpi_keeps_shapes_images_and_vectors_in_the_same_logical_space() {
        let image = ImageData::from_rgba8(2, 2, vec![255; 16]).unwrap();
        let svg = SvgData::decode(br#"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="white"/></svg>"#).unwrap();
        let bounds = Rect::new(20.0, 30.0, 60.0, 40.0);
        let mut list = DisplayList::new();
        list.push(DrawCommand::Clear {
            color: Color::BLACK,
        });
        list.push(DrawCommand::FillRect {
            rect: bounds,
            color: Color::WHITE,
        });
        list.push(DrawCommand::DrawImage {
            command: ImageCommand {
                image,
                bounds,
                opacity: 1.0,
                sampling: ImageSampling::Bilinear,
            },
        });
        list.push(DrawCommand::DrawSvg {
            command: SvgCommand {
                svg,
                bounds,
                opacity: 1.0,
                tint: None,
            },
        });
        let mut frames = Vec::new();
        for scale in [1.0, 2.0] {
            let viewport = Viewport::new(
                Size::new(200.0, 150.0),
                (200.0 * scale) as u32,
                (150.0 * scale) as u32,
                scale,
            );
            let mut output = Vec::new();
            GpuSceneRenderer::new()
                .render(
                    viewport,
                    viewport.logical_bounds(),
                    &list,
                    &mut crate::font::create_font_system(),
                    &mut SwashCache::new(),
                    &mut HashMap::new(),
                    false,
                    &mut output,
                )
                .unwrap();
            let decoded = mochios_viewkit_gpu_protocol::decode(&output).unwrap();
            frames.push(
                decoded
                    .vertices
                    .chunks_exact(36)
                    .map(|v| v[..12].to_vec())
                    .collect::<Vec<_>>(),
            );
        }
        assert_eq!(
            frames[0], frames[1],
            "all primitives must occupy identical logical positions at 1x and 2x"
        );
    }

    #[test]
    fn small_damage_culls_unrelated_window_geometry() {
        let viewport = Viewport::new(Size::new(1280.0, 800.0), 1280, 800, 1.0);
        let mut list = DisplayList::new();
        list.push(DrawCommand::Clear {
            color: Color::WHITE,
        });
        for row in 0..6 {
            for col in 0..10 {
                list.push(DrawCommand::FillRoundedRect {
                    rect: Rect::new(
                        col as f32 * 120.0 + 10.0,
                        row as f32 * 120.0 + 10.0,
                        100.0,
                        100.0,
                    ),
                    radius: 20.0,
                    color: Color::BLACK,
                });
            }
        }
        let mut scene = GpuSceneRenderer::new();
        let full = encode(&mut scene, &list);
        let mut partial = Vec::new();
        let damage = Rect::new(0.0, 0.0, 120.0, 120.0);
        scene
            .render(
                viewport,
                damage,
                &list,
                &mut crate::font::create_font_system(),
                &mut SwashCache::new(),
                &mut HashMap::new(),
                false,
                &mut partial,
            )
            .unwrap();
        let full = mochios_viewkit_gpu_protocol::decode(&full).unwrap();
        let partial = mochios_viewkit_gpu_protocol::decode(&partial).unwrap();
        assert!(partial.vertex_count() < full.vertex_count() / 20);
        for vertex in partial.vertices.chunks_exact(36) {
            let x = (f32::from_le_bytes(vertex[..4].try_into().unwrap()) + 1.0) * 640.0;
            let y = (f32::from_le_bytes(vertex[4..8].try_into().unwrap()) + 1.0) * 400.0;
            assert!((-0.01..=120.01).contains(&x) && (-0.01..=120.01).contains(&y));
        }
    }

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
    #[test]
    fn changing_omochi_contours_use_native_triangles_without_allocating_atlas_images() {
        use crate::{
            geometry::Point,
            omochi::{Material, SurfaceSample, button_contour},
        };
        let mut scene = GpuSceneRenderer::new();
        for frame in 0..80 {
            let points = button_contour(
                Rect::new(40.0, 40.0, 96.0, 30.0),
                9.0,
                SurfaceSample {
                    anchor: Point::new(1.0, 1.0),
                    pull: Point::new(frame as f32 / 5.0, 0.0),
                    press_depth: 3.4,
                    next_redraw_at: None,
                },
                Material::BUTTON,
            );
            let mut list = DisplayList::new();
            list.push(DrawCommand::DrawContour {
                points,
                fill: Color::WHITE,
                stroke: Color::BLACK,
                stroke_width: 1.0,
            });
            let bytes = encode(&mut scene, &list);
            let decoded = mochios_viewkit_gpu_protocol::decode(&bytes).unwrap();
            assert!(!decoded.vertices.is_empty());
            assert!(decoded.atlas_data_height <= 1); // Initial white texel only; subsequent frames upload nothing.
            assert!(scene.external_images.is_empty());
            for vertex in decoded.vertices.chunks_exact(36) {
                for value in vertex.chunks_exact(4) {
                    assert!(f32::from_le_bytes(value.try_into().unwrap()).is_finite());
                }
            }
        }
    }
}

use std::collections::HashMap;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use tiny_skia::{Pixmap, Transform};

use crate::draw_command::{
    DisplayList, DrawCommand, ImageCommand, SvgCommand, TextCommand,
};
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
        if value == 0.0 { 0.0_f32.to_bits() } else { value.to_bits() }
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

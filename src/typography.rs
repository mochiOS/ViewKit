//! 文字スタイルを定義

use crate::font::create_font_system;
use crate::geometry::Size;
use crate::theme::{FigmaTokens, TypographyToken};
use cosmic_text::{Align, FontSystem, SwashCache};
use std::collections::HashMap;

const TEXT_MEASUREMENT_CACHE_CAPACITY: usize = 2048;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct TextMeasurementKey {
    pub text: String,
    pub font_family: String,
    pub font_size_bits: u32,
    pub line_height_bits: u32,
    pub maximum_width_bits: Option<u32>,
    pub weight: u16,
    pub alignment: TextAlignment,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub enum TextAlignment {
    #[default]
    Start,
    Center,
    End,
    Justified,
}

impl TextAlignment {
    pub(crate) fn to_cosmic(self) -> Option<Align> {
        match self {
            // Noneは通常の行列配置
            Self::Start => None,

            Self::Center => Some(Align::Center),

            Self::End => Some(Align::End),

            Self::Justified => Some(Align::Justified),
        }
    }
}

pub struct TextMeasurer {
    font_system: Option<FontSystem>,
    font_scale: f32,
    measurements: HashMap<TextMeasurementKey, Size>,
    ink_measurements: HashMap<TextMeasurementKey, Option<(f32, f32)>>,
    ink_cache: SwashCache,
}

impl Default for TextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextMeasurer {
    pub fn new() -> Self {
        Self {
            font_system: None,
            font_scale: 1.0,
            measurements: HashMap::new(),
            ink_measurements: HashMap::new(),
            ink_cache: SwashCache::new(),
        }
    }

    pub(crate) fn font_system_mut(&mut self) -> &mut FontSystem {
        self.font_system.get_or_insert_with(create_font_system)
    }

    /// Returns the active accessibility font scale used by text layout.
    pub fn font_scale(&self) -> f32 {
        self.font_scale
    }

    pub(crate) fn set_font_scale(&mut self, scale: f32) {
        let next = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
        if self.font_scale != next {
            self.font_scale = next;
            self.measurements.clear();
            self.ink_measurements.clear();
        }
    }

    pub(crate) fn cached_measurement(&self, key: &TextMeasurementKey) -> Option<Size> {
        self.measurements.get(key).copied()
    }

    pub(crate) fn cache_measurement(&mut self, key: TextMeasurementKey, size: Size) {
        if self.measurements.len() >= TEXT_MEASUREMENT_CACHE_CAPACITY {
            self.measurements.clear();
        }
        self.measurements.insert(key, size);
    }

    /// Visible glyph extents relative to a text command's origin, without line leading.
    pub(crate) fn vertical_ink_bounds(&mut self, key: TextMeasurementKey) -> Option<(f32, f32)> {
        if let Some(bounds) = self.ink_measurements.get(&key) {
            return *bounds;
        }
        let font_system = self.font_system.get_or_insert_with(create_font_system);
        let mut buffer = cosmic_text::Buffer::new(
            font_system,
            cosmic_text::Metrics::new(
                f32::from_bits(key.font_size_bits),
                f32::from_bits(key.line_height_bits),
            ),
        );
        {
            let mut borrowed = buffer.borrow_with(font_system);
            borrowed.set_size(key.maximum_width_bits.map(f32::from_bits), None);
            let attrs = cosmic_text::Attrs::new()
                .family(crate::font::resolve_font_family(&key.font_family))
                .weight(cosmic_text::Weight(key.weight));
            borrowed.set_text(
                &key.text,
                &attrs,
                cosmic_text::Shaping::Advanced,
                key.alignment.to_cosmic(),
            );
            borrowed.shape_until_scroll(true);
        }
        let mut top = f32::INFINITY;
        let mut bottom = f32::NEG_INFINITY;
        for run in buffer.layout_runs() {
            for glyph in run.glyphs {
                let physical = glyph.physical((0.0, run.line_y.round()), 1.0);
                if let Some(image) = self.ink_cache.get_image(font_system, physical.cache_key) {
                    if image.placement.width == 0 || image.placement.height == 0 {
                        continue;
                    }
                    let y = physical.y as f32 - image.placement.top as f32;
                    top = top.min(y);
                    bottom = bottom.max(y + image.placement.height as f32);
                }
            }
        }
        let bounds = (top.is_finite() && bottom > top).then_some((top, bottom));
        if self.ink_measurements.len() >= TEXT_MEASUREMENT_CACHE_CAPACITY {
            self.ink_measurements.clear();
        }
        self.ink_measurements.insert(key, bounds);
        bounds
    }

    #[cfg(test)]
    pub(crate) fn measurement_cache_len(&self) -> usize {
        self.measurements.len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFamily {
    Sans,
    Monospace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FontWeight(pub u16);

impl FontWeight {
    pub const THIN: Self = Self(100);
    pub const EXTRA_LIGHT: Self = Self(200);
    pub const LIGHT: Self = Self(300);
    pub const REGULAR: Self = Self(400);
    pub const MEDIUM: Self = Self(500);
    pub const SEMIBOLD: Self = Self(600);
    pub const BOLD: Self = Self(700);
    pub const EXTRA_BOLD: Self = Self(800);
    pub const BLACK: Self = Self(900);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub family: FontFamily,
    pub size: f32,
    pub weight: FontWeight,
    pub line_height: f32,
    pub letter_spacing: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextRole {
    DisplayLarge,
    DisplayMedium,
    TitleLarge,
    TitleMedium,
    TitleSmall,

    #[default]
    Body,

    Label,
    Caption,
    Code,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub display_large: TextStyle,
    pub display_medium: TextStyle,
    pub title_large: TextStyle,
    pub title_medium: TextStyle,
    pub title_small: TextStyle,
    pub body: TextStyle,
    pub label: TextStyle,
    pub caption: TextStyle,
    pub code: TextStyle,
}

impl Typography {
    pub const DEFAULT: Self = Self {
        display_large: text_style(FigmaTokens::TYPOGRAPHY.display_large, FontFamily::Sans),
        display_medium: text_style(FigmaTokens::TYPOGRAPHY.display_medium, FontFamily::Sans),
        title_large: text_style(FigmaTokens::TYPOGRAPHY.title_large, FontFamily::Sans),
        title_medium: text_style(FigmaTokens::TYPOGRAPHY.title_medium, FontFamily::Sans),
        title_small: text_style(FigmaTokens::TYPOGRAPHY.title_small, FontFamily::Sans),
        body: text_style(FigmaTokens::TYPOGRAPHY.body, FontFamily::Sans),
        label: text_style(FigmaTokens::TYPOGRAPHY.label, FontFamily::Sans),
        caption: text_style(FigmaTokens::TYPOGRAPHY.caption, FontFamily::Sans),
        code: text_style(FigmaTokens::TYPOGRAPHY.body, FontFamily::Monospace),
    };

    pub const fn style(self, role: TextRole) -> TextStyle {
        match role {
            TextRole::DisplayLarge => self.display_large,
            TextRole::DisplayMedium => self.display_medium,
            TextRole::TitleLarge => self.title_large,
            TextRole::TitleMedium => self.title_medium,
            TextRole::TitleSmall => self.title_small,
            TextRole::Body => self.body,
            TextRole::Label => self.label,
            TextRole::Caption => self.caption,
            TextRole::Code => self.code,
        }
    }
}

const fn text_style(token: TypographyToken, family: FontFamily) -> TextStyle {
    TextStyle {
        family,
        size: token.font_size,
        weight: FontWeight(token.font_weight),
        line_height: token.line_height,
        letter_spacing: 0.0,
    }
}

//! 文字スタイルを定義

use crate::font::create_font_system;
use crate::theme::{FigmaTokens, TypographyToken};
use cosmic_text::{Align, FontSystem};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
        self.font_scale = if scale.is_finite() && scale > 0.0 {
            scale
        } else {
            1.0
        };
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
        code: TextStyle {
            family: FontFamily::Monospace,
            size: 14.0,
            weight: FontWeight::REGULAR,
            line_height: 20.0,
            letter_spacing: 0.0,
        },
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

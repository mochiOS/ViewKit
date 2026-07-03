//! 文字スタイルを定義

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
    font_system: FontSystem,
}

impl Default for TextMeasurer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextMeasurer {
    pub fn new() -> Self {
        Self {
            font_system: FontSystem::new(),
        }
    }

    pub(crate) fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Typography {
    pub large_title: TextStyle,
    pub title: TextStyle,
    pub headline: TextStyle,
    pub body: TextStyle,
    pub label: TextStyle,
    pub caption: TextStyle,
    pub code: TextStyle,
}

impl Typography {
    pub const DEFAULT: Self = Self {
        large_title: TextStyle {
            family: FontFamily::Sans,
            size: 32.0,
            weight: FontWeight::BOLD,
            line_height: 40.0,
            letter_spacing: 0.0,
        },
        title: TextStyle {
            family: FontFamily::Sans,
            size: 24.0,
            weight: FontWeight::SEMIBOLD,
            line_height: 32.0,
            letter_spacing: 0.0,
        },
        headline: TextStyle {
            family: FontFamily::Sans,
            size: 17.0,
            weight: FontWeight::SEMIBOLD,
            line_height: 24.0,
            letter_spacing: 0.0,
        },
        body: TextStyle {
            family: FontFamily::Sans,
            size: 16.0,
            weight: FontWeight::REGULAR,
            line_height: 24.0,
            letter_spacing: 0.0,
        },
        label: TextStyle {
            family: FontFamily::Sans,
            size: 14.0,
            weight: FontWeight::MEDIUM,
            line_height: 20.0,
            letter_spacing: 0.0,
        },
        caption: TextStyle {
            family: FontFamily::Sans,
            size: 12.0,
            weight: FontWeight::REGULAR,
            line_height: 16.0,
            letter_spacing: 0.0,
        },
        code: TextStyle {
            family: FontFamily::Monospace,
            size: 14.0,
            weight: FontWeight::REGULAR,
            line_height: 20.0,
            letter_spacing: 0.0,
        },
    };
}

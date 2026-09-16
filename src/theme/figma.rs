//! Typed design tokens generated from the Figma exports in `resources/var`.
//!
//! The JSON exports are the source of truth. `build.rs` validates their shape
//! and generates the constants included at the end of this module.

use super::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AccentColorTokens {
    pub primary: Color,
    pub hover: Color,
    pub pressed: Color,
    pub subtle: Color,
    pub on_accent: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackgroundColorTokens {
    pub primary: Color,
    pub secondary: Color,
    pub elevated: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextColorTokens {
    pub primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub disabled: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderColorTokens {
    pub default: Color,
    pub subtle: Color,
    pub strong: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ErrorColorTokens {
    pub base: Color,
    pub hover: Color,
    pub pressed: Color,
    pub disabled: Color,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticColorTokens {
    pub warning: Color,
    pub success: Color,
    pub information: Color,
    pub error: ErrorColorTokens,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigmaColorTokens {
    pub accent: AccentColorTokens,
    pub background: BackgroundColorTokens,
    pub text: TextColorTokens,
    pub border: BorderColorTokens,
    pub semantic: SemanticColorTokens,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigmaSpacingTokens {
    pub space_2: f32,
    pub space_4: f32,
    pub space_8: f32,
    pub space_12: f32,
    pub space_16: f32,
    pub space_24: f32,
    pub space_32: f32,
    pub space_40: f32,
    pub space_48: f32,
    pub space_64: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigmaShapeTokens {
    pub radius_small: f32,
    pub radius_medium: f32,
    pub radius_large: f32,
    pub radius_xlarge: f32,
    pub stroke_default: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypographyToken {
    pub font_size: f32,
    pub font_weight: u16,
    pub line_height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigmaTypographyTokens {
    pub display_large: TypographyToken,
    pub display_medium: TypographyToken,
    pub title_large: TypographyToken,
    pub title_medium: TypographyToken,
    pub title_small: TypographyToken,
    pub body: TypographyToken,
    pub label: TypographyToken,
    pub caption: TypographyToken,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FigmaLayoutTokens {
    pub toolbar_height: f32,
    pub sidebar_width: f32,
    pub page_margin: f32,
    pub content_max_width: f32,
    pub form_width: f32,
    pub section_gap: f32,
}

/// Namespace for the validated values generated from Figma exports.
pub struct FigmaTokens;

impl FigmaTokens {
    pub const LIGHT: FigmaColorTokens = FIGMA_LIGHT_COLORS;
    pub const DARK: FigmaColorTokens = FIGMA_DARK_COLORS;
    pub const SPACING: FigmaSpacingTokens = FIGMA_SPACING;
    pub const SHAPE: FigmaShapeTokens = FIGMA_SHAPE;
    pub const TYPOGRAPHY: FigmaTypographyTokens = FIGMA_TYPOGRAPHY;
    pub const LAYOUT: FigmaLayoutTokens = FIGMA_LAYOUT;
}

include!(concat!(env!("OUT_DIR"), "/figma_tokens.rs"));

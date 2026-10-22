use super::{CornerRadius, FigmaTokens};

/// omochiの独立した操作面（ボタン・入力欄）の寸法。
/// 初期の調整値です。アプリが明示した寸法やシステム装飾の寸法は置き換えません。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlMetrics {
    pub small_height: f32,
    pub height: f32,
    pub large_height: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub button_min_width: f32,
}

impl ControlMetrics {
    pub const HORIZONTAL_PADDING: f32 = FigmaTokens::SPACING.space_12;
    pub const RADIUS: CornerRadius = CornerRadius::Custom(9.0);
    pub const DEFAULT: Self = Self {
        small_height: FigmaTokens::SPACING.space_24,
        height: 30.0, // omochi/playground/button: actual surface height, excluding SVG margins.
        large_height: FigmaTokens::SPACING.space_32,
        font_size: 13.0,
        line_height: 18.0,
        button_min_width: FigmaTokens::SPACING.space_64,
    };
}

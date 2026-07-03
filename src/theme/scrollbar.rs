use super::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollBarTokens {
    pub thickness: f32,
    pub minimum_thumb_length: f32,
    pub inset: f32,
    pub length_inset: f32,
    pub horizontal_offset: f32,
    pub track_color: Color,
    pub thumb_color: Color,
}

impl ScrollBarTokens {
    pub const DEFAULT: Self = Self {
        thickness: 6.0,
        minimum_thumb_length: 28.0,
        inset: 4.0,
        length_inset: 1.0,
        horizontal_offset: 2.0,
        track_color: Color::rgba(0, 0, 0, 20),
        thumb_color: Color::rgba(0, 0, 0, 96),
    };
}

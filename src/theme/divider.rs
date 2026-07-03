//! 区切り線のデザインを定義

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DividerTokens {
    pub thickness: f32,
}

impl DividerTokens {
    pub const DEFAULT: Self = Self { thickness: 1.0 };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum DividerThickness {
    #[default]
    Standard,

    Custom(f32),
}

impl DividerThickness {
    pub fn resolve(self, tokens: &DividerTokens) -> f32 {
        match self {
            Self::Standard => tokens.thickness,

            Self::Custom(value) => value.max(0.0),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EdgeInsets {
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
    pub left: f32,
}

impl EdgeInsets {
    pub const ZERO: Self = Self {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 0.0,
    };

    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top: finite_non_negative(top),
            right: finite_non_negative(right),
            bottom: finite_non_negative(bottom),
            left: finite_non_negative(left),
        }
    }

    pub fn all(value: f32) -> Self {
        let value = finite_non_negative(value);

        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        let horizontal = finite_non_negative(horizontal);

        let vertical = finite_non_negative(vertical);

        Self {
            top: vertical,
            right: horizontal,
            bottom: vertical,
            left: horizontal,
        }
    }

    pub fn horizontal(self) -> f32 {
        finite_non_negative(self.left) + finite_non_negative(self.right)
    }

    pub fn vertical(self) -> f32 {
        finite_non_negative(self.top) + finite_non_negative(self.bottom)
    }

    pub fn sanitized(self) -> Self {
        Self::new(self.top, self.right, self.bottom, self.left)
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

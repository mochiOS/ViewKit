use crate::geometry::{Rect, Size};
use crate::theme::Color;
use crate::typography::TextAlignment;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Ellipse, EllipseColor, Text};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AvatarSize {
    Small,

    #[default]
    Medium,
}

impl AvatarSize {
    const fn length(self) -> f32 {
        match self {
            Self::Small => 24.0,
            Self::Medium => 32.0,
        }
    }

    const fn font_size(self) -> f32 {
        match self {
            Self::Small => 12.0,
            Self::Medium => 13.0,
        }
    }

    const fn line_height(self) -> f32 {
        match self {
            Self::Small => 16.0,
            Self::Medium => 18.0,
        }
    }

    const fn weight(self) -> u16 {
        match self {
            Self::Small => 400,
            Self::Medium => 500,
        }
    }
}

pub struct Avatar {
    initials: String,
    size: AvatarSize,
    background: Option<Color>,
    foreground: Option<Color>,
}

impl Avatar {
    pub fn new(initials: impl Into<String>) -> Self {
        Self {
            initials: initials.into(),
            size: AvatarSize::Medium,
            background: None,
            foreground: None,
        }
    }

    pub fn size(mut self, size: AvatarSize) -> Self {
        self.size = size;
        self
    }

    pub fn background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub fn foreground(mut self, color: Color) -> Self {
        self.foreground = Some(color);
        self
    }
}

impl View for Avatar {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        let length = self.size.length();
        constraints.constrain(Size::new(length, length))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        Ellipse::new()
            .color(EllipseColor::Custom(
                self.background
                    .unwrap_or(context.theme.colors.surface_subtle),
            ))
            .paint(bounds, context);

        let line_height = self.size.line_height().min(bounds.size.height);
        let text_bounds = Rect::new(
            bounds.origin.x,
            bounds.origin.y + (bounds.size.height - line_height) / 2.0,
            bounds.size.width,
            line_height,
        );

        Text::new(self.initials.clone())
            .font_size(self.size.font_size())
            .line_height(self.size.line_height())
            .weight(self.size.weight())
            .alignment(TextAlignment::Center)
            .color(
                self.foreground
                    .unwrap_or(context.theme.colors.text_secondary),
            )
            .paint(text_bounds, context);
    }
}

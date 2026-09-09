use crate::geometry::{Rect, Size};
use crate::theme::{Color, CornerRadius};
use crate::typography::TextAlignment;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor, Text};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BadgeTone {
    #[default]
    Neutral,
    Accent,
    Success,
    Warning,
    Error,
}

pub struct Badge {
    label: String,
    tone: BadgeTone,
}

impl Badge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            tone: BadgeTone::Neutral,
        }
    }

    pub fn tone(mut self, tone: BadgeTone) -> Self {
        self.tone = tone;
        self
    }

    fn colors(&self, context: &PaintContext<'_>) -> (Color, Color) {
        match self.tone {
            BadgeTone::Neutral => (
                context.theme.colors.surface_subtle,
                context.theme.colors.text_secondary,
            ),
            BadgeTone::Accent => (
                context.theme.colors.accent_soft,
                context.theme.colors.accent,
            ),
            BadgeTone::Success => (context.theme.colors.success, Color::WHITE),
            BadgeTone::Warning => (context.theme.colors.warning, Color::WHITE),
            BadgeTone::Error => (context.theme.colors.destructive, Color::WHITE),
        }
    }
}

impl View for Badge {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let measured = Text::new(self.label.as_str())
            .font_size(12.0)
            .line_height(16.0)
            .measure_unbounded(context.text_measurer);

        constraints.constrain(Size::new((measured.width + 16.0).max(20.0), 20.0))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let (background, foreground) = self.colors(context);

        Rectangle::new()
            .color(RectangleColor::Custom(background))
            .radius(CornerRadius::Small)
            .paint(bounds, context);

        Text::new(self.label.clone())
            .font_size(12.0)
            .line_height(16.0)
            .alignment(TextAlignment::Center)
            .color(foreground)
            .paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + 2.0,
                    bounds.size.width,
                    16.0,
                ),
                context,
            );
    }
}

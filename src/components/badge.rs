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
        let measured = Text::caption(self.label.as_str())
            .measure_unbounded_with_typography(context.text_measurer, context.typography);

        let horizontal = context.theme.spacing.small * 2.0;
        let height = context.typography.caption.line_height + context.theme.spacing.micro * 2.0;
        constraints.constrain(Size::new((measured.width + horizontal).max(height), height))
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

        let line_height = context.typography.caption.line_height;
        Text::caption(self.label.clone())
            .alignment(TextAlignment::Center)
            .color(foreground)
            .paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + context.theme.spacing.micro,
                    bounds.size.width,
                    line_height,
                ),
                context,
            );
    }
}

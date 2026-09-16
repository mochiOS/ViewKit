use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::geometry::{Rect, Size};
use crate::theme::Color;
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
        let recipe = context.theme.badge;
        match self.tone {
            BadgeTone::Neutral => (recipe.neutral_background, recipe.neutral_foreground),
            BadgeTone::Accent => (recipe.accent_background, recipe.accent_foreground),
            BadgeTone::Success => (recipe.success_background, recipe.semantic_foreground),
            BadgeTone::Warning => (recipe.warning_background, recipe.semantic_foreground),
            BadgeTone::Error => (recipe.error_background, recipe.semantic_foreground),
        }
    }
}

impl View for Badge {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let measured = Text::caption(self.label.as_str())
            .measure_unbounded_with_typography(context.text_measurer, context.typography);

        let horizontal = context.theme.badge.horizontal_padding * 2.0;
        let height =
            context.typography.caption.line_height + context.theme.badge.vertical_padding * 2.0;
        constraints.constrain(Size::new((measured.width + horizontal).max(height), height))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        let (background, foreground) = self.colors(context);
        let mut node = AccessibilityNode::new(AccessibilityRole::Status, bounds);
        node.label = Some(self.label.clone());
        context.record_accessibility(node);

        Rectangle::new()
            .color(RectangleColor::Custom(background))
            .radius(context.theme.badge.radius)
            .paint(bounds, context);

        let line_height = context.typography.caption.line_height;
        Text::caption(self.label.clone())
            .accessibility_hidden(true)
            .alignment(TextAlignment::Center)
            .color(foreground)
            .paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y + context.theme.badge.vertical_padding,
                    bounds.size.width,
                    line_height,
                ),
                context,
            );
    }
}

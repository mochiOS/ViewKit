use crate::geometry::{Rect, Size};
use crate::theme::{Color, CornerRadius};
use crate::typography::TextAlignment;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Padding, Rectangle, RectangleColor, Text};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MessageDirection {
    #[default]
    Received,
    Sent,
}

pub struct MessageBubble {
    text: String,
    direction: MessageDirection,
    maximum_width: f32,
}

impl MessageBubble {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            direction: MessageDirection::Received,
            maximum_width: 360.0,
        }
    }

    pub fn direction(mut self, direction: MessageDirection) -> Self {
        self.direction = direction;
        self
    }

    pub fn maximum_width(mut self, width: f32) -> Self {
        if width.is_finite() && width > 0.0 {
            self.maximum_width = width;
        }
        self
    }

    fn foreground(&self, context: &PaintContext<'_>) -> Color {
        match self.direction {
            MessageDirection::Received => context.theme.colors.text_primary,
            MessageDirection::Sent => Color::WHITE,
        }
    }

    fn background(&self, context: &PaintContext<'_>) -> Color {
        match self.direction {
            MessageDirection::Received => context.theme.colors.surface_subtle,
            MessageDirection::Sent => context.theme.colors.accent,
        }
    }

    fn text_view(&self, color: Color) -> Text {
        Text::new(self.text.clone())
            .font_size(15.0)
            .line_height(22.0)
            .alignment(TextAlignment::Start)
            .color(color)
    }
}

impl View for MessageBubble {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let max_text_width = (self.maximum_width - 24.0).max(0.0);
        let measured = self
            .text_view(context.theme.colors.text_primary)
            .measure_text(context.text_measurer, Some(max_text_width));

        constraints.constrain(Size::new(measured.width + 24.0, measured.height + 16.0))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        Rectangle::new()
            .color(RectangleColor::Custom(self.background(context)))
            .radius(CornerRadius::ExtraLarge)
            .paint(bounds, context);

        Padding::only(8.0, 12.0, 8.0, 12.0)
            .content(self.text_view(self.foreground(context)))
            .paint(bounds, context);
    }
}

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::geometry::{Rect, Size};
use crate::theme::Color;
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
            maximum_width: crate::theme::LayoutTokens::DEFAULT.message_max_width,
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
            MessageDirection::Received => context.theme.message_bubble.received_foreground,
            MessageDirection::Sent => context.theme.message_bubble.sent_foreground,
        }
    }

    fn background(&self, context: &PaintContext<'_>) -> Color {
        match self.direction {
            MessageDirection::Received => context.theme.message_bubble.received_background,
            MessageDirection::Sent => context.theme.message_bubble.sent_background,
        }
    }

    fn text_view(&self, color: Color) -> Text {
        Text::new(self.text.clone())
            .accessibility_hidden(true)
            .alignment(TextAlignment::Start)
            .color(color)
    }
}

impl View for MessageBubble {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let horizontal = context.theme.message_bubble.horizontal_padding;
        let vertical = context.theme.message_bubble.vertical_padding;
        let max_text_width = (self.maximum_width - horizontal * 2.0).max(0.0);
        let measured = self
            .text_view(context.theme.colors.text_primary)
            .measure_text_with_typography(
                context.text_measurer,
                context.typography,
                Some(max_text_width),
            );

        constraints.constrain(Size::new(
            measured.width + horizontal * 2.0,
            measured.height + vertical * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        Rectangle::new()
            .color(RectangleColor::Custom(self.background(context)))
            .radius(context.theme.message_bubble.radius)
            .paint(bounds, context);

        let mut node = AccessibilityNode::new(AccessibilityRole::StaticText, bounds);
        node.label = Some(self.text.clone());
        context.record_accessibility(node);

        Padding::symmetric(
            context.theme.message_bubble.horizontal_padding,
            context.theme.message_bubble.vertical_padding,
        )
        .content(self.text_view(self.foreground(context)))
        .paint(bounds, context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    fn measure(text: &str) -> Size {
        let mut text_measurer = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &Theme::DEFAULT,
            typography: &Typography::DEFAULT,
            text_measurer: &mut text_measurer,
        };
        MessageBubble::new(text).measure(
            Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
            &mut context,
        )
    }

    #[test]
    fn chat_reference_bubbles_use_intrinsic_figma_dimensions() {
        let measured = [
            measure("Hey! Are we still on for this afternoon?"),
            measure(
                "We've encountered a problem with the project\nand would like to discuss it with someone.",
            ),
            measure("Yes — 3:00 works for me."),
            measure("Let’s meet in the studio.\nI’ll have the notes ready."),
            measure("Perfect. See you then!"),
        ];
        let expected = [
            Size::new(295.0, 38.0),
            Size::new(351.0, 60.0),
            Size::new(206.0, 38.0),
            Size::new(196.0, 60.0),
            Size::new(182.0, 38.0),
        ];

        assert_eq!(measured, expected);
    }
}

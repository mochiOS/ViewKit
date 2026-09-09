use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::theme::CornerRadius;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{BorderStyle, Rectangle, RectangleColor, Text};

pub struct Tooltip {
    label: String,
}

impl Tooltip {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

impl View for Tooltip {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let horizontal = context.theme.spacing.small + context.theme.divider.thickness;
        let vertical = context.theme.spacing.micro;
        let label = Text::caption(self.label.as_str()).measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - horizontal * 2.0).max(0.0),
                f32::INFINITY,
            )),
            context,
        );
        constraints.constrain(Size::new(
            label.width + horizontal * 2.0,
            (label.height + vertical * 2.0).max(context.theme.layout.compact_control_height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        Rectangle::new()
            .color(RectangleColor::Surface)
            .radius(CornerRadius::Small)
            .border(BorderStyle::standard(1.0))
            .paint(bounds, context);

        let horizontal = context.theme.spacing.small + context.theme.divider.thickness;
        let line_height = context.typography.caption.line_height;
        let vertical = (bounds.size.height - line_height).max(0.0) / 2.0;
        Text::caption(self.label.as_str()).paint(
            Rect::new(
                bounds.origin.x + horizontal,
                bounds.origin.y + vertical,
                (bounds.size.width - horizontal * 2.0).max(0.0),
                line_height.min(bounds.size.height),
            ),
            context,
        );
    }

    fn handle_event(
        &self,
        _bounds: Rect,
        _event: &ViewEvent,
        _context: &mut EventContext<'_>,
    ) -> EventResult {
        EventResult::Ignored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn default_tooltip_matches_figma_size() {
        let mut text_measurer = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &Theme::LIGHT,
            typography: &Typography::DEFAULT,
            text_measurer: &mut text_measurer,
        };

        assert_eq!(
            Tooltip::new("Tooltip").measure(
                Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
                &mut context,
            ),
            Size::new(57.0, 24.0),
        );
    }
}

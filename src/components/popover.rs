use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::theme::CornerRadius;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::background::EmptyView;
use super::{BorderStyle, Rectangle, RectangleColor};

pub struct Popover<Content = EmptyView> {
    content: Content,
}

impl Popover<EmptyView> {
    pub const fn new() -> Self {
        Self { content: EmptyView }
    }
}

impl Default for Popover<EmptyView> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Content> Popover<Content> {
    pub fn content<NewContent: View>(self, content: NewContent) -> Popover<NewContent> {
        Popover { content }
    }
}

impl<Content: View> View for Popover<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let inset = context.theme.spacing.small;
        let child = self.content.measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - inset * 2.0).max(0.0),
                (constraints.maximum.height - inset * 2.0).max(0.0),
            )),
            context,
        );
        constraints.constrain(Size::new(
            (child.width + inset * 2.0).max(context.theme.layout.popover_width),
            (child.height + inset * 2.0).max(context.theme.layout.popover_height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        Rectangle::new()
            .color(RectangleColor::Surface)
            .radius(CornerRadius::Medium)
            .border(BorderStyle::strong(1.0))
            .paint(bounds, context);
        context
            .display_list
            .push(DrawCommand::PushClip { rect: bounds });
        self.content
            .paint(inset_rect(bounds, context.theme.spacing.small), context);
        context.display_list.push(DrawCommand::PopClip);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.content.handle_event(
            inset_rect(bounds, context.theme.spacing.small),
            event,
            context,
        )
    }
}

fn inset_rect(bounds: Rect, inset: f32) -> Rect {
    Rect::new(
        bounds.origin.x + inset,
        bounds.origin.y + inset,
        (bounds.size.width - inset * 2.0).max(0.0),
        (bounds.size.height - inset * 2.0).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn empty_popover_uses_foundation_size() {
        let mut text_measurer = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &Theme::LIGHT,
            typography: &Typography::DEFAULT,
            text_measurer: &mut text_measurer,
        };
        assert_eq!(
            Popover::new().measure(
                Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
                &mut context,
            ),
            Size::new(240.0, 120.0),
        );
    }
}

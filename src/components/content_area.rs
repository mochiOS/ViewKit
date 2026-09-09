use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

pub struct ContentArea<Content> {
    content: Content,
}

impl<Content> ContentArea<Content> {
    pub fn new(content: Content) -> Self {
        Self { content }
    }

    fn content_bounds(bounds: Rect, inset: f32) -> Rect {
        Rect::new(
            bounds.origin.x + inset,
            bounds.origin.y + inset,
            (bounds.size.width - inset * 2.0).max(0.0),
            (bounds.size.height - inset * 2.0).max(0.0),
        )
    }
}

impl<Content: View> View for ContentArea<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let inset = context.theme.spacing.extra_large;
        let child = self.content.measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - inset * 2.0).max(0.0),
                (constraints.maximum.height - inset * 2.0).max(0.0),
            )),
            context,
        );
        constraints.constrain(Size::new(
            child.width + inset * 2.0,
            child.height + inset * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        Rectangle::new()
            .color(RectangleColor::Surface)
            .paint(bounds, context);
        self.content.paint(
            Self::content_bounds(bounds, context.theme.spacing.extra_large),
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        self.content.handle_event(
            Self::content_bounds(bounds, context.theme.spacing.extra_large),
            event,
            context,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};
    use std::cell::Cell;
    use std::rc::Rc;

    struct Recorder(Rc<Cell<Option<Rect>>>);

    impl View for Recorder {
        fn paint(&self, bounds: Rect, _context: &mut PaintContext<'_>) {
            self.0.set(Some(bounds));
        }
    }

    #[test]
    fn content_insets_match_the_chat_screen() {
        let recorded = Rc::new(Cell::new(None));
        let view = ContentArea::new(Recorder(recorded.clone()));
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        view.paint(Rect::new(0.0, 0.0, 859.0, 640.0), &mut context);

        assert_eq!(recorded.get(), Some(Rect::new(24.0, 24.0, 811.0, 592.0)));
    }
}

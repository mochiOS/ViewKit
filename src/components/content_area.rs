use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

pub struct ContentArea<Content> {
    content: Content,
    maximum_width: Option<f32>,
    pane: bool,
}

impl<Content> ContentArea<Content> {
    pub fn new(content: Content) -> Self {
        Self {
            content,
            maximum_width: None,
            pane: false,
        }
    }

    /// A full-width native detail pane with the standard chrome inset.
    /// Unlike a centered document/form page, it has no implicit width cap.
    pub fn pane(content: Content) -> Self {
        Self {
            content,
            maximum_width: None,
            pane: true,
        }
    }

    fn margin(&self, theme: &crate::theme::Theme) -> f32 {
        if self.pane {
            theme.spacing.extra_large
        } else {
            theme.layout.page_margin
        }
    }

    pub fn maximum_width(mut self, width: f32) -> Self {
        self.maximum_width = Some(width.max(0.0));
        self
    }

    fn resolved_maximum_width(&self, default: f32) -> f32 {
        self.maximum_width
            .unwrap_or(if self.pane { f32::INFINITY } else { default })
    }

    fn content_bounds(bounds: Rect, margin: f32, maximum_width: f32) -> Rect {
        let vertical = margin.min(bounds.size.height.max(0.0) * 0.5);
        let available_width = (bounds.size.width - margin * 2.0).max(0.0);
        let content_width = available_width.min(maximum_width);
        let horizontal_offset = (bounds.size.width - content_width).max(0.0) / 2.0;
        Rect::new(
            bounds.origin.x + horizontal_offset,
            bounds.origin.y + vertical,
            content_width,
            (bounds.size.height - vertical * 2.0).max(0.0),
        )
    }
}

impl<Content: View> View for ContentArea<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let margin = self.margin(context.theme);
        let child = self.content.measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - margin * 2.0)
                    .max(0.0)
                    .min(self.resolved_maximum_width(context.theme.layout.content_max_width)),
                (constraints.maximum.height - margin * 2.0).max(0.0),
            )),
            context,
        );
        constraints.constrain(Size::new(
            child.width + margin * 2.0,
            child.height + margin * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        Rectangle::new()
            .color(RectangleColor::Surface)
            .paint(bounds, context);
        self.content.paint(
            Self::content_bounds(
                bounds,
                self.margin(context.theme),
                self.resolved_maximum_width(context.theme.layout.content_max_width),
            ),
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
            Self::content_bounds(
                bounds,
                self.margin(context.theme),
                self.resolved_maximum_width(context.theme.layout.content_max_width),
            ),
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
    fn content_uses_standard_page_insets() {
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

        assert_eq!(recorded.get(), Some(Rect::new(69.5, 40.0, 720.0, 560.0)));
    }

    #[test]
    fn maximum_width_keeps_forms_compact_without_changing_page_margins() {
        let recorded = Rc::new(Cell::new(None));
        let view = ContentArea::new(Recorder(recorded.clone())).maximum_width(640.0);
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        view.paint(Rect::new(0.0, 0.0, 859.0, 640.0), &mut context);

        assert_eq!(recorded.get(), Some(Rect::new(109.5, 40.0, 640.0, 560.0)));
    }
    #[test]
    fn pane_uses_chrome_inset_without_a_document_width_cap() {
        let recorded = Rc::new(Cell::new(None));
        let view = ContentArea::pane(Recorder(recorded.clone()));
        let mut list = DisplayList::new();
        let mut text = TextMeasurer::new();
        for theme in [Theme::LIGHT, Theme::DARK] {
            for width in [20.0, 420.0, 1200.0] {
                let bounds = Rect::new(10.0, 20.0, width, 500.0);
                view.paint(
                    bounds,
                    &mut PaintContext::new(&mut list, &theme, &Typography::DEFAULT, &mut text),
                );
                let actual = recorded.get().unwrap();
                assert_eq!(
                    actual.origin.x,
                    bounds.origin.x + theme.spacing.extra_large.min(width * 0.5)
                );
                assert_eq!(actual.origin.y, bounds.origin.y + theme.spacing.extra_large);
                assert_eq!(
                    actual.size.width,
                    (width - 2.0 * theme.spacing.extra_large).max(0.0)
                );
            }
        }
    }
}

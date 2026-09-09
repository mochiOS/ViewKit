use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

pub struct NavigationSplitView<Sidebar, Detail> {
    sidebar: Sidebar,
    detail: Detail,
}

impl<Sidebar, Detail> NavigationSplitView<Sidebar, Detail> {
    pub fn new(sidebar: Sidebar, detail: Detail) -> Self {
        Self { sidebar, detail }
    }

    fn regions(bounds: Rect, sidebar_width: f32, divider_width: f32) -> (Rect, Rect, Rect) {
        let sidebar_width = sidebar_width.min(bounds.size.width).max(0.0);
        let divider_width = divider_width
            .min((bounds.size.width - sidebar_width).max(0.0))
            .max(0.0);
        let sidebar = Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            sidebar_width,
            bounds.size.height,
        );
        let divider = Rect::new(
            bounds.origin.x + sidebar_width,
            bounds.origin.y,
            divider_width,
            bounds.size.height,
        );
        let detail = Rect::new(
            divider.origin.x + divider_width,
            bounds.origin.y,
            (bounds.size.width - sidebar_width - divider_width).max(0.0),
            bounds.size.height,
        );
        (sidebar, divider, detail)
    }

    fn sidebar_content(bounds: Rect, horizontal: f32, vertical: f32) -> Rect {
        Rect::new(
            bounds.origin.x + horizontal,
            bounds.origin.y + vertical,
            (bounds.size.width - horizontal * 2.0).max(0.0),
            (bounds.size.height - vertical * 2.0).max(0.0),
        )
    }
}

impl<Sidebar: View, Detail: View> View for NavigationSplitView<Sidebar, Detail> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let sidebar_width = context.theme.layout.navigation_sidebar_width;
        let divider_width = context.theme.divider.thickness;
        let horizontal = context.theme.spacing.medium;
        let vertical = context.theme.spacing.large;
        let sidebar = self.sidebar.measure(
            Constraints::loose(Size::new(
                (sidebar_width - horizontal * 2.0).max(0.0),
                (constraints.maximum.height - vertical * 2.0).max(0.0),
            )),
            context,
        );
        let detail = self.detail.measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - sidebar_width - divider_width).max(0.0),
                constraints.maximum.height,
            )),
            context,
        );
        constraints.constrain(Size::new(
            sidebar_width + divider_width + detail.width,
            (sidebar.height + vertical * 2.0).max(detail.height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let (sidebar, divider, detail) = Self::regions(
            bounds,
            context.theme.layout.navigation_sidebar_width,
            context.theme.divider.thickness,
        );
        Rectangle::new()
            .color(RectangleColor::SubtleSurface)
            .paint(sidebar, context);
        context.display_list.push(DrawCommand::FillRect {
            rect: divider,
            color: context.theme.colors.surface_muted,
        });
        self.sidebar.paint(
            Self::sidebar_content(
                sidebar,
                context.theme.spacing.medium,
                context.theme.spacing.large,
            ),
            context,
        );
        self.detail.paint(detail, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let (sidebar, _, detail) = Self::regions(
            bounds,
            context.theme.layout.navigation_sidebar_width,
            context.theme.divider.thickness,
        );
        let sidebar = Self::sidebar_content(
            sidebar,
            context.theme.spacing.medium,
            context.theme.spacing.large,
        );

        if event.requires_broadcast() {
            return self
                .sidebar
                .handle_event(sidebar, event, context)
                .merge(self.detail.handle_event(detail, event, context));
        }
        if event.is_inside(sidebar) {
            self.sidebar.handle_event(sidebar, event, context)
        } else if event.is_inside(detail) {
            self.detail.handle_event(detail, event, context)
        } else {
            EventResult::Ignored
        }
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
    fn desktop_regions_match_the_chat_screen() {
        let sidebar = Rc::new(Cell::new(None));
        let detail = Rc::new(Cell::new(None));
        let view = NavigationSplitView::new(Recorder(sidebar.clone()), Recorder(detail.clone()));
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        view.paint(Rect::new(0.0, 0.0, 1180.0, 760.0), &mut context);

        assert_eq!(sidebar.get(), Some(Rect::new(12.0, 16.0, 296.0, 728.0)));
        assert_eq!(detail.get(), Some(Rect::new(321.0, 0.0, 859.0, 760.0)));
    }
}

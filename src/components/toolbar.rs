use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolbarPlacement {
    #[default]
    Top,
    Bottom,
}

pub struct Toolbar<Content> {
    placement: ToolbarPlacement,
    content: Content,
}

impl<Content> Toolbar<Content> {
    pub fn new(content: Content) -> Self {
        Self {
            placement: ToolbarPlacement::Top,
            content,
        }
    }

    pub fn bottom(content: Content) -> Self {
        Self {
            placement: ToolbarPlacement::Bottom,
            content,
        }
    }

    fn target_height(&self, context: &MeasureContext<'_>) -> f32 {
        match self.placement {
            ToolbarPlacement::Top => context.theme.layout.top_bar_height,
            ToolbarPlacement::Bottom => context.theme.layout.bottom_bar_height,
        }
    }

    fn content_bounds(&self, bounds: Rect, horizontal: f32, vertical: f32) -> Rect {
        Rect::new(
            bounds.origin.x + horizontal,
            bounds.origin.y + vertical,
            (bounds.size.width - horizontal * 2.0).max(0.0),
            (bounds.size.height - vertical * 2.0).max(0.0),
        )
    }
}

impl<Content: View> View for Toolbar<Content> {
    fn stack_flex_shrink(&self) -> f32 {
        0.0
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let horizontal = context.theme.spacing.extra_large;
        let vertical = match self.placement {
            ToolbarPlacement::Top => context.theme.spacing.small,
            ToolbarPlacement::Bottom => context.theme.spacing.medium,
        };
        let child_constraints = Constraints::loose(Size::new(
            (constraints.maximum.width - horizontal * 2.0).max(0.0),
            (constraints.maximum.height - vertical * 2.0).max(0.0),
        ));
        let content = self.content.measure(child_constraints, context);

        constraints.constrain(Size::new(
            content.width + horizontal * 2.0,
            self.target_height(context)
                .max(content.height + vertical * 2.0),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.record_accessibility(AccessibilityNode::new(AccessibilityRole::Toolbar, bounds));
        Rectangle::new()
            .color(RectangleColor::Surface)
            .paint(bounds, context);
        let horizontal = context.theme.spacing.extra_large;
        let vertical = match self.placement {
            ToolbarPlacement::Top => context.theme.spacing.small,
            ToolbarPlacement::Bottom => context.theme.spacing.medium,
        };
        self.content
            .paint(self.content_bounds(bounds, horizontal, vertical), context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let horizontal = context.theme.spacing.extra_large;
        let vertical = match self.placement {
            ToolbarPlacement::Top => context.theme.spacing.small,
            ToolbarPlacement::Bottom => context.theme.spacing.medium,
        };
        self.content.handle_event(
            self.content_bounds(bounds, horizontal, vertical),
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
    fn toolbar_insets_match_the_chat_screen() {
        let top = Rc::new(Cell::new(None));
        let bottom = Rc::new(Cell::new(None));
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        Toolbar::new(Recorder(top.clone())).paint(Rect::new(0.0, 0.0, 859.0, 48.0), &mut context);
        Toolbar::bottom(Recorder(bottom.clone()))
            .paint(Rect::new(0.0, 0.0, 859.0, 56.0), &mut context);

        assert_eq!(top.get(), Some(Rect::new(24.0, 8.0, 811.0, 32.0)));
        assert_eq!(bottom.get(), Some(Rect::new(24.0, 12.0, 811.0, 32.0)));
    }
}

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::{Rectangle, RectangleColor};

/// A standard mochiOS navigation surface.
///
/// `Sidebar` owns the sidebar background and content insets so applications do
/// not need to reproduce those details. `NavigationSplitView` remains useful
/// when the sidebar and detail should be arranged as a desktop split view.
pub struct Sidebar<Content> {
    content: Content,
}

impl<Content> Sidebar<Content> {
    pub const fn new(content: Content) -> Self {
        Self { content }
    }

    fn content_bounds(bounds: Rect, horizontal: f32, vertical: f32) -> Rect {
        Rect::new(
            bounds.origin.x + horizontal,
            bounds.origin.y + vertical,
            (bounds.size.width - horizontal * 2.0).max(0.0),
            (bounds.size.height - vertical * 2.0).max(0.0),
        )
    }
}

impl<Content: View> View for Sidebar<Content> {
    fn stack_flex_shrink(&self) -> f32 {
        0.0
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let horizontal = context.theme.spacing.medium;
        let vertical = context.theme.spacing.large;
        let content = self.content.measure(
            Constraints::loose(Size::new(
                (context.theme.layout.navigation_sidebar_width - horizontal * 2.0).max(0.0),
                (constraints.maximum.height - vertical * 2.0).max(0.0),
            )),
            context,
        );

        constraints.constrain(Size::new(
            context.theme.layout.navigation_sidebar_width,
            content.height + vertical * 2.0,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        context.record_accessibility(AccessibilityNode::new(
            AccessibilityRole::Navigation,
            bounds,
        ));
        Rectangle::new()
            .color(RectangleColor::SubtleSurface)
            .paint(bounds, context);
        self.content.paint(
            Self::content_bounds(
                bounds,
                context.theme.spacing.medium,
                context.theme.spacing.large,
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
                context.theme.spacing.medium,
                context.theme.spacing.large,
            ),
            event,
            context,
        )
    }
}

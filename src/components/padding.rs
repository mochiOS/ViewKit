//! 子Viewの周囲へ余白を追加するPaddingを定義

use crate::edge_insets::EdgeInsets;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default)]
pub struct EmptyPaddingContent;

impl View for EmptyPaddingContent {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        constraints.constrain(Size::new(0.0, 0.0))
    }

    fn paint(&self, _bounds: Rect, _context: &mut PaintContext<'_>) {}
}

pub struct Padding<C = EmptyPaddingContent> {
    insets: EdgeInsets,
    content: C,
}

impl Padding<EmptyPaddingContent> {
    pub fn new() -> Self {
        Self {
            insets: EdgeInsets::ZERO,
            content: EmptyPaddingContent,
        }
    }

    pub fn all(value: f32) -> Self {
        Self {
            insets: EdgeInsets::all(value),

            content: EmptyPaddingContent,
        }
    }

    pub fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self {
            insets: EdgeInsets::symmetric(horizontal, vertical),

            content: EmptyPaddingContent,
        }
    }

    pub fn only(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            insets: EdgeInsets::new(top, right, bottom, left),

            content: EmptyPaddingContent,
        }
    }

    pub fn from_insets(insets: EdgeInsets) -> Self {
        Self {
            insets: insets.sanitized(),

            content: EmptyPaddingContent,
        }
    }
}

impl Default for Padding<EmptyPaddingContent> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C> Padding<C> {
    pub fn content<NewContent>(self, content: NewContent) -> Padding<NewContent>
    where
        NewContent: View,
    {
        Padding {
            insets: self.insets,
            content,
        }
    }

    pub fn with_insets(mut self, insets: EdgeInsets) -> Self {
        self.insets = insets.sanitized();

        self
    }

    pub fn insets(&self) -> EdgeInsets {
        self.insets
    }

    fn content_bounds(&self, bounds: Rect) -> Rect {
        let insets = self.insets.sanitized();

        Rect::new(
            bounds.origin.x + insets.left,
            bounds.origin.y + insets.top,
            (bounds.size.width - insets.horizontal()).max(0.0),
            (bounds.size.height - insets.vertical()).max(0.0),
        )
    }
}

impl<C> View for Padding<C>
where
    C: View,
{
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let insets = self.insets.sanitized();

        let horizontal = insets.horizontal();

        let vertical = insets.vertical();

        let child_constraints = Constraints::new(
            Size::new(
                subtract_constraint(constraints.minimum.width, horizontal),
                subtract_constraint(constraints.minimum.height, vertical),
            ),
            Size::new(
                subtract_constraint(constraints.maximum.width, horizontal),
                subtract_constraint(constraints.maximum.height, vertical),
            ),
        );

        let content_size = self.content.measure(child_constraints, context);

        constraints.constrain(Size::new(
            content_size.width + horizontal,
            content_size.height + vertical,
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let content_bounds = self.content_bounds(bounds);

        if content_bounds.size.width <= 0.0 || content_bounds.size.height <= 0.0 {
            return;
        }

        self.content.paint(content_bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let content_bounds = self.content_bounds(bounds);

        if !event.requires_broadcast() && !event.is_inside(content_bounds) {
            return EventResult::Ignored;
        }

        self.content.handle_event(content_bounds, event, context)
    }
}

fn subtract_constraint(value: f32, amount: f32) -> f32 {
    if value == f32::INFINITY {
        return f32::INFINITY;
    }

    if !value.is_finite() {
        return 0.0;
    }

    (value - amount).max(0.0)
}

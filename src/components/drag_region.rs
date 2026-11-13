//! Input-transparent drag fallback around an ordinary View.
use crate::{
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    platform::PointerButton,
    view::{Constraints, MeasureContext, PaintContext, View},
};

/// Marks unconsumed primary pointer presses as window-move gestures.
/// Interactive descendants and their own drag handlers always run first.
/// The platform independently validates a one-use, active input token.
pub struct DragRegion<V> {
    content: V,
}
impl<V: View> DragRegion<V> {
    /// Wraps an ordinary view without changing its layout or state identity.
    pub fn new(content: V) -> Self {
        Self { content }
    }
}
/// Wraps a view in a window drag region using the usual ViewKit event tree.
pub fn drag_region<V: View>(content: V) -> DragRegion<V> {
    DragRegion::new(content)
}
impl<V: View> View for DragRegion<V> {
    fn measure(&self, c: Constraints, cx: &mut MeasureContext<'_>) -> Size {
        self.content.measure(c, cx)
    }
    fn paint(&self, b: Rect, cx: &mut PaintContext<'_>) {
        self.content.paint(b, cx);
    }
    fn handle_event(&self, b: Rect, e: &ViewEvent, cx: &mut EventContext<'_>) -> EventResult {
        let result = self.content.handle_event(b, e, cx);
        if result.is_consumed() {
            return result;
        }
        if matches!(e, ViewEvent::PointerPressed { position, button: PointerButton::Primary } if b.contains(*position))
            && cx.request_window_move()
        {
            return EventResult::Consumed;
        }
        result
    }
}

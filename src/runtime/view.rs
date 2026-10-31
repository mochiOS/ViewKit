use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

impl<T> View for Box<T>
where
    T: View + ?Sized,
{
    fn owns_toolbar_insets(&self) -> bool {
        (**self).owns_toolbar_insets()
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        (**self).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        (**self).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        (**self).handle_event(bounds, event, context)
    }
}

impl<T> View for std::rc::Rc<T>
where
    T: View + ?Sized,
{
    fn stack_flex_shrink(&self) -> f32 {
        (**self).stack_flex_shrink()
    }

    fn owns_toolbar_insets(&self) -> bool {
        (**self).owns_toolbar_insets()
    }

    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        (**self).measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        (**self).paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        (**self).handle_event(bounds, event, context)
    }
}

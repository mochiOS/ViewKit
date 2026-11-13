//! Stable component identity across ordinary view tree reconstruction.
use crate::{
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    runtime::ComponentInstanceId,
    view::{Constraints, MeasureContext, PaintContext, View},
};

/// Gives a State-backed component a stable identity across moves, insertion
/// and removal. Keep IDs unique within a window. This works in title bars and
/// body content equally; it does not create a separate state system.
pub struct Identified<V> {
    id: ComponentInstanceId,
    content: V,
}
impl<V: View> Identified<V> {
    /// Associates an application-stable component ID with a normal View.
    pub fn new(id: ComponentInstanceId, content: V) -> Self {
        Self { id, content }
    }
}
impl<V: View> View for Identified<V> {
    fn measure(&self, c: Constraints, cx: &mut MeasureContext<'_>) -> Size {
        self.content.measure(c, cx)
    }
    fn paint(&self, b: Rect, cx: &mut PaintContext<'_>) {
        cx.with_component_identity(self.id.0, b, |cx| self.content.paint(b, cx));
    }
    fn handle_event(&self, b: Rect, e: &ViewEvent, cx: &mut EventContext<'_>) -> EventResult {
        if let ViewEvent::ReconcileComponents { retained } = e {
            if !retained.iter().any(|key|matches!(key,crate::accessibility::AccessibilityIdentity::Component {id,..} if *id==self.id.0)) {
                // Existing controls already release focus, text selection,
                // pointer arming, IME and their own drag on these events.
                self.content.handle_event(b,&ViewEvent::PointerLeft,cx);
                self.content.handle_event(b,&ViewEvent::FocusChanged {focused:false},cx);
                return EventResult::Ignored;
            }
        }
        self.content.handle_event(b, e, cx)
    }
}

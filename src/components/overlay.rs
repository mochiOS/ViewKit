//! Viewの前面へ別のViewを重ねるOverlayを定義

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::Rect;
use crate::layout::{IntoStackChild, StackChild};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::ZStackAlignment;

pub struct Overlay {
    content: Option<StackChild>,
    overlay: Option<StackChild>,
    alignment: ZStackAlignment,
}

impl Default for Overlay {
    fn default() -> Self {
        Self {
            content: None,
            overlay: None,
            alignment: ZStackAlignment::Center,
        }
    }
}

impl Overlay {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn content<C>(mut self, content: C) -> Self
    where
        C: IntoStackChild,
    {
        self.content = Some(content.into_stack_child());

        self
    }

    pub fn overlay<O>(mut self, overlay: O) -> Self
    where
        O: IntoStackChild,
    {
        self.overlay = Some(overlay.into_stack_child());

        self
    }

    pub fn alignment(mut self, alignment: ZStackAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl View for Overlay {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> crate::geometry::Size {
        let mut measured = crate::geometry::Size::new(0.0, 0.0);
        for child in [self.content.as_ref(), self.overlay.as_ref()].into_iter().flatten() {
            let size = child.measure(Constraints::loose(constraints.maximum), context);
            measured.width = measured.width.max(size.width);
            measured.height = measured.height.max(size.height);
        }
        constraints.constrain(measured)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        if let Some(content) = &self.content {
            let content_size = content.overlay_size(bounds.size);

            let content_bounds = self.alignment.child_bounds(bounds, content_size);

            content.paint(content_bounds, context);
        }

        if let Some(overlay) = &self.overlay {
            let overlay_size = overlay.overlay_size(bounds.size);

            let overlay_bounds = self.alignment.child_bounds(bounds, overlay_size);

            overlay.paint(overlay_bounds, context);
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if event.requires_broadcast() {
            let mut result = EventResult::Ignored;
            for child in [self.content.as_ref(), self.overlay.as_ref()].into_iter().flatten() {
                let child_bounds = self
                    .alignment
                    .child_bounds(bounds, child.overlay_size(bounds.size));
                result = result.merge(child.handle_event(child_bounds, event, context));
            }
            return result;
        }

        let Some(position) = event.position() else {
            return EventResult::Ignored;
        };
        for child in [self.overlay.as_ref(), self.content.as_ref()].into_iter().flatten() {
            let child_bounds = self
                .alignment
                .child_bounds(bounds, child.overlay_size(bounds.size));
            if child_bounds.contains(position) {
                let result = child.handle_event(child_bounds, event, context);
                if result.is_consumed() {
                    return result;
                }
            }
        }
        EventResult::Ignored
    }
}

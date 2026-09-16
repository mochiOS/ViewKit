use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{
    IntoStackChildren, StackAlignment, StackChild, StackDirection, StackDistribution, StackGap,
    handle_stack_event, measure_stack, paint_stack,
};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

/// A width-constrained form region using the standard layout foundation.
pub struct Form<Content> {
    content: Content,
}

impl<Content> Form<Content> {
    pub const fn new(content: Content) -> Self {
        Self { content }
    }

    fn content_bounds(bounds: Rect, maximum_width: f32) -> Rect {
        let width = bounds.size.width.min(maximum_width).max(0.0);
        Rect::new(
            bounds.origin.x + (bounds.size.width - width).max(0.0) / 2.0,
            bounds.origin.y,
            width,
            bounds.size.height,
        )
    }
}

impl<Content: View> View for Form<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let maximum = Size::new(
            constraints
                .maximum
                .width
                .min(context.theme.layout.form_width),
            constraints.maximum.height,
        );
        let measured = self.content.measure(Constraints::loose(maximum), context);
        constraints.constrain(Size::new(
            measured.width.min(maximum.width),
            measured.height.min(maximum.height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.content.paint(
            Self::content_bounds(bounds, context.theme.layout.form_width),
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
            Self::content_bounds(bounds, context.theme.layout.form_width),
            event,
            context,
        )
    }
}

/// A vertical collection of form sections with the standard section gap.
pub struct FormSections {
    children: Vec<StackChild>,
}

impl FormSections {
    pub const fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    pub fn section<Content: IntoStackChildren>(mut self, content: Content) -> Self {
        self.children.extend(content.into_stack_children());
        self
    }

    fn gap(context: &MeasureContext<'_>) -> StackGap {
        StackGap::Custom(context.theme.layout.section_gap)
    }
}

impl Default for FormSections {
    fn default() -> Self {
        Self::new()
    }
}

impl View for FormSections {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        measure_stack(
            StackDirection::Vertical,
            &self.children,
            Self::gap(context),
            constraints,
            context,
        )
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        paint_stack(
            StackDirection::Vertical,
            &self.children,
            bounds,
            StackGap::Custom(context.theme.layout.section_gap),
            StackAlignment::Stretch,
            StackDistribution::Start,
            context,
        );
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        handle_stack_event(
            StackDirection::Vertical,
            &self.children,
            bounds,
            StackGap::Custom(context.theme.layout.section_gap),
            StackAlignment::Stretch,
            StackDistribution::Start,
            event,
            context,
        )
    }
}

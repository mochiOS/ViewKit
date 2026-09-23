use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::layout::{IntoStackChildren, StackChild};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

/// A fixed-cell grid that automatically chooses the number of columns that fit.
///
/// The cell size stays stable as the window grows; extra horizontal space is
/// left at the trailing edge. This is the standard layout for icon libraries,
/// application catalogs, and other browsable collections.
pub struct AdaptiveGrid {
    children: Vec<StackChild>,
    cell_size: Size,
    column_gap: f32,
    row_gap: f32,
}

impl AdaptiveGrid {
    pub fn new(cell_width: f32, cell_height: f32) -> Self {
        Self {
            children: Vec::new(),
            cell_size: Size::new(sanitize(cell_width), sanitize(cell_height)),
            column_gap: 0.0,
            row_gap: 0.0,
        }
    }

    pub fn child<C>(mut self, child: C) -> Self
    where
        C: IntoStackChildren,
    {
        self.children.extend(child.into_stack_children());
        self
    }

    pub fn children<C>(mut self, children: impl IntoIterator<Item = C>) -> Self
    where
        C: IntoStackChildren,
    {
        for child in children {
            self.children.extend(child.into_stack_children());
        }
        self
    }

    pub fn spacing(mut self, column_gap: f32, row_gap: f32) -> Self {
        self.column_gap = sanitize(column_gap);
        self.row_gap = sanitize(row_gap);
        self
    }

    fn columns(&self, width: f32) -> usize {
        if self.children.is_empty() || self.cell_size.width <= 0.0 {
            return 1;
        }
        let width = sanitize(width);
        (((width + self.column_gap) / (self.cell_size.width + self.column_gap)).floor() as usize)
            .max(1)
    }

    fn content_size(&self, available_width: f32) -> Size {
        if self.children.is_empty() {
            return Size::ZERO;
        }
        let columns = self.columns(available_width).min(self.children.len());
        let rows = self.children.len().div_ceil(columns);
        Size::new(
            self.cell_size.width * columns as f32
                + self.column_gap * columns.saturating_sub(1) as f32,
            self.cell_size.height * rows as f32 + self.row_gap * rows.saturating_sub(1) as f32,
        )
    }

    fn child_bounds(&self, bounds: Rect, index: usize) -> Rect {
        let columns = self.columns(bounds.size.width);
        let column = index % columns;
        let row = index / columns;
        Rect::new(
            bounds.origin.x + column as f32 * (self.cell_size.width + self.column_gap),
            bounds.origin.y + row as f32 * (self.cell_size.height + self.row_gap),
            self.cell_size.width,
            self.cell_size.height,
        )
    }
}

impl View for AdaptiveGrid {
    fn measure(&self, constraints: Constraints, _context: &mut MeasureContext<'_>) -> Size {
        let available_width = if constraints.maximum.width.is_finite() {
            constraints.maximum.width
        } else {
            self.content_size(f32::MAX).width
        };
        constraints.constrain(self.content_size(available_width))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        for (index, child) in self.children.iter().enumerate() {
            child.paint(self.child_bounds(bounds, index), context);
        }
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        if event.requires_broadcast() {
            return self
                .children
                .iter()
                .enumerate()
                .fold(EventResult::Ignored, |result, (index, child)| {
                    result.merge(child.handle_event(
                        self.child_bounds(bounds, index),
                        event,
                        context,
                    ))
                });
        }
        let Some(position) = event.position() else {
            return EventResult::Ignored;
        };
        for (index, child) in self.children.iter().enumerate() {
            let child_bounds = self.child_bounds(bounds, index);
            if child_bounds.contains(position) {
                return child.handle_event(child_bounds, event, context);
            }
        }
        EventResult::Ignored
    }
}

fn sanitize(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::draw_command::DisplayList;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};
    use std::cell::RefCell;
    use std::rc::Rc;

    struct Recorder(Rc<RefCell<Vec<Rect>>>);

    impl View for Recorder {
        fn paint(&self, bounds: Rect, _context: &mut PaintContext<'_>) {
            self.0.borrow_mut().push(bounds);
        }
    }

    #[test]
    fn wraps_fixed_cells_without_stretching_them() {
        let recorded = Rc::new(RefCell::new(Vec::new()));
        let grid = AdaptiveGrid::new(184.0, 158.0)
            .spacing(20.0, 32.0)
            .children((0..5).map(|_| Recorder(Rc::clone(&recorded))));
        let mut display_list = DisplayList::new();
        let mut text_measurer = TextMeasurer::new();
        let mut context = PaintContext::new(
            &mut display_list,
            &Theme::LIGHT,
            &Typography::DEFAULT,
            &mut text_measurer,
        );

        grid.paint(Rect::new(10.0, 20.0, 600.0, 400.0), &mut context);

        assert_eq!(
            *recorded.borrow(),
            vec![
                Rect::new(10.0, 20.0, 184.0, 158.0),
                Rect::new(214.0, 20.0, 184.0, 158.0),
                Rect::new(418.0, 20.0, 184.0, 158.0),
                Rect::new(10.0, 210.0, 184.0, 158.0),
                Rect::new(214.0, 210.0, 184.0, 158.0),
            ]
        );
    }
}

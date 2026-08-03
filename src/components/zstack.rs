//! 子Viewを奥行き方向へ重ねるZStackを定義

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Point, Rect, Size};
use crate::layout::{IntoStackChild, StackChild};
use crate::view::{PaintContext, View};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ZStackAlignment {
    TopLeading,
    Top,
    TopTrailing,
    Leading,

    #[default]
    Center,

    Trailing,
    BottomLeading,
    Bottom,
    BottomTrailing,
}

impl ZStackAlignment {
    fn horizontal_factor(self) -> f32 {
        match self {
            Self::TopLeading | Self::Leading | Self::BottomLeading => 0.0,

            Self::Top | Self::Center | Self::Bottom => 0.5,

            Self::TopTrailing | Self::Trailing | Self::BottomTrailing => 1.0,
        }
    }

    fn vertical_factor(self) -> f32 {
        match self {
            Self::TopLeading | Self::Top | Self::TopTrailing => 0.0,

            Self::Leading | Self::Center | Self::Trailing => 0.5,

            Self::BottomLeading | Self::Bottom | Self::BottomTrailing => 1.0,
        }
    }

    pub(crate) fn child_origin(self, bounds: Rect, child_size: Size) -> Point {
        let remaining_width = bounds.size.width - child_size.width;

        let remaining_height = bounds.size.height - child_size.height;

        Point::new(
            bounds.origin.x + remaining_width * self.horizontal_factor(),
            bounds.origin.y + remaining_height * self.vertical_factor(),
        )
    }

    pub(crate) fn child_bounds(self, bounds: Rect, child_size: Size) -> Rect {
        let origin = self.child_origin(bounds, child_size);

        Rect::new(origin.x, origin.y, child_size.width, child_size.height)
    }
}

pub struct ZStack {
    children: Vec<StackChild>,
    alignment: ZStackAlignment,
}

impl Default for ZStack {
    fn default() -> Self {
        Self {
            children: Vec::new(),
            alignment: ZStackAlignment::Center,
        }
    }
}

impl ZStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn child<C>(mut self, child: C) -> Self
    where
        C: IntoStackChild,
    {
        self.children.push(child.into_stack_child());

        self
    }

    pub fn children<C>(mut self, children: impl IntoIterator<Item = C>) -> Self
    where
        C: IntoStackChild,
    {
        self.children
            .extend(children.into_iter().map(IntoStackChild::into_stack_child));

        self
    }

    pub fn alignment(mut self, alignment: ZStackAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

impl View for ZStack {
    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
            return;
        }

        for child in &self.children {
            let child_size = child.overlay_size(bounds.size);

            let child_bounds = self.alignment.child_bounds(bounds, child_size);

            child.paint(child_bounds, context);
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
            for child in &self.children {
                let child_size = child.overlay_size(bounds.size);
                let child_bounds = self.alignment.child_bounds(bounds, child_size);
                result = result.merge(child.handle_event(child_bounds, event, context));
            }
            return result;
        }

        let Some(position) = event.position() else {
            return EventResult::Ignored;
        };

        for child in self.children.iter().rev() {
            let child_size = child.overlay_size(bounds.size);
            let child_bounds = self.alignment.child_bounds(bounds, child_size);
            if !child_bounds.contains(position) {
                continue;
            }
            let result = child.handle_event(child_bounds, event, context);
            if result.is_consumed() {
                return result;
            }
        }

        EventResult::Ignored
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::components::{TextField, TextFieldInteractionState};
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    struct EventProbe {
        events: Rc<Cell<usize>>,
    }

    impl View for EventProbe {
        fn paint(&self, _bounds: Rect, _context: &mut PaintContext<'_>) {}

        fn handle_event(
            &self,
            _bounds: Rect,
            _event: &ViewEvent,
            _context: &mut EventContext<'_>,
        ) -> EventResult {
            self.events.set(self.events.get() + 1);
            EventResult::Consumed
        }
    }

    #[test]
    fn broadcasts_text_input_to_every_child() {
        let events = Rc::new(Cell::new(0));
        let stack = ZStack::new()
            .child(EventProbe {
                events: Rc::clone(&events),
            })
            .child(EventProbe {
                events: Rc::clone(&events),
            });
        let mut text_measurer = TextMeasurer::new();
        let mut context =
            EventContext::new(&Theme::DEFAULT, &Typography::DEFAULT, &mut text_measurer);

        let result = stack.handle_event(
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &ViewEvent::TextInput {
                text: "a".to_owned(),
            },
            &mut context,
        );

        assert_eq!(events.get(), 2);
        assert_eq!(result, EventResult::Consumed);
    }

    #[test]
    fn routes_text_input_to_a_focused_text_field() {
        let interaction = TextFieldInteractionState::new();
        interaction.set_focused(true);
        let stack = ZStack::new().child(TextField::with_interaction(interaction.clone()));
        let mut text_measurer = TextMeasurer::new();
        let mut context =
            EventContext::new(&Theme::DEFAULT, &Typography::DEFAULT, &mut text_measurer);

        let result = stack.handle_event(
            Rect::new(0.0, 0.0, 800.0, 600.0),
            &ViewEvent::TextInput {
                text: "a".to_owned(),
            },
            &mut context,
        );

        assert_eq!(interaction.value(), "a");
        assert_eq!(result, EventResult::Consumed);
    }
}

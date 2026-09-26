//! A responder for files dragged into an application window.

use std::cell::{Cell, RefCell};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

/// Shared hover state for custom drop-target appearance.
#[derive(Clone, Default)]
pub struct FileDropInteractionState {
    targeted: Rc<Cell<bool>>,
}

impl FileDropInteractionState {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn is_targeted(&self) -> bool {
        self.targeted.get()
    }
}

/// Accepts files over a child view and invokes an action after a drop.
pub struct FileDropTarget<Content> {
    content: Content,
    interaction: FileDropInteractionState,
    accepts: Rc<dyn Fn(&Path) -> bool>,
    action: RefCell<Box<dyn FnMut(PathBuf)>>,
}

impl<Content> FileDropTarget<Content> {
    pub fn new(content: Content, action: impl FnMut(PathBuf) + 'static) -> Self {
        Self {
            content,
            interaction: FileDropInteractionState::new(),
            accepts: Rc::new(|_| true),
            action: RefCell::new(Box::new(action)),
        }
    }

    #[must_use]
    pub fn accepts(mut self, predicate: impl Fn(&Path) -> bool + 'static) -> Self {
        self.accepts = Rc::new(predicate);
        self
    }

    #[must_use]
    pub fn interaction(mut self, interaction: FileDropInteractionState) -> Self {
        self.interaction = interaction;
        self
    }

    pub fn content(&self) -> &Content {
        &self.content
    }
}

impl<Content: View> View for FileDropTarget<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.content.paint(bounds, context);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let child = self.content.handle_event(bounds, event, context);
        if child.is_consumed() {
            return child;
        }
        match event {
            ViewEvent::FileDragEntered { path, position } => {
                let targeted = position.is_some_and(|position| bounds.contains(position))
                    && (self.accepts)(path);
                if self.interaction.targeted.replace(targeted) != targeted {
                    context.request_redraw_in(bounds);
                }
                if targeted {
                    EventResult::Consumed
                } else {
                    EventResult::Ignored
                }
            }
            ViewEvent::FileDragExited => {
                if self.interaction.targeted.replace(false) {
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            ViewEvent::FileDropped { path, position }
                if position.is_some_and(|position| bounds.contains(position))
                    && (self.accepts)(path) =>
            {
                self.interaction.targeted.set(false);
                (self.action.borrow_mut())(path.clone());
                context.request_redraw_in(bounds);
                EventResult::Consumed
            }
            ViewEvent::FileDropped { .. } => {
                if self.interaction.targeted.replace(false) {
                    context.request_redraw_in(bounds);
                }
                EventResult::Ignored
            }
            _ => EventResult::Ignored,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::Text;
    use crate::geometry::Point;
    use crate::theme::Theme;
    use crate::typography::TextMeasurer;

    #[test]
    fn accepts_matching_files_inside_its_bounds() {
        let dropped = Rc::new(RefCell::new(None));
        let output = Rc::clone(&dropped);
        let interaction = FileDropInteractionState::new();
        let target = FileDropTarget::new(Text::body("Drop"), move |path| {
            *output.borrow_mut() = Some(path);
        })
        .accepts(|path| path.extension().and_then(|value| value.to_str()) == Some("txt"))
        .interaction(interaction.clone());
        let theme = Theme::LIGHT;
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        let bounds = Rect::new(0.0, 0.0, 100.0, 100.0);
        let path = PathBuf::from("note.txt");

        assert_eq!(
            target.handle_event(
                bounds,
                &ViewEvent::FileDragEntered {
                    path: path.clone(),
                    position: Some(Point::new(20.0, 30.0)),
                },
                &mut context,
            ),
            EventResult::Consumed
        );
        assert!(interaction.is_targeted());
        assert_eq!(
            target.handle_event(
                bounds,
                &ViewEvent::FileDropped {
                    path: path.clone(),
                    position: Some(Point::new(20.0, 30.0)),
                },
                &mut context,
            ),
            EventResult::Consumed
        );
        assert_eq!(*dropped.borrow(), Some(path));
        assert!(!interaction.is_targeted());
    }
}

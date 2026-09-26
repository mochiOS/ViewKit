//! A responder-chain command handler around a child view.

use std::cell::RefCell;

use crate::command::{CommandId, CommandStatus};
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::view::{Constraints, MeasureContext, PaintContext, View};

/// Handles a semantic command after giving the focused descendant first refusal.
pub struct CommandTarget<Content, Action> {
    content: Content,
    command: CommandId,
    action: RefCell<Action>,
}

impl<Content, Action> CommandTarget<Content, Action> {
    pub fn new(content: Content, command: CommandId, action: Action) -> Self {
        Self {
            content,
            command,
            action: RefCell::new(action),
        }
    }

    pub fn content(&self) -> &Content {
        &self.content
    }
}

impl<Content, Action> View for CommandTarget<Content, Action>
where
    Content: View,
    Action: FnMut(),
{
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        self.content.measure(constraints, context)
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        self.content.paint(bounds, context);
        context.record_command_status(CommandStatus::new(self.command, bounds, true));
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let child_result = self.content.handle_event(bounds, event, context);
        if child_result.is_consumed() {
            return child_result;
        }
        if matches!(event, ViewEvent::Command { command, .. } if *command == self.command) {
            (self.action.borrow_mut())();
            context.request_redraw();
            EventResult::Consumed
        } else {
            EventResult::Ignored
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;
    use crate::command::standard;
    use crate::components::Text;
    use crate::theme::Theme;
    use crate::typography::TextMeasurer;

    #[test]
    fn handles_matching_commands() {
        let calls = Rc::new(Cell::new(0));
        let action_calls = Rc::clone(&calls);
        let target = CommandTarget::new(Text::body(""), standard::SAVE, move || {
            action_calls.set(action_calls.get() + 1);
        });
        let theme = Theme::LIGHT;
        let mut measurer = TextMeasurer::new();
        let mut context = EventContext::new(&theme, &theme.typography, &mut measurer);
        let event = ViewEvent::Command {
            command: standard::SAVE,
            target: None,
        };

        assert_eq!(
            target.handle_event(Rect::new(0.0, 0.0, 100.0, 100.0), &event, &mut context),
            EventResult::Consumed
        );
        assert_eq!(calls.get(), 1);
    }
}

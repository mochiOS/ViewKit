use std::cell::RefCell;
use std::rc::Rc;

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::draw_command::DrawCommand;
use crate::event::{EventContext, EventResult, ViewEvent};
use crate::geometry::{Rect, Size};
use crate::platform::Key;
use crate::view::{Constraints, MeasureContext, PaintContext, View};

use super::background::EmptyView;
use super::{BorderStyle, Rectangle, RectangleColor};

pub struct Dialog<Content = EmptyView> {
    content: Content,
    accessibility_label: Option<String>,
    focus_scope: bool,
    on_dismiss: Option<Rc<RefCell<Box<dyn FnMut()>>>>,
}

impl Dialog<EmptyView> {
    pub const fn new() -> Self {
        Self {
            content: EmptyView,
            accessibility_label: None,
            focus_scope: true,
            on_dismiss: None,
        }
    }
}

impl Default for Dialog<EmptyView> {
    fn default() -> Self {
        Self::new()
    }
}

impl<Content> Dialog<Content> {
    pub fn content<NewContent: View>(self, content: NewContent) -> Dialog<NewContent> {
        Dialog {
            content,
            accessibility_label: self.accessibility_label,
            focus_scope: self.focus_scope,
            on_dismiss: self.on_dismiss,
        }
    }

    pub fn accessibility_label(mut self, label: impl Into<String>) -> Self {
        self.accessibility_label = Some(label.into());
        self
    }

    pub fn focus_scope(mut self, focus_scope: bool) -> Self {
        self.focus_scope = focus_scope;
        self
    }

    pub fn on_dismiss(mut self, callback: impl FnMut() + 'static) -> Self {
        self.on_dismiss = Some(Rc::new(RefCell::new(Box::new(callback))));
        self
    }
}

impl<Content: View> View for Dialog<Content> {
    fn measure(&self, constraints: Constraints, context: &mut MeasureContext<'_>) -> Size {
        let inset = context.theme.dialog.padding;
        let child = self.content.measure(
            Constraints::loose(Size::new(
                (constraints.maximum.width - inset * 2.0).max(0.0),
                (constraints.maximum.height - inset * 2.0).max(0.0),
            )),
            context,
        );
        constraints.constrain(Size::new(
            (child.width + inset * 2.0).max(context.theme.layout.dialog_width),
            (child.height + inset * 2.0).max(context.theme.layout.dialog_height),
        ))
    }

    fn paint(&self, bounds: Rect, context: &mut PaintContext<'_>) {
        let mut node = AccessibilityNode::new(AccessibilityRole::Dialog, bounds);
        node.label = self.accessibility_label.clone();
        node.focus_scope = self.focus_scope;
        context.record_accessibility(node);
        Rectangle::new()
            .color(RectangleColor::Custom(context.theme.dialog.background))
            .radius(context.theme.dialog.radius)
            .border(BorderStyle::custom(
                context.theme.dialog.border,
                context.theme.dialog.stroke_width,
            ))
            .paint(bounds, context);
        context
            .display_list
            .push(DrawCommand::PushClip { rect: bounds });
        self.content
            .paint(inset_rect(bounds, context.theme.dialog.padding), context);
        context.display_list.push(DrawCommand::PopClip);
    }

    fn handle_event(
        &self,
        bounds: Rect,
        event: &ViewEvent,
        context: &mut EventContext<'_>,
    ) -> EventResult {
        let result = self.content.handle_event(
            inset_rect(bounds, context.theme.dialog.padding),
            event,
            context,
        );
        if result.is_consumed() {
            return result;
        }
        if matches!(event, ViewEvent::KeyPressed { key: Key::Escape, .. })
            && let Some(callback) = self.on_dismiss.as_ref()
        {
            (callback.borrow_mut())();
            context.request_redraw();
            return EventResult::Consumed;
        }
        result
    }
}

fn inset_rect(bounds: Rect, inset: f32) -> Rect {
    Rect::new(
        bounds.origin.x + inset,
        bounds.origin.y + inset,
        (bounds.size.width - inset * 2.0).max(0.0),
        (bounds.size.height - inset * 2.0).max(0.0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Theme;
    use crate::typography::{TextMeasurer, Typography};

    #[test]
    fn empty_dialog_uses_foundation_size() {
        let mut text_measurer = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &Theme::LIGHT,
            typography: &Typography::DEFAULT,
            text_measurer: &mut text_measurer,
        };
        assert_eq!(
            Dialog::new().measure(
                Constraints::loose(Size::new(f32::INFINITY, f32::INFINITY)),
                &mut context,
            ),
            Size::new(360.0, 200.0),
        );
    }
}

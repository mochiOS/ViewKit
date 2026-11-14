//! Retained hover help, rendered by the outer host after ordinary content.
use super::Tooltip;
use crate::{
    event::{EventContext, EventResult, ViewEvent},
    geometry::{Rect, Size},
    view::{Constraints, MeasureContext, PaintContext, View},
};
use std::{
    cell::RefCell,
    rc::Rc,
    time::{Duration, Instant},
};

const DELAY: Duration = Duration::from_millis(600);

#[derive(Clone)]
struct Anchor {
    label: String,
    bounds: Rect,
    since: Instant,
}

/// Shared hover-help state for one window. Keep this across view reconstruction.
/// Attach controls with `Help` and paint their tooltip using one outer `HelpHost`.
#[derive(Clone, Default)]
pub struct HelpState(Rc<RefCell<Option<Anchor>>>);

impl HelpState {
    /// Creates empty window-local help state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Cancels help when a menu, document or window changes.
    pub fn dismiss(&self) {
        self.0.borrow_mut().take();
    }

    fn hover(&self, label: &str, bounds: Rect, inside: bool) -> bool {
        let mut anchor = self.0.borrow_mut();
        let owns = anchor
            .as_ref()
            .is_some_and(|a| a.label == label && a.bounds == bounds);
        if inside && !owns {
            *anchor = Some(Anchor {
                label: label.into(),
                bounds,
                since: Instant::now(),
            });
            true
        } else if !inside && owns {
            *anchor = None;
            true
        } else {
            false
        }
    }
}

/// Adds delayed help without changing the wrapped control's layout or input.
/// Use an explicit accessible label on the control as well as visible help.
pub struct Help<V> {
    content: V,
    label: String,
    state: HelpState,
}

impl<V: View> Help<V> {
    /// Wraps a native control and shares the window's hover-help owner.
    pub fn new(content: V, label: impl Into<String>, state: HelpState) -> Self {
        Self {
            content,
            label: label.into(),
            state,
        }
    }
}

impl<V: View> View for Help<V> {
    fn measure(&self, c: Constraints, m: &mut MeasureContext<'_>) -> Size {
        self.content.measure(c, m)
    }

    fn paint(&self, b: Rect, c: &mut PaintContext<'_>) {
        self.content.paint(b, c);
    }

    fn handle_event(&self, b: Rect, e: &ViewEvent, c: &mut EventContext<'_>) -> EventResult {
        if let ViewEvent::PointerMoved { position } = e {
            if self.state.hover(&self.label, b, b.contains(*position)) {
                c.request_redraw();
            }
        }
        self.content.handle_event(b, e, c)
    }
}

/// Paints standard ViewKit Tooltip bubbles above content and inside the window.
/// Hosts forward every event unchanged; tooltips never capture pointer or keys.
pub struct HelpHost<V> {
    content: V,
    state: HelpState,
}

impl<V: View> HelpHost<V> {
    /// Wraps the root of one native window, outside clipping/scroll containers.
    pub fn new(content: V, state: HelpState) -> Self {
        Self { content, state }
    }
}

impl<V: View> View for HelpHost<V> {
    fn measure(&self, c: Constraints, m: &mut MeasureContext<'_>) -> Size {
        self.content.measure(c, m)
    }

    fn paint(&self, b: Rect, c: &mut PaintContext<'_>) {
        self.content.paint(b, c);
        let Some(anchor) = self.state.0.borrow().clone() else {
            return;
        };
        if b.intersection(anchor.bounds).is_none() {
            self.state.dismiss();
            return;
        }
        let deadline = anchor.since + DELAY;
        if Instant::now() < deadline {
            c.request_redraw_at(deadline);
            return;
        }
        let bubble = Tooltip::new(anchor.label);
        let size = bubble.measure(
            Constraints::loose(b.size),
            &mut MeasureContext {
                theme: c.theme,
                typography: c.typography,
                text_measurer: c.text_measurer,
            },
        );
        let gap = c.theme.spacing.small;
        let x = anchor.bounds.origin.x.clamp(
            b.origin.x,
            (b.origin.x + b.size.width - size.width).max(b.origin.x),
        );
        let below = anchor.bounds.origin.y + anchor.bounds.size.height + gap;
        let y = if below + size.height <= b.origin.y + b.size.height {
            below
        } else {
            (anchor.bounds.origin.y - size.height - gap).max(b.origin.y)
        };
        bubble.paint(Rect::new(x, y, size.width, size.height), c);
    }

    fn handle_event(&self, b: Rect, e: &ViewEvent, c: &mut EventContext<'_>) -> EventResult {
        if matches!(
            e,
            ViewEvent::PointerLeft
                | ViewEvent::PointerPressed { .. }
                | ViewEvent::KeyPressed { .. }
                | ViewEvent::Scroll { .. }
                | ViewEvent::FocusChanged { focused: false }
        ) {
            self.state.dismiss();
        }
        self.content.handle_event(b, e, c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_another_control_cannot_cancel_the_current_anchor() {
        let state = HelpState::new();
        let a = Rect::new(0., 0., 32., 32.);
        let b = Rect::new(40., 0., 32., 32.);
        state.hover("Build", a, true);
        state.hover("Run", b, true);
        state.hover("Build", a, false);
        assert_eq!(state.0.borrow().as_ref().unwrap().label, "Run");
        state.hover("Run", b, false);
        assert!(state.0.borrow().is_none());
    }
}

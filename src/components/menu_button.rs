//! An icon button that opens a standard command menu.
use super::{ButtonSize, IconButton, IconButtonInteractionState, SymbolName};
use crate::{
    event::{ContextMenuItem, ContextMenuRequest, EventContext, EventResult, ViewEvent},
    geometry::{Point, Rect, Size},
    view::{Constraints, MeasureContext, PaintContext, View},
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct OptionItem {
    label: String,
    enabled: bool,
    callback: RefCell<Box<dyn FnMut()>>,
}

/// A native symbol-only command menu with a visible hover target and an
/// accessible label. Retain the instance across paints while its menu is open.
pub struct MenuButton {
    label: String,
    symbol: SymbolName,
    size: ButtonSize,
    enabled: bool,
    interaction: IconButtonInteractionState,
    clicked: Rc<Cell<bool>>,
    options: Vec<OptionItem>,
    request: Cell<Option<u64>>,
}
impl MenuButton {
    /// Creates a command menu using the shared icon-button appearance.
    pub fn new(label: impl Into<String>, symbol: SymbolName) -> Self {
        Self {
            label: label.into(),
            symbol,
            size: ButtonSize::Small,
            enabled: true,
            interaction: IconButtonInteractionState::new(),
            clicked: Rc::new(Cell::new(false)),
            options: vec![],
            request: Cell::new(None),
        }
    }
    /// Selects a standard native control size.
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }
    /// Enables or disables both button activation and command dispatch.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
    /// Adds an enabled command to the menu.
    pub fn option(mut self, label: impl Into<String>, callback: impl FnMut() + 'static) -> Self {
        self.options.push(OptionItem {
            label: label.into(),
            enabled: true,
            callback: RefCell::new(Box::new(callback)),
        });
        self
    }
    /// Adds a visible unavailable command, without an action callback.
    pub fn disabled_option(mut self, label: impl Into<String>) -> Self {
        self.options.push(OptionItem {
            label: label.into(),
            enabled: false,
            callback: RefCell::new(Box::new(|| {})),
        });
        self
    }
    fn button(&self) -> IconButton {
        let clicked = self.clicked.clone();
        IconButton::with_interaction(self.symbol, self.interaction.clone())
            .size(self.size)
            .enabled(self.enabled)
            .accessibility_label(self.label.clone())
            .on_click(move || clicked.set(true))
    }
}
impl View for MenuButton {
    fn measure(&self, c: Constraints, m: &mut MeasureContext<'_>) -> Size {
        self.button().measure(c, m)
    }
    fn paint(&self, b: Rect, c: &mut PaintContext<'_>) {
        self.button().paint(b, c);
    }
    fn handle_event(&self, b: Rect, e: &ViewEvent, c: &mut EventContext<'_>) -> EventResult {
        if let ViewEvent::ContextMenuResult {
            request_id,
            command_id,
        } = e
        {
            if self.request.get() == Some(*request_id) {
                self.request.set(None);
                if self.enabled {
                    if let Some(option) = command_id
                        .and_then(|id| id.checked_sub(1))
                        .and_then(|i| self.options.get(i as usize))
                        .filter(|o| o.enabled)
                    {
                        (option.callback.borrow_mut())();
                        c.request_redraw();
                    }
                }
                return EventResult::Consumed;
            }
        }
        let result = self.button().handle_event(b, e, c);
        if self.clicked.replace(false) && self.enabled && !self.options.is_empty() {
            let id = ContextMenuRequest::next_id();
            self.request.set(Some(id));
            c.show_context_menu(ContextMenuRequest {
                request_id: id,
                position: Point::new(b.origin.x, b.origin.y + b.size.height),
                items: self
                    .options
                    .iter()
                    .enumerate()
                    .map(|(i, o)| ContextMenuItem {
                        command_id: u32::try_from(i + 1).unwrap_or(u32::MAX),
                        label: o.label.clone(),
                        enabled: o.enabled,
                        checked: false,
                        destructive: false,
                        separator: false,
                    })
                    .collect(),
            });
            return EventResult::Consumed;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accessibility::AccessibilityRole,
        draw_command::{DisplayList, DrawCommand},
        platform::{Key, KeyModifiers},
        theme::Theme,
        typography::TextMeasurer,
    };
    #[test]
    fn symbol_menu_has_button_semantics_without_visible_text_or_chevron() {
        let menu = MenuButton::new("More", SymbolName::More).option("Open", || {});
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut nodes = vec![];
        let mut list = DisplayList::new();
        let b = Rect::new(0., 0., 24., 24.);
        menu.paint(
            b,
            &mut PaintContext::new(&mut list, &theme, &theme.typography, &mut text)
                .with_accessibility_nodes(&mut nodes),
        );
        assert!(
            nodes
                .iter()
                .any(|n| n.role == AccessibilityRole::Button && n.label.as_deref() == Some("More"))
        );
        assert!(
            !list
                .commands()
                .iter()
                .any(|c| matches!(c, DrawCommand::DrawText { .. }))
        );
        assert_eq!(
            list.commands()
                .iter()
                .filter(|c| matches!(c, DrawCommand::DrawSvg { .. }))
                .count(),
            1
        );
    }
    #[test]
    fn keyboard_menu_dispatch_rejects_disabled_stale_and_repeated_results() {
        let count = Rc::new(Cell::new(0));
        let callback = count.clone();
        let menu = MenuButton::new("More", SymbolName::More)
            .disabled_option("Unavailable")
            .option("Open", move || callback.set(callback.get() + 1));
        let theme = Theme::LIGHT;
        let mut text = TextMeasurer::new();
        let mut c = EventContext::new(&theme, &theme.typography, &mut text);
        let b = Rect::new(0., 0., 24., 24.);
        menu.handle_event(
            b,
            &ViewEvent::KeyboardFocusRequested { bounds: Some(b) },
            &mut c,
        );
        menu.handle_event(
            b,
            &ViewEvent::KeyPressed {
                key: Key::Enter,
                modifiers: KeyModifiers::default(),
            },
            &mut c,
        );
        let request = c
            .take_context_menu_request()
            .expect("keyboard activation opens native menu");
        menu.handle_event(
            b,
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id + 1,
                command_id: Some(2),
            },
            &mut c,
        );
        assert_eq!(count.get(), 0);
        menu.handle_event(
            b,
            &ViewEvent::ContextMenuResult {
                request_id: request.request_id,
                command_id: Some(1),
            },
            &mut c,
        );
        assert_eq!(count.get(), 0);
        menu.handle_event(
            b,
            &ViewEvent::KeyPressed {
                key: Key::Enter,
                modifiers: KeyModifiers::default(),
            },
            &mut c,
        );
        let request = c.take_context_menu_request().unwrap();
        let result = ViewEvent::ContextMenuResult {
            request_id: request.request_id,
            command_id: Some(2),
        };
        menu.handle_event(b, &result, &mut c);
        menu.handle_event(b, &result, &mut c);
        assert_eq!(count.get(), 1);
    }
}

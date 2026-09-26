//! Semantic application commands routed through the focused view hierarchy.

use crate::geometry::Rect;

/// Stable identifier for a responder command.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct CommandId(&'static str);

impl CommandId {
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    #[must_use]
    pub const fn name(self) -> &'static str {
        self.0
    }
}

/// The current responder-chain state of a semantic command.
///
/// Views publish these while painting. Menus and keyboard shortcuts then use
/// the entry nearest the focused view, matching AppKit's command validation.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandStatus {
    pub command: CommandId,
    pub bounds: Rect,
    pub enabled: bool,
    pub checked: bool,
    pub title: Option<String>,
}

impl CommandStatus {
    #[must_use]
    pub const fn new(command: CommandId, bounds: Rect, enabled: bool) -> Self {
        Self {
            command,
            bounds,
            enabled,
            checked: false,
            title: None,
        }
    }

    #[must_use]
    pub const fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }
}

/// Commands shared by application menus, document windows, and editable views.
pub mod standard {
    use super::CommandId;

    pub const NEW: CommandId = CommandId::new("app.new");
    pub const OPEN: CommandId = CommandId::new("app.open");
    pub const CLOSE: CommandId = CommandId::new("app.close");
    pub const SAVE: CommandId = CommandId::new("app.save");
    pub const SAVE_AS: CommandId = CommandId::new("app.save-as");
    pub const REVERT: CommandId = CommandId::new("app.revert");
    pub const UNDO: CommandId = CommandId::new("edit.undo");
    pub const REDO: CommandId = CommandId::new("edit.redo");
    pub const CUT: CommandId = CommandId::new("edit.cut");
    pub const COPY: CommandId = CommandId::new("edit.copy");
    pub const PASTE: CommandId = CommandId::new("edit.paste");
    pub const DELETE: CommandId = CommandId::new("edit.delete");
    pub const SELECT_ALL: CommandId = CommandId::new("edit.select-all");
    pub const FIND: CommandId = CommandId::new("edit.find");
}

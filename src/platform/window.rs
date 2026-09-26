//! プラットフォームウィンドウの共通インターフェースを定義

use crate::accessibility::AccessibilityNode;
use crate::app::WindowId;
use crate::draw_command::DisplayList;
use crate::event::ContextMenuRequest;
use crate::geometry::{Rect, Size};
use crate::platform::event::PlatformEvent;
use crate::renderer::Viewport;
use std::time::Instant;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CursorIcon {
    #[default]
    Default,
    Pointer,
    Text,
    EwResize,
    NsResize,
    NwseResize,
    NeswResize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    pub size: Size,
    pub resizable: bool,
    pub fullscreen: bool,
    pub secure_overlay: bool,
    pub system_modal: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformWindowCommand {
    Open {
        id: WindowId,
        config: WindowConfig,
    },
    Close {
        id: WindowId,
    },
    RequestClose {
        id: WindowId,
    },
    Redraw {
        id: WindowId,
    },
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("ViewKit"),
            size: Size::new(800.0, 600.0),
            resizable: true,
            fullscreen: false,
            secure_overlay: false,
            system_modal: false,
        }
    }
}

pub trait PlatformWindow {
    fn id(&self) -> WindowId;

    fn request_redraw(&self);

    fn set_title(&self, title: &str);

    fn viewport(&self) -> Viewport;

    fn set_cursor(&self, cursor: CursorIcon) {
        let _ = cursor;
    }

    fn show_context_menu(&self, request: &ContextMenuRequest) -> bool {
        let _ = request;
        false
    }

    /// Publishes the complete accessibility snapshot for this window.
    /// Platform bridges may translate it to their native accessibility API.
    fn update_accessibility(&self, nodes: &[AccessibilityNode]) {
        let _ = nodes;
    }
}

pub trait PlatformApplication {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow);

    /// Asks the application whether the platform window may close.
    ///
    /// Closing a window is deliberately separate from terminating the
    /// application. Multi-window backends remove only the accepted window;
    /// [`Self::exit_requested`] remains the application-wide termination path.
    fn should_close_window(&mut self, window: &dyn PlatformWindow) -> bool {
        self.handle_event(PlatformEvent::CloseRequested, window);
        true
    }

    fn take_window_commands(&mut self) -> Vec<PlatformWindowCommand> {
        Vec::new()
    }

    fn handle_platform_message(&mut self, _message: &[u8]) -> bool {
        false
    }

    /// Reopens the application's main window after external activation.
    fn reopen(&mut self) {}

    fn draw(&mut self, window: &dyn PlatformWindow, display_list: &mut DisplayList) -> Rect {
        let _ = display_list;
        window.viewport().logical_bounds()
    }

    fn next_redraw_at(&self, _window: WindowId) -> Option<Instant> {
        None
    }

    fn accessibility_nodes(&self, _window: WindowId) -> &[AccessibilityNode] {
        &[]
    }

    fn reload_appearance(&mut self) -> bool {
        false
    }

    fn interface_scale_factor(&self) -> f64 {
        1.0
    }

    fn exit_requested(&self) -> bool {
        false
    }
}

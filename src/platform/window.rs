//! プラットフォームウィンドウの共通インターフェースを定義

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
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("ViewKit"),
            size: Size::new(800.0, 600.0),
            resizable: true,
            fullscreen: false,
            secure_overlay: false,
        }
    }
}

pub trait PlatformWindow {
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
}

pub trait PlatformApplication {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow);

    fn handle_platform_message(&mut self, _message: &[u8]) -> bool {
        false
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) -> Rect {
        let _ = display_list;

        viewport.logical_bounds()
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        None
    }
}

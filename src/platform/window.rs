//! プラットフォームウィンドウの共通インターフェースを定義

use crate::draw_command::DisplayList;
use crate::geometry::{Rect, Size};
use crate::platform::event::PlatformEvent;
use crate::renderer::Viewport;
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub struct WindowConfig {
    pub title: String,
    pub size: Size,
    pub resizable: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: String::from("ViewKit"),
            size: Size::new(800.0, 600.0),
            resizable: true,
        }
    }
}

pub trait PlatformWindow {
    fn request_redraw(&self);

    fn set_title(&self, title: &str);

    fn viewport(&self) -> Viewport;
}

pub trait PlatformApplication {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow);

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) -> Rect {
        let _ = viewport;
        let _ = display_list;

        viewport.logical_bounds()
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        None
    }
}

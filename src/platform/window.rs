//! プラットフォームウィンドウの共通インターフェースを定義

use crate::draw_command::DisplayList;
use crate::geometry::Size;
use crate::platform::event::PlatformEvent;
use crate::renderer::Viewport;

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

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        let _ = viewport;
        let _ = display_list;
    }
}

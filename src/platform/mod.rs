mod event;
mod font;
mod window;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "mochios")]
pub mod mochios;

pub use event::{ButtonState, PlatformEvent, PointerButton};
pub(crate) use font::{DEFAULT_UI_FONT_FAMILY, load_platform_fonts};
pub use window::{CursorIcon, PlatformApplication, PlatformWindow, WindowConfig};

mod event;
mod window;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "mochios")]
pub mod mochios;

pub use event::{ButtonState, PlatformEvent, PointerButton};
pub use window::{CursorIcon, PlatformApplication, PlatformWindow, WindowConfig};

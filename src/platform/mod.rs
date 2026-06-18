mod event;
mod window;

#[cfg(target_os = "linux")]
pub mod linux;

pub use window::{
    PlatformApplication,
    PlatformWindow,
    WindowConfig,
};
pub use event::{
    ButtonState,
    PlatformEvent,
    PointerButton,
};
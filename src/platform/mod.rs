mod event;
mod font;
mod window;

#[cfg(any(target_os = "linux", target_os = "windows"))]
#[path = "linux/mod.rs"]
mod desktop;

#[cfg(target_os = "linux")]
pub mod linux {
    pub use super::desktop::{
        DesktopBackend as LinuxBackend, DesktopBackendError as LinuxBackendError, SoftwareRenderer,
        SoftwareRendererError,
    };
}

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "mochios")]
pub mod mochios;

pub use event::{ButtonState, PlatformEvent, PointerButton};
pub(crate) use font::{DEFAULT_UI_FONT_FAMILY, load_platform_fonts};
pub use window::{CursorIcon, PlatformApplication, PlatformWindow, WindowConfig};

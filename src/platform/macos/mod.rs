//! macOS向けのバックエンド

pub use super::desktop::{
    DesktopBackend as WindowsBackend, DesktopBackendError as WindowsBackendError, SoftwareRenderer,
    SoftwareRendererError,
};

//! Windows向けのWin32ウィンドウ・ソフトウェア描画バックエンド

pub use super::desktop::{
    DesktopBackend as WindowsBackend, DesktopBackendError as WindowsBackendError, SoftwareRenderer,
    SoftwareRendererError,
};

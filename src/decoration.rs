//! Window chrome shared by the application runtime and platform backends.
use crate::{geometry::Rect, theme::Color};

/// Shared logical dimensions and physical-pixel conversion for window chrome.
pub use mochios_viewkit_gpu_protocol::window_decoration as metrics;

/// Selects system chrome or a ViewKit title bar with system-owned controls.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub enum DecorationMode {
    /// Use the platform's standard title bar (the compatibility default).
    #[default]
    System,
    /// Put application views beside the platform's protected window controls.
    Integrated,
}

/// Decoration properties which require platform negotiation. Ordinary view
/// updates do not change this value or recreate a native window.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct DecorationConfig {
    /// Requested mode, independent of fullscreen and borderless attributes.
    pub mode: DecorationMode,
    /// Requested title bar height in logical pixels; the platform sets a minimum.
    pub height: f32,
    /// Background behind the system controls, resolved from the current theme.
    pub background: Color,
    /// Draws the title bar separator, including the protected control strip.
    pub separator: bool,
    /// Whether the window accepts interactive resize requests.
    pub resizable: bool,
}

/// Window-manager state, reported independently of decoration mode.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub enum WindowState {
    /// A normal floating window.
    #[default]
    Normal,
    /// Hidden in the task switcher or Dock.
    Minimized,
    /// Occupies the available work area.
    Maximized,
}
impl WindowState {
    /// Decodes the existing compositor's state values, rejecting unknown ones.
    pub fn from_raw(value: u32) -> Option<Self> {
        match value {
            0 => Some(Self::Normal),
            1 => Some(Self::Minimized),
            2 => Some(Self::Maximized),
            _ => None,
        }
    }
}

/// Platform-owned title bar geometry in logical client coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct DecorationLayout {
    /// The actual title bar height, after applying platform constraints.
    pub height: f32,
    /// Reserved width at the trailing edge. Application views never occupy it.
    pub controls_width: f32,
    /// Current window state, independent of fullscreen and chrome mode.
    pub state: WindowState,
}
impl DecorationLayout {
    /// Returns the rectangle available to application title bar components.
    pub fn custom_bounds(self, bounds: Rect) -> Rect {
        Rect::new(
            bounds.origin.x,
            bounds.origin.y,
            (bounds.size.width - self.controls_width).max(0.0),
            self.height.min(bounds.size.height),
        )
    }
}

/// A decoration configuration was invalid or rejected by the window system.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum DecorationError {
    /// An explicit System mode cannot contain an application title bar.
    #[error("a custom title bar conflicts with explicit System decoration")]
    ConflictingMode,
    /// Heights must be finite, positive and within platform size limits.
    #[error("title bar height must be finite and between 1 and 4096 logical pixels")]
    InvalidHeight,
    /// This backend does not offer protected integrated decorations.
    #[error("integrated window decorations are unsupported by this backend")]
    Unsupported,
    /// The window system rejected the request.
    #[error("the window system rejected the decoration request")]
    Rejected,
}

/// Which edges participate in an input-authorized resize operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum ResizeEdge {
    /// Left edge.
    Left = 1,
    /// Right edge.
    Right = 2,
    /// Top edge.
    Top = 4,
    /// Bottom edge.
    Bottom = 8,
    /// Top left corner.
    TopLeft = 5,
    /// Top right corner.
    TopRight = 6,
    /// Bottom left corner.
    BottomLeft = 9,
    /// Bottom right corner.
    BottomRight = 10,
}

/// A move or resize bound to the current primary pointer press. Tokens cannot
/// be constructed by application code and expire when event dispatch returns.
#[derive(Debug)]
pub struct WindowInteractionToken<'event> {
    pub(crate) serial: u64,
    pub(crate) request: &'event core::cell::Cell<Option<(u64, WindowInteraction)>>,
}

/// Operation requested by an event handler; not an authorization by itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WindowInteraction {
    /// Move the window from its current position.
    Move,
    /// Resize the indicated edges.
    Resize(ResizeEdge),
}

/// Queues an operation on the token's window during its current input dispatch.
/// Backends independently verify and consume the actual platform serial.
pub fn begin_window_interaction(
    token: WindowInteractionToken<'_>,
    operation: WindowInteraction,
) -> bool {
    if token.serial == 0 || token.request.get().is_some() {
        return false;
    }
    token.request.set(Some((token.serial, operation)));
    true
}

//! アプリ開発者はこのファイルをuseしてください。
//!
//! ```ignore
//! use viewkit::prelude::*;
//! ```

pub use crate::accessibility::{AccessibilityNode, AccessibilityRole};
pub use crate::animation::{
    Animation, AnimationSample, Easing, Interpolate, Transition, interpolate,
};
pub use crate::app::{App, ViewContext, WindowId, WindowOptions};
pub use crate::command::{CommandId, CommandStatus, standard as standard_commands};
pub use crate::components::*;
pub use crate::geometry::{Point, Rect, Size};
pub use crate::image::{ImageData, ImageError};
pub use crate::layout::{
    IntoStackChild, IntoStackChildren, LayoutLength, StackAlignment, StackChild, StackDistribution,
    StackGap, ViewExt,
};
pub use crate::platform::{CursorIcon, Key, KeyModifiers};
pub use crate::runtime::{
    ViewKitError, close_window, key_window, main_window, request_close_key_window,
    request_close_window, request_exit, request_new_window, run,
};
pub use crate::state::{Binding, State};
pub use crate::svg::{SvgData, SvgError};
pub use crate::theme::{Color, CornerRadius, FigmaTokens, ShadowStyle, Theme};
pub use crate::typography::{TextAlignment, TextRole};
pub use crate::view::View;

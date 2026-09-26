//! # ViewKit
//!
//! ```no_run
//! use viewkit::app::{
//!     App,
//!     ViewContext,
//!     WindowOptions,
//! };
//! use viewkit::components::Text;
//!
//! struct HelloApp;
//!
//! impl App for HelloApp {
//!     type Body = Text;
//!
//!     fn new() -> Self {
//!         Self
//!     }
//!
//!     fn window(&self) -> WindowOptions {
//!         WindowOptions::new("Hello, ViewKit")
//!             .resizable(true)
//!     }
//!
//!     fn body(
//!         &self,
//!         _context: &ViewContext,
//!     ) -> Self::Body {
//!         Text::new("Hello, ViewKit!")
//!     }
//! }
//!
//! fn main() -> Result<(), viewkit::ViewKitError> {
//!     viewkit::run::<HelloApp>()
//! }
//! ```
//!
//! ViewKitは、mochiOS、LinuxおよびWindowsで動作するGUIフレームワークです。
//!
//! 主にKome言語からの利用を想定していますが、Rustから直接利用することもできます。

use crate::geometry::Size;
use crate::renderer::Viewport;
use crate::theme::Theme;
use crate::view::View;

/// Stable identity for a window owned by a running application.
///
/// The initial compatibility window always uses [`WindowId::PRIMARY`]. New
/// windows receive their own IDs from the application runtime.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct WindowId(u64);

impl WindowId {
    pub const PRIMARY: Self = Self(1);

    #[must_use]
    pub const fn from_raw(raw: u64) -> Option<Self> {
        if raw == 0 { None } else { Some(Self(raw)) }
    }

    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// アプリケーションウィンドウの初期設定
///
/// ウィンドウのタイトル、初期サイズ、サイズ変更の可否を指定します。
/// この設定はアプリケーションの起動時にプラットフォームバックエンドへ渡されます。
#[derive(Clone, Debug, PartialEq)]
pub struct WindowOptions {
    pub(crate) title: String,
    pub(crate) size: Size,
    pub(crate) resizable: bool,
    pub(crate) fullscreen: bool,
    pub(crate) secure_overlay: bool,
    pub(crate) system_modal: bool,
}

impl WindowOptions {
    /// 指定されたタイトルでウィンドウ設定を作成します。
    ///
    /// 初期サイズはFigma layout foundationから取得し、サイズ変更は有効です。
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        let layout = Theme::current().layout;
        Self {
            title: title.into(),
            size: Size::new(layout.standard_window_width, layout.standard_window_height),
            resizable: true,
            fullscreen: false,
            secure_overlay: false,
            system_modal: false,
        }
    }

    /// ウィンドウの初期サイズを論理ピクセル単位で設定します。
    #[must_use]
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Size::new(width, height);
        self
    }

    /// Figma layout foundationに基づくコンパクトな初期サイズを使用します。
    #[must_use]
    pub fn compact(mut self) -> Self {
        let layout = Theme::current().layout;
        self.size = Size::new(layout.compact_window_width, layout.compact_window_height);
        self
    }

    /// ユーザーによるウィンドウサイズの変更を許可するか設定します。
    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// ウィンドウを全画面として扱うか設定します。
    #[must_use]
    pub fn fullscreen(mut self, fullscreen: bool) -> Self {
        self.fullscreen = fullscreen;
        if fullscreen {
            self.system_modal = false;
        }
        self
    }

    /// mochiOSのsecure overlayとして表示します。
    ///
    /// この指定には`window.secure-overlay` capabilityが必要です。
    #[must_use]
    pub fn secure_overlay(mut self, secure_overlay: bool) -> Self {
        self.secure_overlay = secure_overlay;
        if secure_overlay {
            self.fullscreen = true;
            self.system_modal = false;
        }
        self
    }

    /// mochiOSのシステムモーダルウィンドウとして表示します。
    ///
    /// システム所有のパネル向けで、`window.overlay` capabilityが必要です。
    #[must_use]
    pub fn system_modal(mut self, system_modal: bool) -> Self {
        self.system_modal = system_modal;
        if system_modal {
            self.secure_overlay = false;
            self.fullscreen = false;
        }
        self
    }

    /// ウィンドウのタイトルを返します。
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// ウィンドウの初期サイズを返します。
    ///
    /// 戻り値は論理ピクセル単位です。
    #[must_use]
    pub const fn initial_size(&self) -> Size {
        self.size
    }

    /// ユーザーによるウィンドウサイズの変更が許可されているか返します。
    #[must_use]
    pub const fn is_resizable(&self) -> bool {
        self.resizable
    }

    /// 全画面表示を要求しているか返します。
    #[must_use]
    pub const fn is_fullscreen(&self) -> bool {
        self.fullscreen
    }

    #[must_use]
    pub const fn is_secure_overlay(&self) -> bool {
        self.secure_overlay
    }

    #[must_use]
    pub const fn is_system_modal(&self) -> bool {
        self.system_modal
    }
}

/// Actionの処理中にアプリケーションから
/// ViewKitランタイムへ要求を送るためのコンテキストです。
#[derive(Debug, Default)]
pub struct AppContext {
    rebuild_requested: bool,
    redraw_requested: bool,
    exit_requested: bool,
}

#[allow(unused)]
impl AppContext {
    pub(crate) const fn new() -> Self {
        Self {
            rebuild_requested: false,
            redraw_requested: false,
            exit_requested: false,
        }
    }

    /// Viewツリーの再構築を要求します。
    pub fn request_rebuild(&mut self) {
        self.rebuild_requested = true;
        self.redraw_requested = true;
    }

    /// 現在のViewツリーの再描画を要求します。
    pub fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }

    /// アプリケーションの終了を要求します。
    pub fn exit(&mut self) {
        self.exit_requested = true;
    }

    pub(crate) fn take_rebuild_requested(&mut self) -> bool {
        std::mem::take(&mut self.rebuild_requested)
    }

    pub(crate) fn take_redraw_requested(&mut self) -> bool {
        std::mem::take(&mut self.redraw_requested)
    }

    pub(crate) fn take_exit_requested(&mut self) -> bool {
        std::mem::take(&mut self.exit_requested)
    }
}

impl Default for WindowOptions {
    fn default() -> Self {
        Self::new("ViewKit")
    }
}

/// Viewツリーを構築するときに利用できるコンテキストです。
///
/// 現在のウィンドウサイズや表示倍率など、Viewの構築に必要な
/// 実行環境の情報を提供します。
///
/// `ViewContext`はViewKitランタイムによって生成されます。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewContext {
    viewport: Viewport,
}

impl ViewContext {
    /// Viewコンテキストを作成します。
    ///
    /// この関数はViewKitランタイムからのみ使用されます。
    pub(crate) const fn new(viewport: Viewport) -> Self {
        Self { viewport }
    }

    /// ウィンドウの論理サイズを返します。
    ///
    /// 戻り値は表示倍率適用前の論理ピクセル単位です。
    #[must_use]
    pub const fn size(&self) -> Size {
        self.viewport.logical_size
    }

    /// ウィンドウの表示倍率を返します。
    ///
    /// たとえば、論理ピクセルと物理ピクセルが同じ場合は`1.0`です。
    #[must_use]
    pub const fn scale_factor(&self) -> f64 {
        self.viewport.scale_factor
    }

    /// 現在のViewportを返します。
    ///
    /// ViewKit内部のレイアウト処理およびイベント配送で使用されます。
    #[allow(unused)]
    pub(crate) const fn viewport(&self) -> Viewport {
        self.viewport
    }
}

/// ViewKitアプリケーションを定義するトレイトです。
///
/// アプリケーションは[`new`](App::new)で初期状態を作成し、
/// [`body`](App::body)で表示するViewツリーを宣言します。
///
/// プラットフォーム固有のイベントループ、レイアウト、描画処理は
/// ViewKitによって管理されます。
pub trait App: Sized + 'static {
    /// アプリケーションが構築するルートViewの型です。
    type Body: View + 'static;

    /// アプリケーションの初期状態を作成します。
    fn new() -> Self;

    /// アプリケーションウィンドウの初期設定を返します。
    fn window(&self) -> WindowOptions {
        WindowOptions::default()
    }

    /// Returns the configuration for a specific application window.
    ///
    /// Existing single-window apps can continue implementing [`Self::window`].
    fn window_for(&self, _window: WindowId) -> WindowOptions {
        self.window()
    }

    /// 現在のアプリケーション状態からルートViewを構築します。
    fn body(&self, context: &ViewContext) -> Self::Body;

    /// Builds the view hierarchy for a specific application window.
    ///
    /// Existing single-window apps can continue implementing [`Self::body`].
    fn body_for(&self, _window: WindowId, context: &ViewContext) -> Self::Body {
        self.body(context)
    }

    /// Platform固有の補助messageを処理します。
    fn handle_platform_message(&mut self, _message: &[u8]) -> bool {
        false
    }

    /// ウィンドウを閉じる要求を受け入れるか返します。
    ///
    /// 未保存のDocumentなどがあるアプリケーションは`false`を返し、確認UIを
    /// 表示できます。デフォルトは従来どおりWindowを閉じます。
    fn close_requested(&mut self) -> bool {
        true
    }

    /// Asks whether a specific application window may close.
    ///
    /// Returning `false` keeps that window alive, allowing a document or
    /// controller to present an asynchronous save confirmation.
    fn close_requested_for_window(&mut self, _window: WindowId) -> bool {
        self.close_requested()
    }

    /// Called when the user activates a running application with no windows.
    fn reopened(&mut self) {}

    /// Called after the persisted system appearance has changed.
    fn appearance_changed(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::WindowOptions;

    #[test]
    fn exclusive_system_window_roles_follow_the_last_builder_call() {
        let modal = WindowOptions::default()
            .secure_overlay(true)
            .system_modal(true);
        assert!(modal.is_system_modal());
        assert!(!modal.is_secure_overlay());
        assert!(!modal.is_fullscreen());

        let fullscreen = WindowOptions::default()
            .system_modal(true)
            .fullscreen(true);
        assert!(fullscreen.is_fullscreen());
        assert!(!fullscreen.is_system_modal());
    }
}

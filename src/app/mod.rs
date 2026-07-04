//! # ViewKit
//!
//! ```no_run
//! use viewkit::app::{App, ViewContext, WindowOptions};
//! use viewkit::components::Text;
//! use viewkit::view::View;
//!
//! struct HelloApp;
//!
//! impl App for HelloApp {
//!     fn new() -> Self {
//!         Self
//!     }
//!
//!     fn window(&self) -> WindowOptions {
//!         WindowOptions::new("Hello, ViewKit")
//!             .size(800.0, 600.0)
//!             .resizable(true)
//!     }
//!
//!     fn body(&self, _context: &ViewContext) -> Box<dyn View + 'static> {
//!         Box::new(Text::new("Hello, ViewKit!"))
//!     }
//! }
//!
//! fn main() -> Result<(), viewkit::ViewKitError> {
//!     viewkit::run::<HelloApp>()
//! }
//! ```
//!
//! ViewKitは、mochiOSおよびLinuxで動作するGUIフレームワークです。
//!
//! 主にKome言語からの利用を想定していますが、Rustから直接利用することもできます。

mod runtime;

use crate::geometry::Size;
use crate::renderer::Viewport;
use crate::view::View;

/// アプリケーションウィンドウの初期設定
///
/// ウィンドウのタイトル、初期サイズ、サイズ変更の可否を指定します。
/// この設定はアプリケーションの起動時にプラットフォームバックエンドへ渡されます。
#[derive(Clone, Debug, PartialEq)]
pub struct WindowOptions {
    pub(crate) title: String,
    pub(crate) size: Size,
    pub(crate) resizable: bool,
}

impl WindowOptions {
    /// 指定されたタイトルでウィンドウ設定を作成します。
    ///
    /// 初期サイズは800×600論理ピクセルで、サイズ変更は有効です。
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size: Size::new(800.0, 600.0),
            resizable: true,
        }
    }

    /// ウィンドウの初期サイズを論理ピクセル単位で設定します。
    #[must_use]
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.size = Size::new(width, height);
        self
    }

    /// ユーザーによるウィンドウサイズの変更を許可するか設定します。
    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
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
    pub(crate) const fn viewport(&self) -> Viewport {
        self.viewport
    }
}

/// ViewKitアプリケーションを定義するトレイトです。
///
/// アプリケーションは[`new`](App::new)で初期状態を作成し、
/// [`body`](App::body)で表示するViewツリーを構築します。
///
/// プラットフォーム固有のイベントループ、描画処理、再描画要求は
/// ViewKitランタイムによって管理されます。
pub trait App: Sized + 'static {
    /// アプリケーションの初期状態を作成します。
    fn new() -> Self;

    /// アプリケーションウィンドウの設定を返します。
    ///
    /// 実装を省略した場合は、タイトルが`ViewKit`、初期サイズが
    /// 800×600論理ピクセルのウィンドウが作成されます。
    fn window(&self) -> WindowOptions {
        WindowOptions::default()
    }

    /// 現在のアプリケーション状態からViewツリーを構築します。
    ///
    /// この関数は、初回描画や状態変更後の再描画時に
    /// ViewKitランタイムから呼び出されます。
    ///
    /// 返されるViewツリーは、アプリケーション自身への参照を
    /// 保持しない所有済みの値である必要があります。
    fn body(&self, context: &ViewContext) -> Box<dyn View + 'static>;
}

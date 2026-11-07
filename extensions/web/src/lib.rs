#![deny(missing_docs)]
//! ViewKitへWebコンテンツを配置する追加モジュールです。
//!
//! セッション、ページ、表示を分離します。ViewKit本体は本クレートへ依存しません。
//! **ネイティブWebKitアダプターは未接続です。現在の公開コンストラクターは
//! [`WebError::Unavailable`]を返し、空のページや成功した読み込みを捏造しません。**
//!
//! ```rust,no_run
//! use viewkit_web::{WebSession, WebSessionOptions, WebView};
//! let session = WebSession::new(WebSessionOptions::default())?;
//! let page = session.create_page()?;
//! let navigation = page.load_url("https://example.com")?;
//! let view = WebView::new(page.clone())?;
//! # Ok::<(), viewkit_web::WebError>(())
//! ```
//!
//! APIは未リリースの提案実装です。Kome向けABIはまだ公開していません。
//! @since Unreleased
mod backend;
mod page;
mod session;
mod web_view;

pub use page::{WebPage, WebSubscription};
pub use session::WebSession;
use std::fmt;
pub use web_view::WebView;

/// Web操作の受付時または非同期実行中の失敗です。
///
/// エラーはCapabilityの許可やTLS検証を迂回しません。
/// @since Unreleased
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebError {
    /// 使用できるネイティブエンジンが接続されていません。
    Unavailable,
    /// 接続先が要求された機能を実装していません。
    Unsupported,
    /// URLが絶対URLとして解析できません。
    InvalidUrl,
    /// 倍率が有限の正数ではありません。
    InvalidZoom,
    /// ページはすでに閉じられています。
    Closed,
    /// ページのプロセスが終了しました。新しいページを作成してください。
    ProcessExited,
    /// バックエンド通知が上限を超えたためセッションを停止しました。
    EventQueueOverflow,
    /// UI通知登録または識別子の上限に達しました。
    Capacity,
    /// 戻る・進むなどの操作を現在の状態で実行できません。
    ActionUnavailable,
    /// 同一ページの表示オブジェクトがすでに存在します。
    AlreadyPresented,
    /// エンジンから報告された失敗です。
    Backend(String),
}
impl fmt::Display for WebError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Backend(message) => f.write_str(message),
            _ => f.write_str(match self {
                Self::Unavailable => "The web engine is not connected.",
                Self::Unsupported => "The web engine does not support this operation.",
                Self::InvalidUrl => "An absolute URL is required.",
                Self::InvalidZoom => "Zoom must be positive and finite.",
                Self::Closed => "The web page is closed.",
                Self::ProcessExited => "The web page process has exited.",
                Self::EventQueueOverflow => "The web event queue exceeded its capacity.",
                Self::Capacity => "The web integration reached its capacity.",
                Self::ActionUnavailable => "The requested page action is unavailable.",
                Self::AlreadyPresented => "This web page already has a presentation.",
                Self::Backend(_) => unreachable!(),
            }),
        }
    }
}
impl std::error::Error for WebError {}

/// Webサイトのデータを保持する方法です。
///
/// 初版はメモリ内の一時セッションだけを定義します。永続保存の対応前に
/// 保存成功を返したり、アプリ間でCookieを暗黙に共有したりしません。
/// @since Unreleased
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebStorage {
    /// セッション破棄後にWebサイトデータを保持しません。
    #[default]
    Ephemeral,
}

/// Webセッションの作成条件です。設定は作成後に変更しません。
///
/// ```rust
/// let options = viewkit_web::WebSessionOptions::default();
/// assert_eq!(options.storage, viewkit_web::WebStorage::Ephemeral);
/// ```
/// @since Unreleased
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct WebSessionOptions {
    /// CookieなどのWebサイトデータの保存方式です。OSの権限は付与しません。
    pub storage: WebStorage,
}

/// 一つのページで再利用されない読み込み要求の識別子です。
///
/// リダイレクト中も同じ値を使用します。値は成功やTLS検証済みを意味しません。
/// @since Unreleased
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NavigationId(pub(crate) u64);
impl NavigationId {
    /// 診断やログ用の数値を返します。
    ///
    /// ## Returns
    /// ページ内で一意な0以外の値です。他ページの値と比較しないでください。
    /// @since Unreleased
    pub fn get(self) -> u64 {
        self.0
    }
}

/// エンジンの通知に基づく読み込み状態です。
/// @since Unreleased
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebLoadState {
    /// まだ読み込みを要求していません。
    #[default]
    Idle,
    /// 要求は受理されましたが、開始通知はまだ届いていません。
    Requested,
    /// エンジンが読み込みを開始しました。
    Loading,
    /// メイン文書のコミット後、エンジンが完了を通知しました。
    Ready,
    /// エンジンが読み込みの中止を通知しました。
    Stopped,
    /// 要求された読み込みが失敗しました。
    Failed(WebError),
    /// ページのプロセスが終了しました。
    Terminated,
    /// ページが閉じられています。
    Closed,
}

/// ページ状態の読み取り専用スナップショットです。
///
/// コピーを変更してもページは変更されません。実際の状態変更時にViewKitへ
/// 変更通知を発行するため、アプリによる定期ポーリングは不要です。
/// @since Unreleased
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WebPageState {
    /// 現在要求している読み込みです。過去の通知の判別に使用します。
    pub navigation: Option<NavigationId>,
    /// エンジンがコミットしたメイン文書のURLです。要求先と区別します。
    pub url: Option<String>,
    /// エンジンが報告した現在の文書のタイトルです。
    pub title: String,
    /// 現在の要求の処理状態です。
    pub load_state: WebLoadState,
    /// エンジンが報告した「戻る」の可否です。
    pub can_go_back: bool,
    /// エンジンが報告した「進む」の可否です。
    pub can_go_forward: bool,
}

/// UIスレッド上で配送するページ通知です。
///
/// コールバック内で次の読み込みや購読解除を行えます。コールバック中に
/// バックエンドを再入させず、新しい要求はキューへ追加します。
/// @since Unreleased
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WebEvent {
    /// 読み込み・URL・タイトル・履歴可否などの状態が変化しました。
    StateChanged(WebPageState),
    /// ページのプロセスが終了しました。
    ProcessExited,
}

#[cfg(test)]
mod tests;

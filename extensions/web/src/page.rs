use crate::{
    NavigationId, WebError, WebEvent, WebLoadState, WebPageState,
    backend::{Command, Event, PageDriver},
    session::SessionInner,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};
use viewkit::state::State;
type Observer = Rc<dyn Fn(WebEvent)>;

/// 表示の再構築と独立して保持するWebページです。
///
/// Cloneは同じページを参照し、読み込みや履歴を複製しません。最後の参照が
/// 破棄されるとバックエンドへ終了を要求します。UIスレッド専用です。
/// ```rust,compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<viewkit_web::WebPage>();
/// ```
/// @since Unreleased
#[derive(Clone)]
pub struct WebPage {
    pub(crate) inner: Rc<PageInner>,
}
pub(crate) struct PageInner {
    pub session: Rc<SessionInner>,
    pub driver: Rc<dyn PageDriver>,
    id: u64,
    state: State<WebPageState>,
    committed: Cell<Option<NavigationId>>,
    next_navigation: Cell<u64>,
    closed: Cell<bool>,
    driver_closed: Cell<bool>,
    pending: Cell<bool>,
    process_exit_pending: Cell<bool>,
    pub presented: Cell<bool>,
    next_observer: Cell<u64>,
    observers: RefCell<BTreeMap<u64, Weak<dyn Fn(WebEvent)>>>,
}
impl WebPage {
    pub(crate) fn from_driver(
        session: Rc<SessionInner>,
        id: u64,
        driver: Rc<dyn PageDriver>,
    ) -> Self {
        Self {
            inner: Rc::new(PageInner {
                session,
                driver,
                id,
                state: State::new(WebPageState::default()),
                committed: Cell::new(None),
                next_navigation: Cell::new(1),
                closed: Cell::new(false),
                driver_closed: Cell::new(false),
                pending: Cell::new(false),
                process_exit_pending: Cell::new(false),
                presented: Cell::new(false),
                next_observer: Cell::new(1),
                observers: RefCell::default(),
            }),
        }
    }
    /// ページの現在の状態を複製して返します。エンジン処理は実行しません。
    ///
    /// ## Returns
    /// URLはコミット済み文書の値です。読み込み要求先から推測しません。
    /// @since Unreleased
    pub fn state(&self) -> WebPageState {
        self.inner.state.get()
    }
    /// 絶対URLへの読み込みを非同期に要求します。
    ///
    /// ## Parameters
    /// - `url`: 検索語ではなく絶対URLを指定します。ファイルのアクセス権は付与しません。
    /// ## Returns
    /// 受理した要求の識別子です。読み込み完了や安全な接続を意味しません。
    /// ## Errors
    /// 不正なURL、閉じたページ、停止したセッション、エンジンによる拒否を返します。
    /// ## Example
    /// ```rust,no_run
    /// # fn example(page: &viewkit_web::WebPage) -> Result<(), viewkit_web::WebError> {
    /// let navigation = page.load_url("https://example.com")?;
    /// # Ok(()) }
    /// ```
    /// @since Unreleased
    pub fn load_url(&self, url: &str) -> Result<NavigationId, WebError> {
        let url = url::Url::parse(url).map_err(|_| WebError::InvalidUrl)?;
        self.navigate(|id| Command::Load(id, url.into()))
    }
    /// エンジンが保持する一つ前の履歴項目への移動を要求します。
    ///
    /// ## Returns
    /// 受理した履歴移動の読み込み要求IDです。完了は通知で確認します。
    /// ## Errors
    /// 戻れない場合はActionUnavailable、それ以外の拒否はWebErrorで返します。
    /// @since Unreleased
    pub fn go_back(&self) -> Result<NavigationId, WebError> {
        self.inner.check()?;
        if !self.state().can_go_back {
            return Err(WebError::ActionUnavailable);
        }
        self.navigate(Command::Back)
    }
    /// エンジンが保持する一つ次の履歴項目への移動を要求します。
    ///
    /// ## Returns
    /// 受理した履歴移動の読み込み要求IDです。完了は通知で確認します。
    /// ## Errors
    /// 進めない場合はActionUnavailable、それ以外の拒否はWebErrorで返します。
    /// @since Unreleased
    pub fn go_forward(&self) -> Result<NavigationId, WebError> {
        self.inner.check()?;
        if !self.state().can_go_forward {
            return Err(WebError::ActionUnavailable);
        }
        self.navigate(Command::Forward)
    }
    /// 現在の文書の再読み込みを要求します。
    ///
    /// ## Returns
    /// 受理した再読み込み要求のIDです。完了は通知で確認します。
    /// ## Errors
    /// 文書がない場合はActionUnavailable、エンジンの拒否などはWebErrorで返します。
    /// @since Unreleased
    pub fn reload(&self) -> Result<NavigationId, WebError> {
        self.inner.check()?;
        if self.state().url.is_none() {
            return Err(WebError::ActionUnavailable);
        }
        self.navigate(Command::Reload)
    }
    /// 現在の要求の読み込み中止を要求します。中止完了は状態通知で確認します。
    ///
    /// ## Returns
    /// 要求を受理した場合はOkを返します。中止完了を意味しません。
    /// ## Errors
    /// 読み込み中でない場合はActionUnavailable、エンジンの拒否などはWebErrorで返します。
    /// @since Unreleased
    pub fn stop(&self) -> Result<(), WebError> {
        self.inner.check()?;
        let state = self.state();
        if !matches!(
            state.load_state,
            WebLoadState::Requested | WebLoadState::Loading
        ) {
            return Err(WebError::ActionUnavailable);
        }
        self.inner.driver.enqueue(Command::Stop(
            state.navigation.ok_or(WebError::ActionUnavailable)?,
        ))?;
        self.inner.session.sink.wake.notify();
        Ok(())
    }
    /// 文書の表示倍率の変更を要求します。
    ///
    /// ## Parameters
    /// - `scale`: 1.0を標準倍率とする有限の正数です。
    /// ## Returns
    /// 要求を受理した場合はOkを返します。倍率適用の完了を意味しません。
    /// ## Errors
    /// InvalidZoom、未対応ならUnsupported、エンジンの拒否などはWebErrorで返します。
    /// @since Unreleased
    pub fn set_zoom(&self, scale: f64) -> Result<(), WebError> {
        self.inner.check()?;
        if !scale.is_finite() || scale <= 0.0 {
            return Err(WebError::InvalidZoom);
        }
        self.inner.driver.enqueue(Command::Zoom(scale))?;
        self.inner.session.sink.wake.notify();
        Ok(())
    }
    /// UIイベントループでページ通知を受け取る購読を作成します。
    ///
    /// ## Parameters
    /// - `callback`: 通知処理です。その中で新しい読み込みや購読解除を要求できます。
    /// ## Returns
    /// 保持している間だけ有効な購読です。現在値はstateで取得してください。
    /// ## Errors
    /// 閉じたページ、停止したセッション、識別子の上限をWebErrorで返します。
    /// ## Example
    /// ```rust,no_run
    /// # fn example(page: &viewkit_web::WebPage) -> Result<(), viewkit_web::WebError> {
    /// let subscription = page.subscribe(|event| println!("{event:?}"))?;
    /// // subscriptionをアプリの状態に保持します。
    /// # Ok(()) }
    /// ```
    /// @since Unreleased
    pub fn subscribe(
        &self,
        callback: impl Fn(WebEvent) + 'static,
    ) -> Result<WebSubscription, WebError> {
        self.inner.check()?;
        let id = self.inner.next_observer.get();
        let next = id.checked_add(1).ok_or(WebError::Capacity)?;
        self.inner.next_observer.set(next);
        let callback: Observer = Rc::new(callback);
        self.inner
            .observers
            .borrow_mut()
            .insert(id, Rc::downgrade(&callback));
        Ok(WebSubscription {
            page: Rc::downgrade(&self.inner),
            id,
            _callback: callback,
        })
    }
    /// ページを明示的に閉じます。繰り返し呼び出しても追加の終了要求は送りません。
    ///
    /// 保持中のWebViewは入力・描画を停止します。以後アプリへの通知は配送せず、
    /// 遅れて届いたエンジン通知も破棄します。ブロックしてプロセス終了を待ちません。
    /// @since Unreleased
    pub fn close(&self) {
        self.inner.close();
    }
    fn navigate(
        &self,
        command: impl FnOnce(NavigationId) -> Command,
    ) -> Result<NavigationId, WebError> {
        self.inner.check()?;
        let id = self.inner.next_navigation.get();
        let next = id.checked_add(1).ok_or(WebError::Capacity)?;
        // IDs are consumed even on rejection, so delayed native responses can
        // never be mistaken for a later accepted command.
        self.inner.next_navigation.set(next);
        let navigation = NavigationId(id);
        self.inner.driver.enqueue(command(navigation))?;
        let mut state = self.state();
        state.navigation = Some(navigation);
        state.load_state = WebLoadState::Requested;
        self.inner.update(state);
        self.inner.session.sink.wake.notify();
        Ok(navigation)
    }
}
impl PageInner {
    pub fn check(&self) -> Result<(), WebError> {
        if self.closed.get() {
            Err(WebError::Closed)
        } else if self.session.failed.get() {
            Err(WebError::EventQueueOverflow)
        } else if self
            .state
            .with(|s| s.load_state == WebLoadState::Terminated)
        {
            Err(WebError::ProcessExited)
        } else {
            Ok(())
        }
    }
    fn update(&self, state: WebPageState) {
        if self.state.set_if_changed(state) {
            self.pending.set(true);
        }
    }
    fn close_driver(&self) {
        if !self.driver_closed.replace(true) {
            self.driver.close();
        }
    }
    fn close(&self) {
        if self.closed.replace(true) {
            return;
        }
        self.close_driver();
        self.observers.borrow_mut().clear();
        self.pending.set(false);
        self.session.pages.borrow_mut().remove(&self.id);
        let mut state = self.state.get();
        state.load_state = WebLoadState::Closed;
        state.can_go_back = false;
        state.can_go_forward = false;
        self.state.set_if_changed(state);
    }
    pub fn queue_failed(&self) {
        if self.closed.get() {
            return;
        }
        self.close_driver();
        let mut state = self.state.get();
        state.load_state = WebLoadState::Failed(WebError::EventQueueOverflow);
        state.can_go_back = false;
        state.can_go_forward = false;
        self.update(state);
    }
    pub fn receive(&self, event: Event) {
        if self.check().is_err() {
            return;
        }
        let mut state = self.state.get();
        if let Event::ProcessExited = event {
            self.close_driver();
            state.load_state = WebLoadState::Terminated;
            state.can_go_back = false;
            state.can_go_forward = false;
            self.process_exit_pending.set(true);
        } else {
            let id = match &event {
                Event::Started(id)
                | Event::Committed { navigation: id, .. }
                | Event::Finished(id)
                | Event::Failed(id, _)
                | Event::Stopped(id)
                | Event::Title(id, _) => *id,
                Event::ProcessExited => unreachable!(),
            };
            if state.navigation != Some(id) {
                return;
            }
            let loading = matches!(
                state.load_state,
                WebLoadState::Requested | WebLoadState::Loading
            );
            match event {
                Event::Started(_) if loading => state.load_state = WebLoadState::Loading,
                Event::Committed {
                    url,
                    title,
                    back,
                    forward,
                    ..
                } if loading && self.committed.get() != Some(id) => {
                    // Invalid engine data never becomes a trusted committed URL.
                    if url::Url::parse(&url).is_err() {
                        state.load_state = WebLoadState::Failed(WebError::Backend(
                            "The engine reported an invalid committed URL.".into(),
                        ));
                        self.update(state);
                        return;
                    }
                    self.committed.set(Some(id));
                    state.url = Some(url);
                    state.title = title;
                    state.can_go_back = back;
                    state.can_go_forward = forward;
                    state.load_state = WebLoadState::Loading;
                }
                Event::Finished(_) if loading && self.committed.get() == Some(id) => {
                    state.load_state = WebLoadState::Ready
                }
                Event::Failed(_, error) if loading => {
                    state.load_state = WebLoadState::Failed(error)
                }
                Event::Stopped(_) if loading => state.load_state = WebLoadState::Stopped,
                Event::Title(_, title) if self.committed.get() == Some(id) => state.title = title,
                _ => return,
            }
        }
        self.update(state);
    }
    pub fn flush(&self) {
        if self.closed.get() {
            return;
        }
        let changed = self.pending.replace(false);
        let exited = self.process_exit_pending.replace(false);
        if changed {
            self.emit(WebEvent::StateChanged(self.state.get()));
        }
        if exited && !self.closed.get() {
            self.emit(WebEvent::ProcessExited);
        }
    }
    fn emit(&self, event: WebEvent) {
        let observers: Vec<_> = self
            .observers
            .borrow()
            .iter()
            .filter_map(|(id, f)| f.upgrade().map(|f| (*id, f)))
            .collect();
        for (id, callback) in observers {
            if self.closed.get() {
                break;
            }
            let subscribed = self.observers.borrow().contains_key(&id);
            if subscribed {
                callback(event.clone());
            }
        }
    }
}
impl Drop for PageInner {
    fn drop(&mut self) {
        self.close_driver();
        self.session.pages.borrow_mut().remove(&self.id);
    }
}

/// Webページ通知の購読を保持するUIスレッド専用のハンドルです。
///
/// Dropで直ちに購読解除します。ページの寿命は延長しません。コールバックが
/// 自分の購読や別の購読を解除しても、その後の通知は配送しません。
/// ```rust,compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<viewkit_web::WebSubscription>();
/// ```
/// @since Unreleased
pub struct WebSubscription {
    page: Weak<PageInner>,
    id: u64,
    _callback: Observer,
}
impl Drop for WebSubscription {
    fn drop(&mut self) {
        if let Some(page) = self.page.upgrade() {
            page.observers.borrow_mut().remove(&self.id);
        }
    }
}

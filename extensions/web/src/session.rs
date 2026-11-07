use crate::{
    WebError, WebPage, WebSessionOptions,
    backend::{self, EVENT_BUDGET, EventSink, Inbox, SessionDriver},
    page::PageInner,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
    sync::{Arc, Mutex},
};
use viewkit::dispatch::UiSubscription;

/// Cookieなどを共有するページ群の所有者です。
///
/// Cloneは同じセッションを共有します。ページが存在する間はセッションも
/// 保持されます。UIスレッド専用で、Send/Syncではありません。
/// ```rust,compile_fail
/// fn require_send<T: Send>() {}
/// require_send::<viewkit_web::WebSession>();
/// ```
/// @since Unreleased
#[derive(Clone)]
pub struct WebSession {
    pub(crate) inner: Rc<SessionInner>,
}
pub(crate) struct SessionInner {
    pub driver: Rc<dyn SessionDriver>,
    pub sink: EventSink,
    pub pages: RefCell<BTreeMap<u64, Weak<PageInner>>>,
    next_page: Cell<u64>,
    pub failed: Cell<bool>,
    _dispatch: UiSubscription,
}
impl WebSession {
    /// 指定した条件で独立したWebセッションを作成します。
    ///
    /// ## Parameters
    /// - `options`: 保存方式などの作成条件です。アプリのOS権限は変更しません。
    /// ## Returns
    /// 接続できたエンジンを所有するセッションを返します。
    /// ## Errors
    /// - [`WebError::Unavailable`]: 現在はネイティブアダプター未接続のため返します。
    /// - [`WebError::Capacity`]: UI通知登録の上限に達した場合に返します。
    /// ## Example
    /// ```rust,no_run
    /// let session = viewkit_web::WebSession::new(Default::default())?;
    /// # Ok::<(), viewkit_web::WebError>(())
    /// ```
    /// @since Unreleased
    pub fn new(options: WebSessionOptions) -> Result<Self, WebError> {
        Self::start(options, backend::connect)
    }
    pub(crate) fn start(
        options: WebSessionOptions,
        connect: impl FnOnce(&WebSessionOptions, EventSink) -> Result<Rc<dyn SessionDriver>, WebError>,
    ) -> Result<Self, WebError> {
        let slot: Rc<RefCell<Weak<SessionInner>>> = Rc::default();
        let weak = slot.clone();
        let subscription = UiSubscription::new(move || {
            let session = weak.borrow().upgrade();
            if let Some(session) = session {
                session.dispatch();
            }
        })
        .map_err(|_| WebError::Capacity)?;
        let sink = EventSink {
            queue: Arc::new(Mutex::new(Inbox::default())),
            wake: subscription.notifier(),
        };
        let driver = connect(&options, sink.clone())?;
        let inner = Rc::new(SessionInner {
            driver,
            sink,
            pages: RefCell::default(),
            next_page: Cell::new(1),
            failed: Cell::new(false),
            _dispatch: subscription,
        });
        *slot.borrow_mut() = Rc::downgrade(&inner);
        Ok(Self { inner })
    }
    /// 履歴と表示状態を共有しない新しいページを作成します。
    ///
    /// ## Returns
    /// このセッションを保持するページを返します。URLはまだ読み込みません。
    /// ## Errors
    /// エンジンの作成失敗、識別子の上限、セッション停止を返します。
    /// ## Example
    /// ```rust,no_run
    /// # fn example(session: &viewkit_web::WebSession) -> Result<(), viewkit_web::WebError> {
    /// let page = session.create_page()?;
    /// # Ok(()) }
    /// ```
    /// @since Unreleased
    pub fn create_page(&self) -> Result<WebPage, WebError> {
        if self.inner.failed.get() {
            return Err(WebError::EventQueueOverflow);
        }
        let id = self.inner.next_page.get();
        let next = id.checked_add(1).ok_or(WebError::Capacity)?;
        self.inner.next_page.set(next);
        let driver = self.inner.driver.create_page(id)?;
        let page = WebPage::from_driver(self.inner.clone(), id, driver);
        self.inner
            .pages
            .borrow_mut()
            .insert(id, Rc::downgrade(&page.inner));
        Ok(page)
    }
}
impl SessionInner {
    pub fn dispatch(&self) {
        if self.failed.get() {
            return;
        }
        let more = self.driver.dispatch(EVENT_BUDGET);
        let (events, overflow, remaining) = {
            let mut inbox = self.sink.queue.lock().unwrap_or_else(|p| p.into_inner());
            let overflow = inbox.overflow;
            let count = inbox.events.len().min(EVENT_BUDGET);
            let events: Vec<_> = inbox.events.drain(..count).collect();
            for (_, event) in &events {
                inbox.text_bytes -= event.text_bytes();
            }
            (events, overflow, !inbox.events.is_empty())
        };
        if overflow {
            self.failed.set(true);
            let pages: Vec<_> = self
                .pages
                .borrow()
                .values()
                .filter_map(Weak::upgrade)
                .collect();
            for page in &pages {
                page.queue_failed();
            }
            for page in pages {
                page.flush();
            }
            return;
        }
        for (id, event) in events {
            let page = self.pages.borrow().get(&id).and_then(Weak::upgrade);
            if let Some(page) = page {
                page.receive(event);
            }
        }
        let pages: Vec<_> = self
            .pages
            .borrow()
            .values()
            .filter_map(Weak::upgrade)
            .collect();
        for page in pages {
            page.flush();
        }
        if more || remaining {
            self.sink.wake.notify();
        }
    }
}

#![deny(missing_docs)]
//! UIスレッドで通知をまとめて処理します。通知にUIオブジェクトを渡しません。
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    marker::PhantomData,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
const MAX_SUBSCRIPTIONS: usize = 128;
type Wake = Arc<dyn Fn() -> bool + Send + Sync>;
struct Signal {
    pending: AtomicBool,
    alive: AtomicBool,
    wake: Arc<Mutex<Option<Wake>>>,
}
struct Entry {
    signal: Arc<Signal>,
    callback: Option<Box<dyn FnMut()>>,
}
struct Registry {
    next: u64,
    entries: BTreeMap<u64, Entry>,
    wake: Arc<Mutex<Option<Wake>>>,
}
impl Registry {
    fn new() -> Self {
        Self {
            next: 1,
            entries: BTreeMap::new(),
            wake: Arc::new(Mutex::new(None)),
        }
    }
}
thread_local! {static DRAINING:Cell<bool>=const{Cell::new(false)};static REGISTRY:RefCell<Registry>=RefCell::new(Registry::new());}

/// UI通知の登録が上限に達したことを表します。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchCapacity;
impl std::fmt::Display for DispatchCapacity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("UI notification capacity exceeded")
    }
}
impl std::error::Error for DispatchCapacity {}

/// 作成したUIスレッドでコールバックを受け取る登録です。
///
/// 他スレッドへ移動できません。Drop後はコールバックを呼ばず、通知も拒否します。
/// コールバックは次のイベントループ処理で実行され、notify中には実行しません。
pub struct UiSubscription {
    id: u64,
    signal: Arc<Signal>,
    _ui: PhantomData<Rc<()>>,
}
impl UiSubscription {
    /// 現在のUIスレッドへ通知コールバックを登録します。
    ///
    /// ## Parameters
    /// - `callback`: UIスレッドで実行する処理です。重複通知はまとめられます。
    /// ## Errors
    /// - [`DispatchCapacity`]: 登録数または識別子の上限に達した場合。
    /// ## Example
    /// ```rust,no_run
    /// let subscription = viewkit::dispatch::UiSubscription::new(|| {})?;
    /// let notifier = subscription.notifier();
    /// # Ok::<(), viewkit::dispatch::DispatchCapacity>(())
    /// ```
    pub fn new(callback: impl FnMut() + 'static) -> Result<Self, DispatchCapacity> {
        REGISTRY.with_borrow_mut(|r| {
            if r.entries.len() >= MAX_SUBSCRIPTIONS || r.next == u64::MAX {
                return Err(DispatchCapacity);
            }
            let id = r.next;
            r.next += 1;
            let signal = Arc::new(Signal {
                pending: AtomicBool::new(false),
                alive: AtomicBool::new(true),
                wake: r.wake.clone(),
            });
            r.entries.insert(
                id,
                Entry {
                    signal: signal.clone(),
                    callback: Some(Box::new(callback)),
                },
            );
            Ok(Self {
                id,
                signal,
                _ui: PhantomData,
            })
        })
    }
    /// ワーカーへ渡せる通知専用の参照を返します。
    ///
    /// 登録の寿命は延長しません。UI状態やコールバック自体は含みません。
    pub fn notifier(&self) -> UiNotifier {
        UiNotifier {
            signal: self.signal.clone(),
        }
    }
}
impl Drop for UiSubscription {
    fn drop(&mut self) {
        self.signal.alive.store(false, Ordering::Release);
        REGISTRY.with_borrow_mut(|r| {
            r.entries.remove(&self.id);
        });
    }
}
/// UI通知だけを他スレッドから要求する参照です。Cloneとスレッド間共有が可能です。
#[derive(Clone)]
pub struct UiNotifier {
    signal: Arc<Signal>,
}
impl UiNotifier {
    /// 登録元のUIスレッドへ通知を予約します。
    ///
    /// ## Returns
    /// 登録が生存していればtrueを返します。trueは処理終了やコールバック実行済みを
    /// 意味しません。重複した通知は一回にまとまり、呼び出し元でUI処理は実行しません。
    /// イベントループ開始前の通知は開始後に配送されます。
    pub fn notify(&self) -> bool {
        if !self.signal.alive.load(Ordering::Acquire) {
            return false;
        }
        if !self.signal.pending.swap(true, Ordering::AcqRel) {
            let wake = self
                .signal
                .wake
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            if let Some(wake) = wake {
                let _ = wake();
            }
        }
        self.signal.alive.load(Ordering::Acquire)
    }
}
pub(crate) fn drain() -> bool {
    if DRAINING.replace(true) {
        return false;
    }
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            DRAINING.set(false)
        }
    }
    let _reset = Reset;
    let ids = REGISTRY.with_borrow(|r| {
        r.entries
            .iter()
            .filter(|(_, e)| e.signal.pending.swap(false, Ordering::AcqRel))
            .map(|(id, _)| *id)
            .collect::<Vec<_>>()
    });
    let mut ran = false;
    for id in ids {
        let callback =
            REGISTRY.with_borrow_mut(|r| r.entries.get_mut(&id).and_then(|e| e.callback.take()));
        if let Some(mut callback) = callback {
            callback();
            ran = true;
            REGISTRY.with_borrow_mut(|r| {
                if let Some(entry) = r.entries.get_mut(&id) {
                    entry.callback = Some(callback)
                }
            });
        }
    }
    ran
}
pub(crate) struct WakeGuard;
pub(crate) fn install_wake(wake: impl Fn() -> bool + Send + Sync + 'static) -> WakeGuard {
    REGISTRY.with_borrow_mut(|r| {
        *r.wake.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::new(wake))
    });
    WakeGuard
}
impl Drop for WakeGuard {
    fn drop(&mut self) {
        REGISTRY.with_borrow_mut(|r| *r.wake.lock().unwrap_or_else(|e| e.into_inner()) = None);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[test]
    fn worker_only_wakes_and_callbacks_run_on_the_registering_thread() {
        let thread = std::thread::current().id();
        let count = Rc::new(Cell::new(0));
        let seen = count.clone();
        let registration = UiSubscription::new(move || {
            assert_eq!(std::thread::current().id(), thread);
            seen.set(seen.get() + 1);
        })
        .unwrap();
        let notify = registration.notifier();
        std::thread::spawn(move || {
            for _ in 0..1000 {
                assert!(notify.notify());
            }
        })
        .join()
        .unwrap();
        assert_eq!(count.get(), 0);
        assert!(drain());
        assert_eq!(count.get(), 1);
        assert!(!drain());
        let notify = registration.notifier();
        drop(registration);
        assert!(!notify.notify());
        assert!(!drain());
    }
    #[test]
    fn callbacks_can_remove_themselves_and_register_without_borrow_panics() {
        let holder = Rc::new(RefCell::new(None));
        let h = holder.clone();
        let s = UiSubscription::new(move || {
            h.borrow_mut().take();
            let _other = UiSubscription::new(|| {}).unwrap();
        })
        .unwrap();
        s.notifier().notify();
        *holder.borrow_mut() = Some(s);
        assert!(drain());
        assert!(holder.borrow().is_none());
    }
    #[test]
    fn notification_during_dispatch_is_delivered_on_the_next_turn() {
        let notify = Rc::new(RefCell::new(None::<UiNotifier>));
        let n = notify.clone();
        let count = Rc::new(Cell::new(0));
        let c = count.clone();
        let s = UiSubscription::new(move || {
            c.set(c.get() + 1);
            if c.get() == 1 {
                n.borrow().as_ref().unwrap().notify();
                assert!(!drain());
            }
        })
        .unwrap();
        *notify.borrow_mut() = Some(s.notifier());
        s.notifier().notify();
        assert!(drain());
        assert_eq!(count.get(), 1);
        assert!(drain());
        assert_eq!(count.get(), 2);
    }
    #[test]
    fn worker_wakes_an_idle_ui_without_input_or_redraw() {
        let current = std::thread::current();
        let _wake = install_wake(move || {
            current.unpark();
            true
        });
        let called = Rc::new(Cell::new(false));
        let c = called.clone();
        let registration = UiSubscription::new(move || c.set(true)).unwrap();
        let notify = registration.notifier();
        let (start, wait) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            wait.recv().unwrap();
            notify.notify()
        });
        start.send(()).unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while !called.get() {
            drain();
            assert!(std::time::Instant::now() < deadline);
            if !called.get() {
                std::thread::park_timeout(std::time::Duration::from_millis(100));
            }
        }
        assert!(worker.join().unwrap());
    }
}

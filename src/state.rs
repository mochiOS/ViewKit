//! アプリケーションの変更可能な状態を扱います。

use crate::animation::Transition;
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::time::Instant;

thread_local! {
    static STATE_CHANGED: Cell<bool> = const { Cell::new(false) };
}

fn mark_changed() {
    STATE_CHANGED.set(true);
}

/// 状態が変更されたか確認し、変更フラグを解除します。
pub(crate) fn take_state_changed() -> bool {
    STATE_CHANGED.replace(false)
}

/// アプリケーションが所有する変更可能な状態です.
///
/// cloneした`State`は、同じ値を共有します。
pub struct State<T> {
    value: Rc<RefCell<T>>,
    previous_value: Rc<RefCell<Option<T>>>,
    last_changed_at: Rc<Cell<Option<Instant>>>,
}

impl<T> State<T> {
    /// 指定した初期値で状態を作成します。
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
            previous_value: Rc::new(RefCell::new(None)),
            last_changed_at: Rc::new(Cell::new(None)),
        }
    }

    /// 現在の値を複製して返します。
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }

    /// 現在の値を置き換えます。
    pub fn set(&self, value: T) {
        let previous = self.value.replace(value);

        self.previous_value.replace(Some(previous));
        self.last_changed_at.set(Some(Instant::now()));

        mark_changed();
    }

    /// 現在の値を変更します。
    pub fn update<R>(&self, update: impl FnOnce(&mut T) -> R) -> R {
        self.previous_value.replace(None);
        let result = update(&mut self.value.borrow_mut());
        self.last_changed_at.set(Some(Instant::now()));

        mark_changed();

        result
    }

    /// Viewへ渡すためのBindingを作成します。
    #[must_use]
    pub fn binding(&self) -> Binding<T> {
        Binding {
            value: Rc::clone(&self.value),
            previous_value: Rc::clone(&self.previous_value),
            last_changed_at: Rc::clone(&self.last_changed_at),
        }
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            previous_value: Rc::clone(&self.previous_value),
            last_changed_at: Rc::clone(&self.last_changed_at),
        }
    }
}

/// Viewから状態を読み書きするための参照です。
pub struct Binding<T> {
    value: Rc<RefCell<T>>,
    previous_value: Rc<RefCell<Option<T>>>,
    last_changed_at: Rc<Cell<Option<Instant>>>,
}

impl<T> Binding<T> {
    /// 現在の値を複製して返します。
    #[must_use]
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.value.borrow().clone()
    }

    /// 現在の値を置き換えます。
    pub fn set(&self, value: T) {
        let previous = self.value.replace(value);
        self.previous_value.replace(Some(previous));
        self.last_changed_at.set(Some(Instant::now()));

        mark_changed();
    }

    /// 現在の値を変更します。
    pub fn update<R>(&self, update: impl FnOnce(&mut T) -> R) -> R {
        self.previous_value.replace(None);
        let result = update(&mut self.value.borrow_mut());
        self.last_changed_at.set(Some(Instant::now()));

        mark_changed();

        result
    }

    /// 状態変更通知を発生させずに値を置き換えます。
    ///
    /// View内部の操作状態を維持したままBindingへ値を同期するために使用します。
    pub(crate) fn set_without_notification(&self, value: T) {
        *self.value.borrow_mut() = value;
        self.previous_value.replace(None);
        self.last_changed_at.set(None);
    }

    pub(crate) fn transition(&self) -> Option<Transition<T>>
    where
        T: Clone,
    {
        let started_at = self.last_changed_at.get()?;

        let from = self.previous_value.borrow().as_ref()?.clone();

        let to = self.value.borrow().clone();

        Some(Transition::new(from, to, started_at))
    }

    pub(crate) fn commit(&self) {
        mark_changed();
    }
}

impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
            previous_value: Rc::clone(&self.previous_value),
            last_changed_at: Rc::clone(&self.last_changed_at),
        }
    }
}

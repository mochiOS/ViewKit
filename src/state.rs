//! アプリケーションの変更可能な状態を扱います。

use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
}

impl<T> State<T> {
    /// 指定した初期値で状態を作成します。
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(value)),
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
        *self.value.borrow_mut() = value;
        mark_changed();
    }

    /// 現在の値を変更します。
    pub fn update<R>(&self, update: impl FnOnce(&mut T) -> R) -> R {
        let result = update(&mut self.value.borrow_mut());

        mark_changed();

        result
    }

    /// Viewへ渡すためのBindingを作成します。
    #[must_use]
    pub fn binding(&self) -> Binding<T> {
        Binding {
            value: Rc::clone(&self.value),
        }
    }
}

impl<T> Clone for State<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
        }
    }
}

/// Viewから状態を読み書きするための参照です。
pub struct Binding<T> {
    value: Rc<RefCell<T>>,
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
        *self.value.borrow_mut() = value;
        mark_changed();
    }

    /// 現在の値を変更します。
    pub fn update<R>(&self, update: impl FnOnce(&mut T) -> R) -> R {
        let result = update(&mut self.value.borrow_mut());

        mark_changed();

        result
    }

    /// 状態変更通知を発生させずに値を置き換えます。
    ///
    /// View内部の操作状態を維持したままBindingへ値を同期するために使用します。
    pub(crate) fn set_without_notification(&self, value: T) {
        *self.value.borrow_mut() = value;
    }
}

impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        Self {
            value: Rc::clone(&self.value),
        }
    }
}

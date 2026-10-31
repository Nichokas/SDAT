//! Port of `internal/signals`: stateful and stateless signals. Everything in
//! Tonearm's UI and player lives on the GTK main thread, so these are
//! single-threaded (`Rc`/`RefCell`); each thread gets its own instances.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub const CONTINUE: bool = false;
pub const UNSUBSCRIBE: bool = true;

pub type Subscription = u64;

type Handler<T> = Rc<dyn Fn(&T) -> bool>;

pub struct Signal<T> {
    handlers: RefCell<Vec<(Subscription, Handler<T>)>>,
    next_id: Cell<u64>,
}

impl<T> Default for Signal<T> {
    fn default() -> Self {
        Signal { handlers: RefCell::new(Vec::new()), next_id: Cell::new(1) }
    }
}

impl<T> Signal<T> {
    /// Registers a handler; returning [`UNSUBSCRIBE`] removes it.
    pub fn on(&self, f: impl Fn(&T) -> bool + 'static) -> Subscription {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        self.handlers.borrow_mut().push((id, Rc::new(f)));
        id
    }
    pub fn unsubscribe(&self, sub: Subscription) {
        self.handlers.borrow_mut().retain(|(id, _)| *id != sub);
    }
    pub fn notify(&self, value: &T) {
        let handlers: Vec<_> = self.handlers.borrow().clone();
        for (id, h) in handlers {
            if !self.handlers.borrow().iter().any(|(i, _)| *i == id) {
                continue;
            }
            if h(value) {
                self.unsubscribe(id);
            }
        }
    }
}

/// A signal that remembers its last value (`StatefulSignal`).
pub struct State<T> {
    value: RefCell<T>,
    signal: Signal<T>,
}

impl<T: Clone + 'static> State<T> {
    pub fn new(v: T) -> Self {
        State { value: RefCell::new(v), signal: Signal::default() }
    }
    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }
    pub fn with<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.value.borrow())
    }
    /// Updates the value through `f` and notifies subscribers.
    pub fn notify(&self, f: impl FnOnce(T) -> T) {
        let old = self.get();
        let new = f(old);
        *self.value.borrow_mut() = new.clone();
        self.signal.notify(&new);
    }
    pub fn set(&self, v: T) {
        self.notify(|_| v);
    }
    /// Subscribes and immediately calls the handler with the current value.
    pub fn on(&self, f: impl Fn(&T) -> bool + 'static) -> Subscription {
        let cur = self.get();
        let id = self.signal.on(f);
        let h = self.signal.handlers.borrow().iter().find(|(i, _)| *i == id).map(|(_, h)| std::rc::Rc::<dyn for<'a> std::ops::Fn(&'a T) -> bool>::clone(h));
        if let Some(h) = h
            && h(&cur) {
                self.signal.unsubscribe(id);
            }
        id
    }
    /// Subscribes without calling the handler for the current value.
    pub fn on_lazy(&self, f: impl Fn(&T) -> bool + 'static) -> Subscription {
        self.signal.on(f)
    }
    pub fn unsubscribe(&self, sub: Subscription) {
        self.signal.unsubscribe(sub);
    }
}

/// Declares a per-thread, `'static` signal accessor:
/// `global_state!(pub fn foo() -> State<i32> = State::new(0));`
#[macro_export]
macro_rules! global_signal {
    ($vis:vis fn $name:ident() -> $ty:ty = $init:expr) => {
        $vis fn $name() -> &'static $ty {
            thread_local! {
                static S: &'static $ty = Box::leak(Box::new($init));
            }
            S.with(|s| *s)
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stateful_semantics() {
        let s = State::new(1);
        let seen = Rc::new(RefCell::new(vec![]));
        let s2 = Rc::clone(&seen);
        s.on(move |v| {
            s2.borrow_mut().push(*v);
            *v >= 3
        });
        s.set(2);
        s.set(3);
        s.set(4);
        assert_eq!(*seen.borrow(), vec![1, 2, 3]);
        let lazy = Rc::new(Cell::new(0));
        let l2 = Rc::clone(&lazy);
        s.on_lazy(move |v| {
            l2.set(*v);
            CONTINUE
        });
        assert_eq!(lazy.get(), 0);
        s.notify(|v| v + 1);
        assert_eq!(lazy.get(), 5);
    }
}

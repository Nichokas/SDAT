//! Coordinates `GStreamer`'s streaming-thread `about-to-finish` wait with the
//! main-thread playbin operations that would deadlock against it.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
struct WaitState {
    blocking_ops: usize,
    /// A stop or start is queued or being handled on the main loop.
    held: bool,
    generation: u64,
    waiting: bool,
    done: bool,
}

/// Coordinates `GStreamer`'s streaming-thread `about-to-finish` wait with
/// main-thread playbin operations (`set_state(Ready | Null)`, flushing seeks)
/// that acquire `GST_PAD_STREAM_LOCK`, preventing a 15 s deadlock.
#[derive(Debug, Default)]
pub struct GaplessGate {
    state: Mutex<WaitState>,
    cond: Condvar,
}

#[derive(Debug)]
pub struct GaplessToken {
    gate: Arc<GaplessGate>,
    generation: u64,
    finished: AtomicBool,
}

#[derive(Debug)]
pub struct BlockingOpGuard<'a> {
    gate: &'a GaplessGate,
}

impl WaitState {
    fn blocked(&self) -> bool {
        self.blocking_ops > 0 || self.held
    }
}

impl GaplessGate {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WaitState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn generation(&self) -> u64 {
        self.lock().generation
    }

    /// Wakes any waiting streaming thread and stops new `about-to-finish`
    /// callbacks from blocking until the guard drops.
    pub fn enter_blocking_op(&self) -> BlockingOpGuard<'_> {
        let mut st = self.lock();
        st.blocking_ops += 1;
        self.cancel_waiters(&mut st);
        BlockingOpGuard { gate: self }
    }

    /// Set while a stop or start is queued or being handled, so a wait that
    /// begins before it runs cannot outlive it; it blocks like a blocking op.
    pub fn set_held(&self, held: bool) {
        let mut st = self.lock();
        if held && !st.held {
            self.cancel_waiters(&mut st);
        }
        st.held = held;
    }

    fn cancel_waiters(&self, st: &mut WaitState) {
        st.generation = st.generation.wrapping_add(1);
        if st.waiting {
            st.done = true;
            st.waiting = false;
            self.cond.notify_all();
        }
    }

    /// Mints a non-blocking token when `about-to-finish` fires on the main thread.
    pub fn begin_nonblocking(self: &Arc<Self>) -> Option<GaplessToken> {
        let mut st = self.lock();
        if st.blocked() {
            return None;
        }
        st.generation = st.generation.wrapping_add(1);
        let generation = st.generation;
        Some(GaplessToken {
            gate: Arc::clone(self),
            generation,
            finished: AtomicBool::new(false),
        })
    }

    /// Dispatches `dispatch` with a cancellation-aware token and blocks until
    /// the token finishes, a main-thread op cancels it, or `timeout` elapses.
    /// Returns `true` only when the wait timed out.
    pub fn wait(self: &Arc<Self>, timeout: Duration, dispatch: impl FnOnce(GaplessToken)) -> bool {
        let generation = {
            let mut st = self.lock();
            if st.blocked() {
                return false;
            }
            if st.waiting {
                st.done = true;
                self.cond.notify_all();
            }
            st.generation = st.generation.wrapping_add(1);
            st.waiting = true;
            st.done = false;
            st.generation
        };
        dispatch(GaplessToken {
            gate: Arc::clone(self),
            generation,
            finished: AtomicBool::new(false),
        });
        let deadline = Instant::now() + timeout;
        let mut st = self.lock();
        while !st.done && st.generation == generation {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let (next, res) = self
                .cond
                .wait_timeout(st, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            st = next;
            if res.timed_out() && !st.done && st.generation == generation {
                break;
            }
        }
        let timed_out = !st.done && st.generation == generation;
        if timed_out {
            st.generation = st.generation.wrapping_add(1);
            st.waiting = false;
        } else if st.generation == generation {
            st.waiting = false;
        }
        timed_out
    }
}

impl Drop for BlockingOpGuard<'_> {
    fn drop(&mut self) {
        let mut st = self.gate.lock();
        st.blocking_ops = st.blocking_ops.saturating_sub(1);
    }
}

impl GaplessToken {
    pub fn is_cancelled(&self) -> bool {
        let st = self.gate.lock();
        st.generation != self.generation || st.blocked()
    }

    pub fn finish(&self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut st = self.gate.lock();
        if st.generation == self.generation && st.waiting {
            st.done = true;
            st.waiting = false;
            self.gate.cond.notify_all();
        }
    }
}

impl Drop for GaplessToken {
    fn drop(&mut self) {
        self.finish();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gapless_gate_wakes_immediately_on_blocking_op() {
        let gate = Arc::new(GaplessGate::new());
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter_gate = Arc::clone(&gate);
        let handle = std::thread::spawn(move || {
            let started = Instant::now();
            let timed_out = waiter_gate.wait(Duration::from_secs(15), move |token| {
                tx.send(token).unwrap();
            });
            (timed_out, started.elapsed())
        });

        let token = rx.recv().unwrap();
        let guard = gate.enter_blocking_op();
        let (timed_out, elapsed) = handle.join().unwrap();
        assert!(!timed_out, "canceling the gate must not report a timeout");
        assert!(elapsed < Duration::from_secs(1), "waiter stalled for {elapsed:?}");
        assert!(token.is_cancelled());

        // While a blocking op is active, new waits return immediately without dispatching.
        let dispatched = Arc::new(AtomicBool::new(false));
        let d = Arc::clone(&dispatched);
        assert!(!gate.wait(Duration::from_secs(15), move |_| d.store(true, Ordering::SeqCst)));
        assert!(!dispatched.load(Ordering::SeqCst));
        drop(guard);

        // Normal completion wakes immediately and keeps the token uncanceled.
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter_gate = Arc::clone(&gate);
        let handle = std::thread::spawn(move || {
            let started = Instant::now();
            let timed_out = waiter_gate.wait(Duration::from_secs(15), move |token| {
                tx.send(token).unwrap();
            });
            (timed_out, started.elapsed())
        });
        let token = rx.recv().unwrap();
        assert!(!token.is_cancelled());
        drop(token);
        let (timed_out, elapsed) = handle.join().unwrap();
        assert!(!timed_out);
        assert!(elapsed < Duration::from_secs(1), "drop wake stalled for {elapsed:?}");
    }

    #[test]
    fn hold_cancels_waits_until_released() {
        let gate = Arc::new(GaplessGate::new());
        let token = gate.begin_nonblocking().unwrap();
        gate.set_held(true);
        assert!(token.is_cancelled());
        assert!(gate.begin_nonblocking().is_none());
        assert!(!gate.wait(Duration::from_secs(5), |_| panic!("dispatched while held")));
        gate.set_held(false);
        assert!(gate.begin_nonblocking().is_some());
    }
}

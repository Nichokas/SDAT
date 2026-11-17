//! Diagnostics: the core's `log` output and app-side errors in one bounded, redacted event log that the
//! apps show, export and forward (to Sentry, logcat, oslog) through a [`Sink`].
//!
//! Call [`init`] once at startup. Everything else does nothing before it.

use std::cell::Cell;
use std::fmt;
use std::io::Write as _;
use std::ops::{Deref, DerefMut};
use std::panic::AssertUnwindSafe;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, Once, OnceLock, PoisonError};
use std::time::Instant;

mod install;
pub(crate) mod net;
mod process;
mod redact;
mod report;
mod store;

pub use net::{NetCategory, NetStat};

use store::Store;

const MARK_TARGET: &str = "mark";
const PANIC_TARGET: &str = "panic";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "ERROR",
            Self::Warn => "WARN",
            Self::Info => "INFO",
            Self::Debug => "DEBUG",
            Self::Trace => "TRACE",
        }
    }
}

impl From<log::Level> for Level {
    fn from(l: log::Level) -> Self {
        match l {
            log::Level::Error => Self::Error,
            log::Level::Warn => Self::Warn,
            log::Level::Info => Self::Info,
            log::Level::Debug => Self::Debug,
            log::Level::Trace => Self::Trace,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Strictly increasing over the life of the process, also across [`clear`].
    pub seq: u64,
    /// Unix time in milliseconds.
    pub at_ms: i64,
    pub level: Level,
    pub target: String,
    pub message: String,
    pub backtrace: Option<String>,
}

impl fmt::Display for Event {
    /// The event as it is written to the log file: a header line, then the rest of the message and the
    /// backtrace indented below it.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let time = chrono::DateTime::from_timestamp_millis(self.at_ms)
            .map_or_else(|| self.at_ms.to_string(), |t| t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());
        let mut message = self.message.lines();
        write!(f, "{time} {:<5} [{}] {}", self.level.as_str(), self.target, message.next().unwrap_or_default())?;
        for line in message.chain(self.backtrace.iter().flat_map(|b| b.lines())) {
            write!(f, "\n    {line}")?;
        }
        Ok(())
    }
}

/// Platform hook, called without any lock held for events of level warn and above, for panics and for marks.
/// A sink that logs itself is not called again for that event.
pub trait Sink: Send + Sync {
    fn on_event(&self, event: &Event);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub uptime_ms: u64,
    pub cpu_user_ms: u64,
    pub cpu_system_ms: u64,
    /// Peak resident memory of the process.
    pub max_rss_kb: u64,
    /// One row per [`NetCategory`].
    pub net: Vec<NetStat>,
}

struct Global {
    store: Mutex<Store>,
    sink: Option<Box<dyn Sink>>,
}

thread_local! {
    static HOLDS_STORE: Cell<bool> = const { Cell::new(false) };
    static IN_SINK: Cell<bool> = const { Cell::new(false) };
}

/// The store's lock, remembered per thread: a panic raised while this thread holds it must not wait for it.
struct Held<'a>(MutexGuard<'a, Store>);

impl Deref for Held<'_> {
    type Target = Store;
    fn deref(&self) -> &Store {
        &self.0
    }
}

impl DerefMut for Held<'_> {
    fn deref_mut(&mut self) -> &mut Store {
        &mut self.0
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        HOLDS_STORE.set(false);
    }
}

impl Global {
    /// `None` when this thread already holds the lock: the caller loses its event instead of deadlocking.
    fn lock(&self) -> Option<Held<'_>> {
        if HOLDS_STORE.get() {
            return None;
        }
        let guard = self.store.lock().unwrap_or_else(PoisonError::into_inner);
        HOLDS_STORE.set(true);
        Some(Held(guard))
    }

    fn notify(&self, event: &Event) {
        let Some(sink) = &self.sink else { return };
        if IN_SINK.replace(true) {
            return;
        }
        let _reset = ResetInSink;
        // A panicking sink must not unwind into the code that logged; the panic hook has already recorded it.
        let _ = std::panic::catch_unwind(AssertUnwindSafe(|| sink.on_event(event)));
    }
}

struct ResetInSink;

impl Drop for ResetInSink {
    fn drop(&mut self) {
        IN_SINK.set(false);
    }
}

static GLOBAL: OnceLock<Global> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();

fn uptime_ms() -> u64 {
    u64::try_from(STARTED.get_or_init(Instant::now).elapsed().as_millis()).unwrap_or(u64::MAX)
}

/// Starts collecting. Only the first call has an effect; a later one is logged as a warning and ignored.
///
/// Installs the `log` logger (default level info, echoed to stderr) and a panic hook, and with `dir` keeps
/// `diag.log` there, moving the last run's to `diag.prev.log`.
pub fn init(dir: Option<PathBuf>, sink: Option<Box<dyn Sink>>) {
    static INIT: Once = Once::new();
    STARTED.get_or_init(Instant::now);
    let mut first = false;
    // A concurrent caller waits here until the logger and the panic hook are in place, not only the store.
    INIT.call_once(|| {
        first = true;
        let _ = GLOBAL.set(Global { store: Mutex::new(Store::new(dir)), sink });
        install::install();
    });
    // Outside `call_once`: these reach the sink, which may call back into `init`.
    if !first {
        log::warn!("diag::init called again; its directory and sink are ignored");
    } else if !install::owns_logger() {
        record(Level::Warn, "diag", "another log logger was installed first: log records do not reach diagnostics", None);
    }
}

/// Longest message or backtrace kept, so one huge event (an error page logged whole) neither overflows the
/// log file nor holds the buffer's memory.
const MAX_TEXT_BYTES: usize = 32 * 1024;
const MAX_TARGET_BYTES: usize = 256;

/// `text` redacted and cut to `max` bytes, marked when anything was cut. Only the first `max` bytes are
/// redacted, so a huge message costs one bounded pass: a secret cut there keeps its key and is redacted to
/// the end, and one whose key is cut is dropped with the rest.
fn bounded(text: &str, max: usize) -> String {
    const MARK: &str = " …(truncated)";
    let cut = text.floor_char_boundary(max);
    let mut out = redact::redact(&text[..cut]).into_owned();
    if cut < text.len() || out.len() > max {
        out.truncate(out.floor_char_boundary(max.saturating_sub(MARK.len())));
        out.push_str(MARK);
        out.shrink_to_fit();
    }
    out
}

/// The one path every event takes, so redaction happens before anything is stored, written or sent.
fn ingest(level: Level, target: &str, message: &str, backtrace: Option<String>, echo: bool) {
    let Some(global) = GLOBAL.get() else { return };
    let target = bounded(target, MAX_TARGET_BYTES);
    let message = bounded(message, MAX_TEXT_BYTES);
    let backtrace = backtrace.map(|b| bounded(&b, MAX_TEXT_BYTES));
    let notify = matches!(level, Level::Error | Level::Warn) || target == MARK_TARGET || target == PANIC_TARGET;
    let (copy, line) = {
        let Some(mut store) = global.lock() else { return };
        let event = store.push(level, &target, message, backtrace);
        (notify.then(|| event.clone()), echo.then(|| event.to_string()))
    };
    if let Some(line) = line {
        // Not `eprintln!`: it panics when stderr is closed, and logging must never fail the caller.
        let _ = writeln!(std::io::stderr(), "{line}");
    }
    if let Some(event) = copy {
        global.notify(&event);
    }
}

/// Adds an event from the app (a caught Kotlin or Swift exception, say) to the same log.
pub fn record(level: Level, target: &str, message: &str, backtrace: Option<String>) {
    ingest(level, target, message, backtrace, false);
}

/// Puts a note in the log at this moment ("the track skipped now") and sends it to the sink.
pub fn mark(note: &str) {
    ingest(Level::Info, MARK_TARGET, note, None, false);
}

/// The events after `seq`, oldest first.
pub fn events_since(seq: u64) -> Vec<Event> {
    GLOBAL.get().and_then(Global::lock).map_or_default(|store| store.since(seq))
}

/// Debug level on, or back to info. Does nothing before [`init`], which resets it to info, or when another
/// `log` logger was installed first.
pub fn set_verbose(on: bool) {
    if !install::owns_logger() {
        return;
    }
    log::set_max_level(if on { log::LevelFilter::Debug } else { log::LevelFilter::Info });
}

pub fn verbose() -> bool {
    install::owns_logger() && log::max_level() >= log::LevelFilter::Debug
}

/// The previous run's log, or an empty string.
pub fn previous_session() -> String {
    GLOBAL.get().and_then(Global::lock).map_or_default(|store| store.previous_session())
}

/// Empties the buffer, the current log file and the previous run's.
pub fn clear() {
    if let Some(mut store) = GLOBAL.get().and_then(Global::lock) {
        store.clear();
    }
}

pub fn stats() -> Stats {
    let usage = process::usage();
    Stats {
        uptime_ms: uptime_ms(),
        cpu_user_ms: usage.cpu_user_ms,
        cpu_system_ms: usage.cpu_system_ms,
        max_rss_kb: usage.max_rss_kb,
        net: net::snapshot(),
    }
}

/// A plain-text report to share: `extra` (the platform header: app version, OS, device) first, then
/// uptime, process and network figures and every event in the buffer.
pub fn report(extra: &str) -> String {
    report::build(&redact::redact(extra), &stats(), &events_since(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::panic::catch_unwind;

    static SENT: Mutex<Vec<Event>> = Mutex::new(Vec::new());

    struct Collect;

    impl Sink for Collect {
        fn on_event(&self, event: &Event) {
            assert!(!event.message.contains("diag-test-sink-panics"), "the sink failed");
            SENT.lock().unwrap_or_else(PoisonError::into_inner).push(event.clone());
        }
    }

    // The logger, panic hook and buffer are process-wide: the tests share one init and look for their own
    // unique text, so they can run in parallel.
    fn setup() {
        static ONCE: Once = Once::new();
        ONCE.call_once(|| init(None, Some(Box::new(Collect))));
    }

    fn stored(needle: &str) -> Vec<Event> {
        events_since(0).into_iter().filter(|e| e.message.contains(needle)).collect()
    }

    fn sent(needle: &str) -> Vec<Event> {
        SENT.lock().unwrap_or_else(PoisonError::into_inner).iter().filter(|e| e.message.contains(needle)).cloned().collect()
    }

    #[test]
    fn a_panic_becomes_an_error_event_with_a_backtrace_and_reaches_the_sink() {
        setup();
        let caught = catch_unwind(|| panic!("diag-test-panic-1"));
        assert!(caught.is_err());
        let events = stored("diag-test-panic-1");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].level, Level::Error);
        assert_eq!(events[0].target, "panic");
        assert!(events[0].backtrace.as_deref().is_some_and(|b| !b.is_empty()));
        assert_eq!(sent("diag-test-panic-1").len(), 1);
    }

    #[test]
    fn a_huge_message_is_clipped_and_a_secret_cut_by_the_clip_stays_redacted() {
        setup();
        let filler = "x".repeat(32 * 1024 - 20);
        record(Level::Info, "test", &format!("diag-test-huge {filler} password=hunter2hunter2 {}", "y".repeat(1 << 20)), None);
        let event = stored("diag-test-huge").pop().expect("stored");
        assert!(event.message.len() < 33 * 1024);
        assert!(!event.message.contains("hunter"), "{}", &event.message[event.message.len() - 60..]);
    }

    #[test]
    fn a_cut_event_is_always_marked_and_bounded() {
        let marked = |s: &String| s.ends_with("…(truncated)") && s.len() <= MAX_TEXT_BYTES && s.capacity() < MAX_TEXT_BYTES + 32;
        // Redaction shrinks this one far below the limit; the cut must still show.
        assert!(marked(&bounded(&format!("password={}", "a".repeat(40_000)), MAX_TEXT_BYTES)));
        assert!(marked(&bounded(&"é".repeat(20_000), MAX_TEXT_BYTES)));
        assert!(marked(&bounded(&"token=a ".repeat(10_000), MAX_TEXT_BYTES)), "redaction grows it past the limit");
        assert_eq!(bounded("short", MAX_TEXT_BYTES), "short");
        assert!(bounded(&"t".repeat(1 << 20), MAX_TARGET_BYTES).len() <= MAX_TARGET_BYTES);
        assert!(bounded(&"token=a ".repeat(32), MAX_TARGET_BYTES).len() <= MAX_TARGET_BYTES);
    }

    #[test]
    fn a_panicking_sink_does_not_reach_the_caller() {
        setup();
        record(Level::Warn, "test", "diag-test-sink-panics", None);
        assert_eq!(stored("diag-test-sink-panics").len(), 1);
        assert_eq!(stored("the sink failed").len(), 1, "the sink's own panic is recorded");
        record(Level::Warn, "test", "diag-test-after-sink-panic", None);
        assert_eq!(sent("diag-test-after-sink-panic").len(), 1);
    }

    #[test]
    fn a_second_init_keeps_the_first_sink_and_says_so() {
        setup();
        init(None, None);
        assert_ne!(stored("diag::init called again").len(), 0);
        record(Level::Warn, "test", "diag-test-after-second-init", None);
        assert_eq!(sent("diag-test-after-second-init").len(), 1);
    }

    #[test]
    fn a_panic_while_the_log_is_locked_does_not_deadlock() {
        setup();
        let held = GLOBAL.get().and_then(Global::lock);
        assert!(held.is_some());
        let caught = catch_unwind(AssertUnwindSafe(|| {
            record(Level::Warn, "test", "diag-test-lost-event", None);
            panic!("diag-test-panic-2")
        }));
        assert!(caught.is_err());
        drop(held);
        assert_eq!(stored("diag-test-panic-2").len(), 0, "the event of the locked thread is lost, not stuck");
        record(Level::Info, "test", "diag-test-after-lock", None);
        assert_eq!(stored("diag-test-after-lock").len(), 1);
    }

    #[test]
    fn log_records_enter_the_buffer_and_only_warnings_reach_the_sink() {
        setup();
        log::warn!(target: "diag-test", "diag-test-warn-3");
        log::info!(target: "diag-test", "diag-test-info-3");
        let warn = stored("diag-test-warn-3");
        assert_eq!((warn.len(), warn[0].level, warn[0].target.as_str()), (1, Level::Warn, "diag-test"));
        assert_eq!(stored("diag-test-info-3")[0].level, Level::Info);
        assert_eq!(sent("diag-test-warn-3").len(), 1);
        assert_eq!(sent("diag-test-info-3").len(), 0);
    }

    #[test]
    fn verbose_lets_debug_records_in() {
        setup();
        assert!(!verbose());
        log::debug!("diag-test-debug-hidden");
        set_verbose(true);
        assert!(verbose());
        log::debug!("diag-test-debug-shown");
        set_verbose(false);
        assert!(!verbose());
        assert_eq!(stored("diag-test-debug-hidden").len(), 0);
        assert_eq!(stored("diag-test-debug-shown")[0].level, Level::Debug);
    }

    #[test]
    fn marks_and_app_errors_are_logged_and_sent() {
        setup();
        mark("diag-test-mark-5");
        let marks = stored("diag-test-mark-5");
        assert_eq!((marks[0].level, marks[0].target.as_str()), (Level::Info, "mark"));
        assert_eq!(sent("diag-test-mark-5").len(), 1);

        record(Level::Error, "kotlin", "diag-test-app-error-5", Some("at Foo.bar(Foo.kt:1)".into()));
        let events = stored("diag-test-app-error-5");
        assert_eq!(events[0].backtrace.as_deref(), Some("at Foo.bar(Foo.kt:1)"));
        assert_eq!(sent("diag-test-app-error-5").len(), 1);
    }

    #[test]
    fn secrets_never_reach_the_buffer_the_sink_or_the_report() {
        setup();
        log::error!("diag-test-secret-6 refresh failed: refresh_token=hunter2-r&x=1");
        record(Level::Warn, "swift", "diag-test-secret-6 Authorization: Bearer hunter2-a", Some("trace password=hunter2-p".into()));
        mark("diag-test-secret-6 code=hunter2-c");
        for events in [stored("diag-test-secret-6"), sent("diag-test-secret-6")] {
            assert_eq!(events.len(), 3);
            let text = format!("{events:?}");
            assert!(!text.contains("hunter2"), "{text}");
            assert!(text.contains("<redacted>"));
        }
        record(Level::Error, "https://cb?code=hunter2-t", "diag-test-secret-6 target", None);
        assert!(stored("diag-test-secret-6 target").iter().all(|e| !e.target.contains("hunter2")));
        assert!(sent("diag-test-secret-6 target").iter().all(|e| !e.target.contains("hunter2")));
        let text = report("SDAT Authorization: Bearer hunter2-h");
        assert!(!text.contains("hunter2"), "{text}");
    }

    #[test]
    fn the_report_has_the_header_figures_and_events() {
        setup();
        record(Level::Info, "test", "diag-test-report-7", None);
        let text = report("SDAT 9.9.9\nTest OS");
        assert!(text.starts_with("SDAT 9.9.9\nTest OS\n"), "{text}");
        for part in ["Uptime:", "CPU time:", "Peak memory:", "Network", "TIDAL", "Last.fm", "Events (", "diag-test-report-7"] {
            assert!(text.contains(part), "{part} missing in\n{text}");
        }
    }

    #[test]
    fn stats_report_the_process() {
        setup();
        let s = stats();
        assert!(s.max_rss_kb > 0);
        assert_eq!(s.net.len(), NetCategory::ALL.len());
    }

    #[test]
    fn a_sink_that_logs_is_not_called_for_its_own_event() {
        struct Chatty;
        impl Sink for Chatty {
            fn on_event(&self, _: &Event) {
                log::warn!("diag-test-from-sink-9");
            }
        }
        setup();
        let global = Global { store: Mutex::new(Store::new(None)), sink: Some(Box::new(Chatty)) };
        let event = Event { seq: 1, at_ms: 0, level: Level::Warn, target: "t".into(), message: "m".into(), backtrace: None };
        global.notify(&event);
        assert_eq!(stored("diag-test-from-sink-9").len(), 1, "stored once");
        assert_eq!(sent("diag-test-from-sink-9").len(), 0, "not sent again while a sink is running");
    }
}

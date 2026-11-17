use std::backtrace::Backtrace;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use log::{LevelFilter, Log, Metadata, Record};

use super::{Level, ingest, redact};

/// Crates whose debug and trace output verbose mode keeps; the rest (hyper, rustls, iroh) would flood the buffer.
const OWN_CRATES: [&str; 3] = ["tonearm_core", "tonearm_ffi", "sdat_desktop"];

fn accepts(level: log::Level, target: &str, max: LevelFilter) -> bool {
    level <= max
        && (level <= log::Level::Info
            || OWN_CRATES.iter().any(|c| target.strip_prefix(c).is_some_and(|rest| rest.is_empty() || rest.starts_with("::"))))
}

struct Logger;

static LOGGER: Logger = Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        accepts(metadata.level(), metadata.target(), log::max_level())
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            ingest(record.level().into(), record.target(), &record.args().to_string(), None, true);
        }
    }

    fn flush(&self) {}
}

/// Records panics, backtrace included, and prints them to stderr in place of the default hook, which would
/// print the message unredacted. `GLib` callbacks abort on a panic (it cannot unwind through C), so the
/// event is already written when the process dies.
fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let message = info.to_string();
        let _ = writeln!(std::io::stderr(), "{}", redact::redact(&message));
        ingest(Level::Error, "panic", &message, Some(Backtrace::force_capture().to_string()), false);
    }));
}

static OWNS_LOGGER: AtomicBool = AtomicBool::new(false);

/// Whether `log` records reach diagnostics; only then is the `log` max level ours to change.
pub(super) fn owns_logger() -> bool {
    OWNS_LOGGER.load(Ordering::Relaxed)
}

pub(super) fn install() {
    // Another logger installed first keeps its place and its level; panics and `record` still work.
    if log::set_logger(&LOGGER).is_ok() {
        OWNS_LOGGER.store(true, Ordering::Relaxed);
        log::set_max_level(LevelFilter::Info);
    } else {
        ingest(Level::Warn, "diag", "another log logger was installed first: log records do not reach diagnostics", None, true);
    }
    install_panic_hook();
}

#[cfg(test)]
mod tests {
    use log::{Level, LevelFilter};

    use super::accepts;

    #[test]
    fn verbose_keeps_debug_from_own_crates_only() {
        assert!(accepts(Level::Debug, "tonearm_core::api::client", LevelFilter::Debug));
        assert!(accepts(Level::Debug, "sdat_desktop", LevelFilter::Debug));
        assert!(!accepts(Level::Debug, "hyper_util::client", LevelFilter::Debug));
        assert!(!accepts(Level::Debug, "tonearm_corex", LevelFilter::Debug));
        assert!(accepts(Level::Info, "iroh::magicsock", LevelFilter::Debug));
        assert!(!accepts(Level::Debug, "tonearm_core", LevelFilter::Info));
    }
}

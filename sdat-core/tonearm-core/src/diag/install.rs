use std::backtrace::Backtrace;

use log::{LevelFilter, Log, Metadata, Record};

use super::{Level, ingest};

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

/// Records panics, backtrace included, then runs the hook that was installed before. `GLib` callbacks
/// abort on a panic (it cannot unwind through C), so the event is already written when the process dies.
fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::force_capture().to_string();
        ingest(Level::Error, "panic", &info.to_string(), Some(backtrace), false);
        previous(info);
    }));
}

pub(super) fn install() {
    // Another logger installed first keeps its place; the panic hook and the buffer still work.
    let _ = log::set_logger(&LOGGER);
    log::set_max_level(LevelFilter::Info);
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

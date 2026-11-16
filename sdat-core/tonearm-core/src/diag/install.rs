use std::backtrace::Backtrace;

use log::{LevelFilter, Log, Metadata, Record};

use super::{Level, ingest};

struct Logger;

static LOGGER: Logger = Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
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

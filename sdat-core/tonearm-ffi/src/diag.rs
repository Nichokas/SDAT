//! Diagnostics log, stats and report, over `tonearm_core::diag`. Every function is synchronous and cheap.

use std::sync::Arc;

use tonearm_core::diag as core;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DiagLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<core::Level> for DiagLevel {
    fn from(l: core::Level) -> Self {
        match l {
            core::Level::Error => Self::Error,
            core::Level::Warn => Self::Warn,
            core::Level::Info => Self::Info,
            core::Level::Debug => Self::Debug,
            core::Level::Trace => Self::Trace,
        }
    }
}

impl From<DiagLevel> for core::Level {
    fn from(l: DiagLevel) -> Self {
        match l {
            DiagLevel::Error => Self::Error,
            DiagLevel::Warn => Self::Warn,
            DiagLevel::Info => Self::Info,
            DiagLevel::Debug => Self::Debug,
            DiagLevel::Trace => Self::Trace,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DiagEvent {
    /// Strictly increasing; pass the last one seen to `diag_events` for what came after.
    pub seq: u64,
    /// Unix ms.
    pub at_ms: i64,
    pub level: DiagLevel,
    pub target: String,
    pub message: String,
    pub backtrace: Option<String>,
}

impl From<&core::Event> for DiagEvent {
    fn from(e: &core::Event) -> Self {
        Self {
            seq: e.seq,
            at_ms: e.at_ms,
            level: e.level.into(),
            target: e.target.clone(),
            message: e.message.clone(),
            backtrace: e.backtrace.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum DiagNetCategory {
    Tidal,
    Images,
    LastFm,
    ListenBrainz,
    Other,
}

impl From<core::NetCategory> for DiagNetCategory {
    fn from(c: core::NetCategory) -> Self {
        match c {
            core::NetCategory::Tidal => Self::Tidal,
            core::NetCategory::Images => Self::Images,
            core::NetCategory::LastFm => Self::LastFm,
            core::NetCategory::ListenBrainz => Self::ListenBrainz,
            core::NetCategory::Other => Self::Other,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DiagNetStat {
    pub category: DiagNetCategory,
    pub requests: u64,
    /// Transport errors and HTTP statuses of 400 and above.
    pub failures: u64,
    pub bytes_in: u64,
    pub total_ms: u64,
    pub max_ms: u64,
    pub last_status: Option<u16>,
}

impl From<core::NetStat> for DiagNetStat {
    fn from(n: core::NetStat) -> Self {
        Self {
            category: n.category.into(),
            requests: n.requests,
            failures: n.failures,
            bytes_in: n.bytes_in,
            total_ms: n.total_ms,
            max_ms: n.max_ms,
            last_status: n.last_status,
        }
    }
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct DiagStats {
    pub uptime_ms: u64,
    pub cpu_user_ms: u64,
    pub cpu_system_ms: u64,
    /// Peak resident memory of the process.
    pub max_rss_kb: u64,
    pub net: Vec<DiagNetStat>,
}

impl From<core::Stats> for DiagStats {
    fn from(s: core::Stats) -> Self {
        Self {
            uptime_ms: s.uptime_ms,
            cpu_user_ms: s.cpu_user_ms,
            cpu_system_ms: s.cpu_system_ms,
            max_rss_kb: s.max_rss_kb,
            net: s.net.into_iter().map(Into::into).collect(),
        }
    }
}

/// Implemented by the app. Gets events of level warn and above, panics and marks, on any thread, with no
/// core lock held. Forward them to Sentry and the platform log.
#[uniffi::export(with_foreign)]
pub trait DiagSink: Send + Sync {
    fn on_event(&self, event: DiagEvent);
}

struct Bridge(Arc<dyn DiagSink>);

impl core::Sink for Bridge {
    fn on_event(&self, event: &core::Event) {
        self.0.on_event(event.into());
    }
}

/// Start collecting. Call first thing at launch; later calls do nothing. `dir` keeps the log files
/// (empty for none): the last run's log is then available from `diag_previous_session`.
#[uniffi::export]
pub fn diag_init(dir: String, sink: Option<Arc<dyn DiagSink>>) {
    let dir = (!dir.is_empty()).then(|| dir.into());
    core::init(dir, sink.map(|s| Box::new(Bridge(s)) as Box<dyn core::Sink>));
}

/// Adds an error or note from the app (a caught Kotlin or Swift exception) to the same log.
#[uniffi::export]
pub fn diag_record(level: DiagLevel, target: String, message: String, backtrace: Option<String>) {
    core::record(level.into(), &target, &message, backtrace);
}

/// Puts a note in the log at this moment and sends it to the sink.
#[uniffi::export]
pub fn diag_mark(note: String) {
    core::mark(&note);
}

/// Events with a `seq` above `since`, oldest first. The log keeps the last 2000.
#[uniffi::export]
pub fn diag_events(since: u64) -> Vec<DiagEvent> {
    core::events_since(since).iter().map(Into::into).collect()
}

/// Debug logging on or off.
#[uniffi::export]
pub fn diag_set_verbose(on: bool) {
    core::set_verbose(on);
}

#[uniffi::export]
pub fn diag_verbose() -> bool {
    core::verbose()
}

#[uniffi::export]
pub fn diag_stats() -> DiagStats {
    core::stats().into()
}

/// A plain-text report to share. `extra` goes first: app version, OS and device.
#[uniffi::export]
pub fn diag_report(extra: String) -> String {
    core::report(&extra)
}

/// The previous run's log, or an empty string.
#[uniffi::export]
pub fn diag_previous_session() -> String {
    core::previous_session()
}

/// Empties the log and the log files, the previous run's included.
#[uniffi::export]
pub fn diag_clear() {
    core::clear();
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct Collect(Mutex<Vec<DiagEvent>>);

    impl DiagSink for Collect {
        fn on_event(&self, event: DiagEvent) {
            self.0.lock().expect("sink lock").push(event);
        }
    }

    #[test]
    fn events_cross_the_boundary_in_both_directions() {
        let sink = Arc::new(Collect(Mutex::new(Vec::new())));
        diag_init(String::new(), Some(Arc::clone(&sink) as Arc<dyn DiagSink>));
        let before = diag_events(0).last().map_or(0, |e| e.seq);

        diag_record(DiagLevel::Error, "kotlin".into(), "IllegalStateException: boom".into(), Some("at Foo.bar".into()));
        diag_mark("track skipped".into());
        diag_record(DiagLevel::Debug, "swift".into(), "not sent".into(), None);

        let events = diag_events(before);
        assert_eq!(events.len(), 3);
        assert_eq!((events[0].level, events[0].target.as_str()), (DiagLevel::Error, "kotlin"));
        assert_eq!(events[0].backtrace.as_deref(), Some("at Foo.bar"));
        assert_eq!((events[1].level, events[1].target.as_str()), (DiagLevel::Info, "mark"));
        assert!(events[0].seq < events[1].seq && events[1].seq < events[2].seq);

        let sent = sink.0.lock().expect("sink lock");
        assert_eq!(sent.iter().map(|e| e.message.as_str()).collect::<Vec<_>>(), ["IllegalStateException: boom", "track skipped"]);
        drop(sent);

        assert_eq!(diag_stats().net.len(), 5);
        assert!(diag_report("SDAT test".into()).starts_with("SDAT test"));
    }
}

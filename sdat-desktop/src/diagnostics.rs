//! Crash reporting: the consent stored in the settings, the Sentry client that runs only while it is
//! allowed, and the diag sink that feeds it. The Diagnostics page lives in `ui::diagnostics`.

use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use sentry::protocol::{Attachment, Breadcrumb};
use tonearm_core::diag::{self, Event, Level, Sink};

use crate::settings::{self, Consent};
use crate::ui::window::{COMMIT, VERSION};

/// Set only by the release workflow; empty or missing in dev builds, local builds and forks.
const DSN: Option<&str> = option_env!("SENTRY_DSN");

/// How long closing the client waits for queued events to go out.
const FLUSH: Duration = Duration::from_secs(2);

static CLIENT: Mutex<Option<Arc<sentry::Client>>> = Mutex::new(None);

fn dsn() -> Option<&'static str> {
    DSN.map(str::trim).filter(|d| !d.is_empty())
}

/// Whether this build can send reports at all and keep the user's answer. When not, the prompt and the switch
/// stay hidden.
pub fn available() -> bool {
    dsn().is_some() && settings::crash_reports_supported()
}

pub fn running() -> bool {
    CLIENT.lock().unwrap_or_else(PoisonError::into_inner).is_some()
}

/// The version as Sentry's release and the shared report name it.
pub fn app_version() -> String {
    let version = VERSION.filter(|v| !v.is_empty()).unwrap_or(env!("CARGO_PKG_VERSION"));
    let version = version.strip_prefix('v').unwrap_or(version);
    if COMMIT.is_empty() || VERSION.is_some() { version.to_owned() } else { format!("{version}+{}", COMMIT.chars().take(7).collect::<String>()) }
}

fn pretty_os_name(os_release: &str) -> Option<String> {
    let value = os_release.lines().find_map(|l| l.strip_prefix("PRETTY_NAME="))?;
    let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
    (!value.is_empty()).then(|| value.to_owned())
}

pub fn os_name() -> String {
    ["/etc/os-release", "/usr/lib/os-release"]
        .iter()
        .find_map(|p| std::fs::read_to_string(p).ok().and_then(|s| pretty_os_name(&s)))
        .unwrap_or_else(|| std::env::consts::OS.to_owned())
}

/// The platform header of a shared report.
pub fn report_header() -> String {
    format!("SDAT desktop {}\nOS: {}", app_version(), os_name())
}

/// Makes Sentry match the stored consent: started when this build has a DSN and reports are allowed,
/// closed otherwise. Calling it again changes nothing.
pub fn apply() {
    let wanted = available() && settings::crash_reports() == Consent::Allowed;
    match (wanted, dsn()) {
        (true, Some(dsn)) => open(dsn),
        _ => close(),
    }
}

pub fn set_consent(consent: Consent) {
    settings::set_crash_reports(consent);
    apply();
}

fn open(dsn: &str) {
    if dsn.parse::<sentry::types::Dsn>().is_err() {
        log::warn!("the Sentry DSN of this build is not valid; crash reports stay off");
        return;
    }
    // Defaults on purpose: no default PII, no tracing, no sessions. The `panic` and `contexts` features of the
    // sentry crate are off: the diag panic hook already reports panics, and contexts would send the host name.
    let options = sentry::ClientOptions::new().dsn(dsn).release(format!("sdat-desktop@{}", app_version())).environment("production");
    bind(sentry::Client::from(sentry::apply_defaults(options)));
}

fn bind(client: sentry::Client) {
    let mut slot = CLIENT.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.is_some() {
        return;
    }
    let client = Arc::new(client);
    let hub = sentry::Hub::main();
    hub.bind_client(Some(Arc::clone(&client)));
    hub.configure_scope(|scope| scope.set_tag("os", os_name()));
    *slot = Some(client);
}

/// Also at shutdown, so reports queued near exit still go out.
pub fn close() {
    let Some(client) = CLIENT.lock().unwrap_or_else(PoisonError::into_inner).take() else { return };
    sentry::Hub::main().bind_client(None);
    // Not the client guard of `sentry::init`: its drop closes whatever client the dropping thread's hub holds.
    client.close(Some(FLUSH));
}

/// The sink handed to `diag::init`. It does nothing while Sentry is not running.
pub fn sink() -> Box<dyn Sink> {
    Box::new(SentrySink)
}

#[derive(Debug)]
struct SentrySink;

impl Sink for SentrySink {
    fn on_event(&self, event: &Event) {
        // Always the main hub: another thread's hub keeps the client it was created with, also after a restart.
        let main = sentry::Hub::main();
        if main.client().is_none() {
            return;
        }
        if event.level == Level::Error || event.backtrace.is_some() {
            // A hub of its own, so the tag and attachment of this event cannot leak into another thread's.
            let hub = sentry::Hub::new_from_top(main);
            hub.configure_scope(|scope| {
                scope.set_tag("target", &event.target);
                if let Some(trace) = &event.backtrace {
                    scope.add_attachment(Attachment {
                        buffer: trace.clone().into_bytes(),
                        filename: "backtrace.txt".into(),
                        content_type: Some("text/plain".into()),
                        ty: None,
                    });
                }
            });
            hub.capture_event(sentry::protocol::Event {
                message: Some(event.message.clone()),
                level: sentry::Level::Error,
                ..Default::default()
            });
            // The panic hook aborts right after this returns, before the transport thread could send.
            if event.target == "panic"
                && let Some(client) = hub.client()
            {
                client.flush(Some(FLUSH));
            }
        } else {
            main.add_breadcrumb(Breadcrumb {
                category: Some(event.target.clone()),
                message: Some(event.message.clone()),
                level: if event.level == Level::Warn { sentry::Level::Warning } else { sentry::Level::Info },
                ..Default::default()
            });
        }
    }
}

/// An error the Diagnostics page records on request, through the same path as every other one.
pub fn send_test_error() {
    diag::record(Level::Error, "diagnostics", "Test error from Diagnostics", Some(std::backtrace::Backtrace::force_capture().to_string()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use sentry::protocol::EnvelopeItem;
    use sentry::test::TestTransport;

    fn event(level: Level, target: &str, message: &str, backtrace: Option<&str>) -> Event {
        Event { seq: 1, at_ms: 0, level, target: target.into(), message: message.into(), backtrace: backtrace.map(Into::into) }
    }

    #[test]
    fn os_name_comes_from_pretty_name() {
        assert_eq!(pretty_os_name("NAME=Fedora\nPRETTY_NAME=\"Fedora Linux 43 (Workstation Edition)\"\n").as_deref(), Some("Fedora Linux 43 (Workstation Edition)"));
        assert_eq!(pretty_os_name("PRETTY_NAME='Ubuntu 25.10'").as_deref(), Some("Ubuntu 25.10"));
        assert_eq!(pretty_os_name("NAME=Fedora"), None);
        assert_eq!(pretty_os_name("PRETTY_NAME=\"\""), None);
    }

    #[test]
    fn the_sink_sends_errors_and_panics_and_keeps_warnings_and_marks_as_breadcrumbs() {
        let transport = TestTransport::new();
        let options = sentry::ClientOptions::new().dsn("https://key@example.invalid/1").transport(Arc::clone(&transport)).release("sdat-desktop@test");
        // Not `apply`: no setting, and the test transport replaces the network.
        bind(sentry::Client::from(options));
        assert!(running());

        let sink = SentrySink;
        sink.on_event(&event(Level::Warn, "net", "slow response", None));
        sink.on_event(&event(Level::Info, "mark", "skipped now", None));
        assert!(transport.fetch_and_clear_envelopes().is_empty(), "warnings and marks alone send nothing");
        sink.on_event(&event(Level::Error, "playback", "stream failed", None));
        sink.on_event(&event(Level::Warn, "panic", "boom", Some("0: main\n1: run")));

        let envelopes = transport.fetch_and_clear_envelopes();
        assert_eq!(envelopes.len(), 2);
        let first = envelopes[0].event().expect("an event");
        assert_eq!(first.message.as_deref(), Some("stream failed"));
        assert_eq!(first.level, sentry::Level::Error);
        assert_eq!(first.tags.get("target").map(String::as_str), Some("playback"));
        assert_eq!(first.release.as_deref(), Some("sdat-desktop@test"));
        let crumbs: Vec<_> = first.breadcrumbs.iter().map(|b| (b.category.clone().unwrap_or_default(), b.level)).collect();
        assert_eq!(crumbs, [("net".to_owned(), sentry::Level::Warning), ("mark".to_owned(), sentry::Level::Info)]);
        assert!(!envelopes[0].items().any(|i| matches!(i, EnvelopeItem::Attachment(_))), "no backtrace, no attachment");

        let panic = envelopes[1].event().expect("an event");
        assert_eq!(panic.tags.get("target").map(String::as_str), Some("panic"));
        let attachment = envelopes[1].items().find_map(|i| match i {
            EnvelopeItem::Attachment(a) => Some(a),
            _ => None,
        });
        let attachment = attachment.expect("the backtrace is attached");
        assert_eq!(attachment.filename, "backtrace.txt");
        assert_eq!(attachment.buffer, b"0: main\n1: run");

        close();
        assert!(!running());
        sink.on_event(&event(Level::Error, "playback", "after close", None));
        assert!(transport.fetch_and_clear_envelopes().is_empty(), "nothing is sent after the client closed");
    }
}

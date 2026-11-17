//! The Diagnostics page of the preferences dialog and the first-launch consent prompt.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

use adw::prelude::*;
use gtk::gio;
use tonearm_core::diag::{self, Event, Level, NetStat};

use crate::diagnostics;
use crate::i18n::{tr, trf};
use crate::settings::{self, Consent};

const REFRESH: Duration = Duration::from_secs(2);
/// Events kept in memory, and how many of them the list shows.
const KEPT_EVENTS: usize = 500;
const SHOWN_EVENTS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LevelFilter {
    All,
    WarningsAndErrors,
    Errors,
}

impl LevelFilter {
    const ALL: [Self; 3] = [Self::All, Self::WarningsAndErrors, Self::Errors];

    fn from_index(i: u32) -> Self {
        Self::ALL.get(i as usize).copied().unwrap_or(Self::All)
    }

    fn label(self) -> String {
        match self {
            Self::All => tr("All"),
            Self::WarningsAndErrors => tr("Warnings and errors"),
            Self::Errors => tr("Errors"),
        }
    }

    fn passes(self, level: Level) -> bool {
        match self {
            Self::All => true,
            Self::WarningsAndErrors => matches!(level, Level::Error | Level::Warn),
            Self::Errors => level == Level::Error,
        }
    }
}

fn duration_text(ms: u64) -> String {
    let secs = ms / 1000;
    match secs {
        0 => format!("{ms} ms"),
        1..=59 if ms < 10_000 => format!("{:.1} s", ms as f64 / 1000.0),
        1..=59 => format!("{secs} s"),
        60..=3599 => format!("{} min {} s", secs / 60, secs % 60),
        3600..=86_399 => format!("{} h {} min", secs / 3600, secs % 3600 / 60),
        _ => format!("{} d {} h", secs / 86_400, secs % 86_400 / 3600),
    }
}

fn bytes_text(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KiB", "MiB", "GiB", "TiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}

fn net_text(s: &NetStat) -> String {
    let average = s.total_ms.checked_div(s.requests).unwrap_or(0);
    let status = s.last_status.map_or_else(|| "–".to_owned(), |c| c.to_string());
    trf(
        "%d requests · %d failed · %s received · average %s · slowest %s · last HTTP status %s",
        &[&s.requests, &s.failures, &bytes_text(s.bytes_in), &duration_text(average), &duration_text(s.max_ms), &status],
    )
}

fn event_time(at_ms: i64) -> String {
    chrono::DateTime::from_timestamp_millis(at_ms).map_or_else(String::new, |t| t.with_timezone(&chrono::Local).format("%H:%M:%S").to_string())
}

fn toast(dialog: &adw::PreferencesDialog, msg: &str) {
    let t = adw::Toast::new(msg);
    t.set_timeout(3);
    dialog.add_toast(t);
}

fn property_row(title: &str) -> adw::ActionRow {
    let row = adw::ActionRow::builder().title(title).subtitle_selectable(true).build();
    row.add_css_class("property");
    row
}

fn action_row(title: &str, subtitle: &str) -> adw::ActionRow {
    adw::ActionRow::builder().title(title).subtitle(subtitle).activatable(true).build()
}

fn with_chevron(row: &adw::ActionRow) {
    row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
}

/// The row of the General page that opens the Diagnostics page.
pub fn entry_row(dialog: &adw::PreferencesDialog) -> adw::ActionRow {
    let row = action_row(&tr("Diagnostics"), &tr("Errors, network and processor use, and tools to find bugs"));
    with_chevron(&row);
    let d = dialog.downgrade();
    row.connect_activated(move |_| {
        if let Some(d) = d.upgrade() {
            d.push_subpage(&page(&d));
        }
    });
    row
}

struct Live {
    dialog: glib::WeakRef<adw::PreferencesDialog>,
    uptime: adw::ActionRow,
    cpu: adw::ActionRow,
    memory: adw::ActionRow,
    network: Vec<adw::ActionRow>,
    filter: adw::ComboRow,
    list: gtk::ListBox,
    events: RefCell<VecDeque<Event>>,
    last_seq: Cell<u64>,
    /// The list shows what `events` and `filter` hold.
    stale: Cell<bool>,
}

impl Live {
    fn refresh(&self) {
        let stats = diag::stats();
        self.uptime.set_subtitle(&duration_text(stats.uptime_ms));
        self.cpu.set_subtitle(&trf("%s user, %s system", &[&duration_text(stats.cpu_user_ms), &duration_text(stats.cpu_system_ms)]));
        self.memory.set_subtitle(&bytes_text(stats.max_rss_kb.saturating_mul(1024)));
        for (row, stat) in self.network.iter().zip(&stats.net) {
            row.set_subtitle(&net_text(stat));
        }

        let new = diag::events_since(self.last_seq.get());
        if let Some(last) = new.last() {
            self.last_seq.set(last.seq);
            let mut events = self.events.borrow_mut();
            events.extend(new);
            let excess = events.len().saturating_sub(KEPT_EVENTS);
            events.drain(..excess);
            self.stale.set(true);
        }
        if self.stale.replace(false) {
            self.rebuild_list();
        }
    }

    fn rebuild_list(&self) {
        // Rows only: the placeholder is a child of the list as well.
        while let Some(row) = self.list.row_at_index(0) {
            self.list.remove(&row);
        }
        let filter = LevelFilter::from_index(self.filter.selected());
        for event in self.events.borrow().iter().rev().filter(|e| filter.passes(e.level)).take(SHOWN_EVENTS) {
            self.list.append(&self.event_row(event));
        }
    }

    fn event_row(&self, event: &Event) -> adw::ActionRow {
        let title = event.message.lines().next().unwrap_or_default();
        let row = adw::ActionRow::builder()
            .title(title)
            .title_lines(2)
            .use_markup(false)
            .subtitle(format!("{} · {} · {}", event_time(event.at_ms), event.level.as_str(), event.target))
            .build();
        if event.backtrace.is_some() {
            row.set_activatable(true);
            with_chevron(&row);
            let (d, text) = (self.dialog.clone(), event.to_string());
            row.connect_activated(move |_| {
                if let Some(d) = d.upgrade() {
                    present_text(&d, &tr("Stack trace"), &text);
                }
            });
        }
        row
    }
}

/// A read-only, selectable monospace text in a dialog of its own.
fn present_text(parent: &adw::PreferencesDialog, title: &str, text: &str) {
    let view = gtk::TextView::builder().editable(false).cursor_visible(true).monospace(true).wrap_mode(gtk::WrapMode::WordChar).build();
    view.set_left_margin(12);
    view.set_right_margin(12);
    view.set_top_margin(12);
    view.set_bottom_margin(12);
    view.buffer().set_text(text);
    let scrolled = gtk::ScrolledWindow::builder().child(&view).vexpand(true).build();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scrolled));
    let dialog = adw::Dialog::builder().title(title).content_width(720).content_height(520).child(&toolbar).build();
    dialog.present(Some(parent));
}

fn page(dialog: &adw::PreferencesDialog) -> adw::NavigationPage {
    let prefs = adw::PreferencesPage::new();

    let overview = adw::PreferencesGroup::builder().title(tr("Overview")).build();
    let version = property_row(&tr("App version"));
    version.set_subtitle(&diagnostics::app_version());
    let os = property_row(&tr("Operating system"));
    os.set_subtitle(&diagnostics::os_name());
    let (uptime, cpu, memory) = (property_row(&tr("Uptime")), property_row(&tr("Processor time")), property_row(&tr("Peak memory")));
    for row in [&version, &os, &uptime, &cpu, &memory] {
        overview.add(row);
    }
    prefs.add(&overview);

    let net_group = adw::PreferencesGroup::builder().title(tr("Network")).description(tr("Requests since SDAT started")).build();
    let network: Vec<adw::ActionRow> = diag::NetCategory::ALL
        .iter()
        .map(|c| {
            let row = adw::ActionRow::builder().title(c.name()).build();
            net_group.add(&row);
            row
        })
        .collect();
    prefs.add(&net_group);

    let filter = adw::ComboRow::builder().title(tr("Show")).build();
    let labels: Vec<String> = LevelFilter::ALL.iter().map(|f| f.label()).collect();
    let labels: Vec<&str> = labels.iter().map(String::as_str).collect();
    filter.set_model(Some(&gtk::StringList::new(&labels)));
    let list = gtk::ListBox::builder().selection_mode(gtk::SelectionMode::None).css_classes(["boxed-list"]).build();
    list.set_placeholder(Some(&gtk::Label::builder().label(tr("No events")).margin_top(12).margin_bottom(12).css_classes(["dim-label"]).build()));
    let log = adw::PreferencesGroup::builder().title(tr("Log")).description(tr("Newest first. Open an entry with a stack trace to read it")).build();
    log.add(&filter);
    log.add(&list);
    prefs.add(&log);

    let live = Rc::new(Live {
        dialog: dialog.downgrade(),
        uptime,
        cpu,
        memory,
        network,
        filter: filter.clone(),
        list,
        events: RefCell::new(VecDeque::new()),
        last_seq: Cell::new(0),
        stale: Cell::new(true),
    });
    let l = Rc::downgrade(&live);
    filter.connect_selected_notify(move |_| {
        if let Some(l) = l.upgrade() {
            l.rebuild_list();
        }
    });

    let test_error = action_row(&tr("Send test error"), &tr("Records an error and sends it, to check that reports arrive"));
    test_error.connect_activated({
        let d = dialog.downgrade();
        move |_| {
            diagnostics::send_test_error();
            if let Some(d) = d.upgrade() {
                toast(&d, &tr("Test error sent"));
            }
        }
    });
    test_error.set_visible(diagnostics::running());
    prefs.add(&bug_hunting(dialog, &test_error));

    if diagnostics::available() {
        prefs.add(&crash_reports(&test_error));
    }

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&prefs));
    let nav = adw::NavigationPage::builder().title(tr("Diagnostics")).child(&toolbar).build();

    // Only while the page is on screen.
    let timer: Rc<RefCell<Option<glib::SourceId>>> = Rc::default();
    nav.connect_map({
        let (live, timer) = (Rc::clone(&live), Rc::clone(&timer));
        move |_| {
            live.refresh();
            let live = Rc::clone(&live);
            let id = glib::timeout_add_local(REFRESH, move || {
                live.refresh();
                glib::ControlFlow::Continue
            });
            if let Some(old) = timer.borrow_mut().replace(id) {
                old.remove();
            }
        }
    });
    nav.connect_unmap(move |_| {
        if let Some(id) = timer.borrow_mut().take() {
            id.remove();
        }
    });
    nav
}

fn bug_hunting(dialog: &adw::PreferencesDialog, test_error: &adw::ActionRow) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(tr("Bug hunting")).build();

    let verbose = adw::SwitchRow::builder().title(tr("Verbose logging")).subtitle(tr("Also record debug messages. Turns off when SDAT restarts")).active(diag::verbose()).build();
    verbose.connect_active_notify(|r| diag::set_verbose(r.is_active()));
    group.add(&verbose);

    let marker = action_row(&tr("Add marker"), &tr("Write a note into the log at this moment, for example when something just went wrong"));
    marker.connect_activated({
        let d = dialog.downgrade();
        move |_| {
            if let Some(d) = d.upgrade() {
                ask_marker(&d);
            }
        }
    });
    group.add(&marker);

    let previous = diag::previous_session();
    let previous_row = action_row(&tr("Previous session"), &tr("The log of the last time SDAT ran, useful after a crash"));
    previous_row.set_sensitive(!previous.is_empty());
    previous_row.connect_activated({
        let d = dialog.downgrade();
        move |_| {
            if let Some(d) = d.upgrade() {
                present_text(&d, &tr("Previous session"), &diag::previous_session());
            }
        }
    });
    group.add(&previous_row);

    let share = action_row(&tr("Share report"), &tr("Copy a report to the clipboard and save it to a file to attach to a bug report"));
    share.connect_activated({
        let d = dialog.downgrade();
        move |_| {
            if let Some(d) = d.upgrade() {
                share_report(&d);
            }
        }
    });
    group.add(&share);
    group.add(test_error);
    group
}

fn ask_marker(dialog: &adw::PreferencesDialog) {
    let ad = adw::AlertDialog::new(Some(&tr("Add marker")), Some(&tr("Write a short note. It is added to the log now.")));
    let entry = gtk::Entry::builder().activates_default(true).max_length(200).build();
    ad.set_extra_child(Some(&entry));
    ad.add_response("cancel", &tr("Cancel"));
    ad.add_response("add", &tr("Add"));
    ad.set_response_appearance("add", adw::ResponseAppearance::Suggested);
    ad.set_default_response(Some("add"));
    ad.set_close_response("cancel");
    ad.connect_response(None, move |_, response| {
        let note = entry.text();
        if response == "add" && !note.trim().is_empty() {
            diag::mark(note.trim());
        }
    });
    ad.present(Some(dialog));
}

fn share_report(dialog: &adw::PreferencesDialog) {
    let report = diag::report(&diagnostics::report_header());
    dialog.display().clipboard().set_text(&report);
    toast(dialog, &tr("Report copied to clipboard"));
    let file_dialog = gtk::FileDialog::builder().title(tr("Save report")).initial_name("sdat-report.txt").build();
    let d = dialog.downgrade();
    file_dialog.save(super::window::window().as_ref(), gio::Cancellable::NONE, move |result| {
        // Dismissing the chooser is an answer, not an error.
        let Ok(file) = result else { return };
        let saved = file.replace_contents(report.as_bytes(), None, false, gio::FileCreateFlags::NONE, gio::Cancellable::NONE);
        if let Some(d) = d.upgrade() {
            match saved {
                Ok(_) => toast(&d, &tr("Report saved")),
                Err(e) => {
                    log::warn!("failed to save the diagnostics report error={e}");
                    toast(&d, &tr("Could not save the report"));
                }
            }
        }
    });
}

fn crash_reports(test_error: &adw::ActionRow) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title(tr("Crash reports")).build();
    let row = adw::SwitchRow::builder()
        .title(tr("Send crash reports"))
        .subtitle(tr("If SDAT crashes or hits an error, send a report to the developer. Reports include the error, a stack trace, recent log entries, the app version and the OS version, but no account details."))
        .active(settings::crash_reports() == Consent::Allowed)
        .build();
    let test_error = test_error.downgrade();
    row.connect_active_notify(move |r| {
        diagnostics::set_consent(if r.is_active() { Consent::Allowed } else { Consent::Denied });
        if let Some(t) = test_error.upgrade() {
            t.set_visible(diagnostics::running());
        }
    });
    group.add(&row);
    group
}

/// Asks once, the first time the main window appears, when this build can send reports at all.
pub fn ask_consent_once(window: &adw::ApplicationWindow) {
    if !diagnostics::available() || settings::crash_reports() != Consent::Unset {
        return;
    }
    let ad = adw::AlertDialog::new(
        Some(&tr("Send crash reports?")),
        Some(&tr("If SDAT crashes or hits an error, send a report to the developer so it can be fixed. Reports include the error, a stack trace, recent log entries, the app version and the OS version, but no account details. You can change this in Settings > Diagnostics.")),
    );
    ad.add_response("deny", &tr("Don't send"));
    ad.add_response("allow", &tr("Send reports"));
    ad.set_response_appearance("allow", adw::ResponseAppearance::Suggested);
    ad.set_default_response(Some("allow"));
    // Escape counts as no: nothing is sent unless the user said so.
    ad.set_close_response("deny");
    ad.connect_response(None, |_, response| diagnostics::set_consent(if response == "allow" { Consent::Allowed } else { Consent::Denied }));
    ad.present(Some(window));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_use_the_two_largest_units() {
        assert_eq!(duration_text(0), "0 ms");
        assert_eq!(duration_text(340), "340 ms");
        assert_eq!(duration_text(1250), "1.2 s");
        assert_eq!(duration_text(42_000), "42 s");
        assert_eq!(duration_text(125_000), "2 min 5 s");
        assert_eq!(duration_text(3 * 3_600_000 + 7 * 60_000), "3 h 7 min");
        assert_eq!(duration_text(2 * 86_400_000 + 5 * 3_600_000), "2 d 5 h");
    }

    #[test]
    fn byte_sizes_scale_to_binary_units() {
        assert_eq!(bytes_text(0), "0 B");
        assert_eq!(bytes_text(1023), "1023 B");
        assert_eq!(bytes_text(1536), "1.5 KiB");
        assert_eq!(bytes_text(5 * 1024 * 1024), "5.0 MiB");
        assert_eq!(bytes_text(u64::MAX), "16777216.0 TiB");
    }

    #[test]
    fn the_filter_keeps_the_levels_it_names() {
        assert!(LevelFilter::All.passes(Level::Trace));
        assert!(LevelFilter::WarningsAndErrors.passes(Level::Warn));
        assert!(LevelFilter::WarningsAndErrors.passes(Level::Error));
        assert!(!LevelFilter::WarningsAndErrors.passes(Level::Info));
        assert!(LevelFilter::Errors.passes(Level::Error));
        assert!(!LevelFilter::Errors.passes(Level::Warn));
        assert_eq!(LevelFilter::from_index(2), LevelFilter::Errors);
        assert_eq!(LevelFilter::from_index(9), LevelFilter::All);
    }
}

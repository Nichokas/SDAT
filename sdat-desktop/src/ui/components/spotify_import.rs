//! "Import Spotify Playlist…" in the main menu: copies a public Spotify playlist
//! into a new TIDAL playlist.

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use adw::prelude::*;
use tonearm_core::playlist_import::{self, Progress, Report, Stage};
use tonearm_core::{CoreError, spotify};

use crate::i18n::{tr, trf};
use crate::ui::window::toast;
use crate::{api, router};

/// Asks for the playlist link, then imports it.
pub fn present(window: &adw::ApplicationWindow) {
    let ad = adw::AlertDialog::new(
        Some(&tr("Import Spotify Playlist")),
        Some(&tr("Paste the link of a public Spotify playlist. SDAT creates a TIDAL playlist with the tracks TIDAL has.")),
    );
    let entry = gtk::Entry::builder().activates_default(true).placeholder_text("https://open.spotify.com/playlist/…").width_chars(40).build();
    ad.set_extra_child(Some(&entry));
    ad.add_response("cancel", &tr("Cancel"));
    ad.add_response("import", &tr("Import"));
    ad.set_response_appearance("import", adw::ResponseAppearance::Suggested);
    ad.set_response_enabled("import", false);
    ad.set_default_response(Some("import"));
    ad.set_close_response("cancel");
    let dialog = ad.downgrade();
    entry.connect_changed(move |e| {
        if let Some(d) = dialog.upgrade() {
            d.set_response_enabled("import", spotify::playlist_id(&e.text()).is_some());
        }
    });
    let w = window.downgrade();
    ad.connect_response(None, move |_, response| {
        if response == "import"
            && let Some(w) = w.upgrade()
        {
            run(&w, entry.text().to_string());
        }
    });
    ad.present(Some(window));
}

fn run(window: &adw::ApplicationWindow, link: String) {
    let bar = gtk::ProgressBar::builder().show_text(true).text(stage_label(Stage::Reading)).build();
    let ad = adw::AlertDialog::new(Some(&tr("Importing from Spotify")), None);
    ad.set_extra_child(Some(&bar));
    ad.add_response("cancel", &tr("Cancel"));
    ad.set_close_response("cancel");
    let cancelled = Arc::new(AtomicBool::new(false));
    let c = Arc::clone(&cancelled);
    ad.connect_response(None, move |_, _| c.store(true, Ordering::SeqCst));
    ad.present(Some(window));

    // The import reports from its worker thread; the bar follows it from the main loop.
    let progress: Arc<Mutex<Option<Progress>>> = Arc::default();
    let shown = Arc::clone(&progress);
    let dialog = ad.downgrade();
    let last_us = Cell::new(None);
    let last_stage = Cell::new(None);
    let last_fraction = Cell::new(None::<f64>);
    let ticker = bar.add_tick_callback(move |bar, clock| {
        let now = clock.frame_time();
        let dt_secs = last_us.replace(Some(now)).map_or(1.0 / 60.0, |prev| (now.saturating_sub(prev) as f64 / 1_000_000.0).clamp(0.0, 0.05));
        let current = *shown.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(p) = current {
            if last_stage.get() != Some(p.stage) {
                bar.set_text(Some(&stage_label(p.stage)));
                // Saving always finishes, so that a cancel never leaves half a playlist.
                if let Some(d) = dialog.upgrade() {
                    d.set_response_enabled("cancel", p.stage != Stage::Saving);
                }
                last_stage.set(Some(p.stage));
            }
            if p.total > 0 {
                let frac = p.done as f64 / p.total as f64;
                if last_fraction.get().is_none_or(|prev| (prev - frac).abs() > f64::EPSILON) {
                    bar.set_fraction(frac);
                    last_fraction.set(Some(frac));
                }
                return glib::ControlFlow::Continue;
            }
        }
        last_fraction.set(None);
        bar.set_pulse_step((dt_secs * 0.67).clamp(0.001, 0.1));
        bar.pulse();
        glib::ControlFlow::Continue
    });

    let window = window.downgrade();
    glib::spawn_future_local(async move {
        let result = api::run(move || {
            playlist_import::import_spotify_playlist(
                &link,
                &|p| *progress.lock().unwrap_or_else(PoisonError::into_inner) = Some(p),
                &|| cancelled.load(Ordering::SeqCst),
            )
        })
        .await;
        ticker.remove();
        ad.force_close();
        // A failed import may have created its playlist too.
        super::add_to_playlist::forget();
        match result {
            Ok(report) => {
                router::navigate(&format!("playlist/{}", report.playlist_id));
                match window.upgrade() {
                    Some(w) if !report.missing.is_empty() || report.skipped > 0 => present_left_out(&w, &report),
                    _ => toast(&summary(&report)),
                }
            }
            Err(e) => match e.downcast_ref::<CoreError>() {
                Some(CoreError::Cancelled(_)) => {}
                // The core words the import's own failures (a private playlist, nothing found) for the user.
                Some(CoreError::Other(message)) => toast(message),
                Some(CoreError::NotSignedIn) => toast(&tr("Sign in to TIDAL to import a playlist")),
                _ => {
                    log::error!("importing a Spotify playlist failed error={e:#}");
                    toast(&tr("Could not import the playlist"));
                }
            },
        }
    });
}

fn stage_label(stage: Stage) -> String {
    match stage {
        Stage::Reading => tr("Reading from Spotify…"),
        Stage::Matching => tr("Finding tracks on TIDAL…"),
        Stage::Saving => tr("Creating the playlist…"),
    }
}

fn summary(report: &Report) -> String {
    trf("Imported %s tracks into %s", &[&report.imported, &report.name])
}

/// What the new playlist lacks: the tracks TIDAL does not have, so the user can look for
/// them by hand, and how many Spotify entries were not tracks.
fn present_left_out(window: &adw::ApplicationWindow, report: &Report) {
    let mut body = Vec::new();
    if report.skipped > 0 {
        body.push(trf("%s entries were skipped: local files, podcast episodes or tracks Spotify no longer has.", &[&report.skipped]));
    }
    if !report.missing.is_empty() {
        body.push(trf("%s tracks are not on TIDAL:", &[&report.missing.len()]));
    }
    let ad = adw::AlertDialog::new(Some(&summary(report)), Some(&body.join("\n\n")));
    if !report.missing.is_empty() {
        let list = gtk::Label::builder().label(report.missing.join("\n")).selectable(true).xalign(0.0).wrap(true).build();
        let scroll = gtk::ScrolledWindow::builder().child(&list).min_content_height(200).hscrollbar_policy(gtk::PolicyType::Never).build();
        ad.set_extra_child(Some(&scroll));
    }
    ad.add_response("close", &tr("Close"));
    ad.present(Some(window));
}

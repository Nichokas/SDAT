//! Thin typed wrappers around the `GSettings` schemas from
//! `internal/settings/dev.dergs.Tonearm.gschema.xml` (unchanged keys).

use gtk::gio;
use gtk::gio::prelude::*;

use crate::model::TrackQuality;

pub const APP_ID: &str = "dev.dergs.Tonearm";

thread_local! {
    static GENERAL: gio::Settings = gio::Settings::new(APP_ID);
    static PLAYBACK: gio::Settings = gio::Settings::new("dev.dergs.Tonearm.playback");
    static PERFORMANCE: gio::Settings = gio::Settings::new("dev.dergs.Tonearm.performance");
    static PLAYER: gio::Settings = gio::Settings::new("dev.dergs.Tonearm.player");
    static SCROBBLING: gio::Settings = gio::Settings::new("dev.dergs.Tonearm.scrobbling");
    static SERVICE_TIDAL: gio::Settings = gio::Settings::new("dev.dergs.Tonearm.service.tidal");
}

pub fn general() -> gio::Settings {
    GENERAL.with(std::clone::Clone::clone)
}
pub fn playback() -> gio::Settings {
    PLAYBACK.with(std::clone::Clone::clone)
}
pub fn performance() -> gio::Settings {
    PERFORMANCE.with(std::clone::Clone::clone)
}
pub fn player() -> gio::Settings {
    PLAYER.with(std::clone::Clone::clone)
}
pub fn scrobbling() -> gio::Settings {
    SCROBBLING.with(std::clone::Clone::clone)
}
pub fn service_tidal() -> gio::Settings {
    SERVICE_TIDAL.with(std::clone::Clone::clone)
}

/// Binds a key the same way the Go code does (`GSettingsBindNoSensitivity`).
pub fn bind(settings: &gio::Settings, key: &str, target: &impl IsA<glib::Object>, property: &str) {
    settings.bind(key, target, property).flags(gio::SettingsBindFlags::DEFAULT | gio::SettingsBindFlags::NO_SENSITIVITY).build();
}

// ---- service.tidal ----

/// The core's view of these settings (`tonearm_core::config`).
#[derive(Debug, Clone, Copy)]
pub struct GSettingsConfig;
impl tonearm_core::config::Config for GSettingsConfig {
    fn api_base_url(&self) -> String {
        api_base_url()
    }
    fn openapi_base_url(&self) -> String {
        openapi_base_url()
    }
    fn resources_base_url(&self) -> String {
        resources_base_url()
    }
    fn auth_base_url(&self) -> String {
        auth_base_url()
    }
    fn audio_quality(&self) -> TrackQuality {
        audio_quality()
    }
    fn config_dir(&self) -> Option<std::path::PathBuf> {
        Some(glib::user_config_dir().join("tonearm"))
    }
}

pub fn api_base_url() -> String {
    service_tidal().string("api-baseurl").into()
}
pub fn openapi_base_url() -> String {
    service_tidal().string("openapi-baseurl").into()
}
pub fn resources_base_url() -> String {
    service_tidal().string("resources-baseurl").into()
}
pub fn auth_base_url() -> String {
    service_tidal().string("auth-baseurl").into()
}

// ---- general ----
pub fn should_run_in_background() -> bool {
    general().boolean("allow-running-in-background")
}
pub fn default_page() -> String {
    general().string("default-page").into()
}
/// `GSettings` writes only fail for a read-only or invalid key; log instead of
/// ignoring the error (Go ignored it silently).
fn warn_on_err(key: &str, r: Result<(), glib::BoolError>) {
    if let Err(e) = r {
        log::warn!("failed to write setting {key}: {e}");
    }
}

pub fn set_default_page(p: &str) {
    warn_on_err("default-page", general().set_string("default-page", p));
}
pub fn should_hide_secret_service_warning() -> bool {
    general().boolean("hide-secret-service-warning")
}
pub fn set_hide_secret_service_warning(v: bool) {
    warn_on_err("hide-secret-service-warning", general().set_boolean("hide-secret-service-warning", v));
}
pub fn window_width() -> i32 {
    general().int("window-width")
}
pub fn window_height() -> i32 {
    general().int("window-height")
}
pub fn set_window_size(w: i32, h: i32) {
    warn_on_err("window-width", general().set_int("window-width", w));
    warn_on_err("window-height", general().set_int("window-height", h));
}

// ---- performance ----
pub fn allow_mediacard_images() -> bool {
    performance().boolean("allow-mediacard-images")
}
pub fn allow_shortcut_images() -> bool {
    performance().boolean("allow-shortcuts-images")
}
pub fn allow_tracklist_images() -> bool {
    performance().boolean("allow-tracklist-images")
}
pub fn should_cache_images() -> bool {
    performance().boolean("cache-images")
}
pub fn max_router_history_size() -> i32 {
    performance().int("max-router-history-size")
}

// ---- playback ----
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayGainMode {
    Auto = 0,
    Album = 1,
    Track = 2,
}

pub fn replay_gain_mode_strings() -> Vec<String> {
    vec![crate::i18n::tr("Auto"), crate::i18n::tr("Album"), crate::i18n::tr("Track")]
}

pub fn allow_autoplay() -> bool {
    playback().boolean("allow-autoplay")
}
pub fn replay_gain_mode() -> ReplayGainMode {
    match playback().int("replay-gain-mode") {
        1 => ReplayGainMode::Album,
        2 => ReplayGainMode::Track,
        _ => ReplayGainMode::Auto,
    }
}
pub fn set_replay_gain_mode(m: ReplayGainMode) {
    warn_on_err("replay-gain-mode", playback().set_int("replay-gain-mode", m as i32));
}

// ---- player ----
pub fn audio_quality() -> TrackQuality {
    let q: String = player().string("audio-quality").into();
    TrackQuality::parse(&q).unwrap_or_else(|| {
        log::error!("configured audio quality was invalid, defaulting to maximum configured={q}");
        TrackQuality::HighResLossless
    })
}
pub fn set_audio_quality(q: TrackQuality) {
    warn_on_err("audio-quality", player().set_string("audio-quality", q.as_str()));
}
pub fn volume() -> f64 {
    player().double("volume")
}
pub fn set_volume(v: f64) {
    warn_on_err("volume", player().set_double("volume", v));
}

// ---- scrobbling ----
pub fn enable_listenbrainz() -> bool {
    scrobbling().boolean("enable-listenbrainz")
}
pub fn listenbrainz_token() -> String {
    scrobbling().string("listenbrainz-token").into()
}
pub fn listenbrainz_url() -> String {
    scrobbling().string("listenbrainz-url").into()
}
pub fn enable_lastfm() -> bool {
    scrobbling().boolean("enable-lastfm")
}
pub fn lastfm_token() -> String {
    scrobbling().string("lastfm-token").into()
}
pub fn set_lastfm_token(t: &str) {
    warn_on_err("lastfm-token", scrobbling().set_string("lastfm-token", t));
}

/// Calls `cb` immediately and again every time `key` changes (main thread only).
pub fn connect_changed(settings: &gio::Settings, key: &'static str, cb: impl Fn(&gio::Settings) + 'static) -> glib::SignalHandlerId {
    cb(settings);
    settings.connect_changed(Some(key), move |s, _| cb(s))
}

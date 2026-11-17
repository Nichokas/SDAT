//! Tonearm, an unofficial GTK4 / libadwaita client for TIDAL.
//!
//! Rust port of the Go application; the UI, features and TIDAL integration
//! mirror the original one-to-one.

mod api;
mod cache;
mod diagnostics;
mod handoff;
mod imgutil;
mod live;
mod player;
mod router;
mod scrobbling;
mod secrets;
mod service;
mod migrate;
mod settings;
mod ui;

use tonearm_core::{country, i18n, model, signals, state};
pub(crate) use tonearm_core::global_signal;

use adw::prelude::*;
use gtk::gio;

static ICONS: &[u8] = include_bytes!("../internal/icons/icons.gresource");
static STYLES: &[u8] = include_bytes!("../internal/styles/styles.gresource");

pub(crate) fn register_resources() {
    for data in [ICONS, STYLES] {
        let r = gio::Resource::from_data(&glib::Bytes::from_static(data)).expect("Failed to create resources");
        gio::resources_register(&r);
    }
}

/// Falls back to the schema compiled at build time when it is not installed.
///
/// This must run before anything touches `gio::SettingsSchemaSource::default()`:
/// `GLib` reads `GSETTINGS_SCHEMA_DIR` once, when the default source is first built.
/// `GSETTINGS_SCHEMA_DIR` is searched in addition to the installed schemas, and the
/// build-time directory only exists on the machine that built the binary, so packaged
/// installs are unaffected.
pub(crate) fn ensure_schema() {
    let dir = env!("TONEARM_SCHEMA_DIR");
    if std::env::var_os("GSETTINGS_SCHEMA_DIR").is_none() && std::path::Path::new(dir).join("gschemas.compiled").is_file() {
        // SAFETY: single threaded at this point (before GTK and any worker starts).
        unsafe { std::env::set_var("GSETTINGS_SCHEMA_DIR", dir) };
    }
}

fn on_activate(app: &adw::Application) {
    let window = ui::window::new_window(app);
    window.present();
    window.connect_close_request(|w| {
        // Only run in background if a track is loaded, so MPRIS can bring it back.
        if settings::should_run_in_background() && player::track_changed().get().is_some() {
            w.set_visible(false);
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    if let Some(err) = secrets::healthcheck() {
        log::error!("Secret service health check failed title={} fatal={}", err.title, err.fatal);
        ui::window::present_secret_service_error(&window, &err);
    }
}

fn main() -> glib::ExitCode {
    tonearm_core::diag::init(Some(glib::user_config_dir().join(settings::CONFIG_DIR_NAME)), Some(diagnostics::sink()));
    // One level for everything: per-crate directives are ignored, and debug or trace means our own crates.
    if let Ok(value) = std::env::var("RUST_LOG") {
        match value.trim().parse::<log::LevelFilter>() {
            Ok(level) => log::set_max_level(level),
            Err(_) => log::warn!("RUST_LOG={value} ignored: only one level for everything, such as debug, is supported"),
        }
    }
    ensure_schema();
    migrate::run();
    diagnostics::apply();
    tonearm_core::config::install(Box::new(settings::GSettingsConfig));
    tonearm_core::auth::install(Box::new(secrets::KeyringTokens));

    register_resources();
    let app = adw::Application::new(Some(settings::APP_ID), gio::ApplicationFlags::HANDLES_COMMAND_LINE);
    // Bundled icons/styles still live under the original resource prefix; GTK would
    // otherwise derive it from the new app ID and miss them.
    app.set_resource_base_path(Some("/dev/dergs/Tonearm"));
    app.connect_startup(|_| {
        ui::components::tracklist_header::init();
        ui::components::sidebar::player_view::init();
        ui::components::sidebar::lyrics::init();
        ui::routes::register();
        if let Err(e) = player::init() {
            log::error!("failed to initialise the player error={e:#}");
        }
        player::mpris::init();
        scrobbling::init();
        handoff::init();
    });
    app.connect_shutdown(|_| {
        player::shutdown();
        handoff::stop();
        // Detach before the process ends, or other devices list this one for 90 s.
        tonearm_core::handoff::wait_detached(std::time::Duration::from_secs(2));
    });
    app.connect_activate(on_activate);
    app.connect_command_line(|app, cmd| {
        let args = cmd.arguments();
        if ui::window::window().is_none() {
            app.activate();
        } else if let Some(w) = ui::window::window() {
            w.set_visible(true);
            w.present();
        }
        if args.len() == 2 {
            let url = args[1].to_string_lossy().to_string();
            match live::link::classify(&url) {
                live::link::Arg::Join(link) => live::join(&link),
                live::link::Arg::BadInvitation(e) => {
                    log::warn!("ignoring invalid Listen Together link error={e}");
                    player::toasts().notify(&i18n::tr("This invitation link is not valid"));
                }
                live::link::Arg::TidalTrack(id) => {
                    glib::spawn_future_local(async move {
                        if let Err(e) = player::play_track_id(&id).await {
                            log::error!("failed to play track error={e:#}");
                        }
                    });
                }
                live::link::Arg::Route(route) => router::navigate(&route),
            }
        }
        glib::ExitCode::SUCCESS
    });
    app.run()
}

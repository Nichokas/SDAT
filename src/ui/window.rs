//! Port of `internal/ui/window*.go`: the main window, its actions, dialogs and popups.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use adw::prelude::*;
use gtk::gio;

use crate::i18n::{tr, trf};
use crate::ui::accels::PRIMARY;
use crate::ui::components::sidebar::{lyrics, player_view, queue};
use crate::ui::components::{RouteButton, linking, media_viewer};
use crate::ui::dsl::{on_signal, clamp, W, spacing, hstack, on_state, vstack};
use crate::{api, model, player, router, secrets, service, settings, state};

thread_local! {
    static WINDOW: RefCell<Option<adw::ApplicationWindow>> = const { RefCell::new(None) };
}

pub fn window() -> Option<adw::ApplicationWindow> {
    WINDOW.with(|w| w.borrow().clone())
}

pub fn toast(msg: &str) {
    player::toasts().notify(&msg.to_string());
}

fn accel(app: &adw::Application, action: &str, accels: &[String]) {
    let a: Vec<&str> = accels.iter().map(std::string::String::as_str).collect();
    app.set_accels_for_action(action, &a);
}

fn p(k: &str) -> String {
    format!("{PRIMARY}{k}")
}

fn simple(name: &str, f: impl Fn() + 'static) -> gio::SimpleAction {
    let a = gio::SimpleAction::new(name, None);
    a.connect_activate(move |_, _| f());
    a
}

fn with_string(name: &str, f: impl Fn(String) + 'static) -> gio::SimpleAction {
    let a = gio::SimpleAction::new(name, Some(glib::VariantTy::STRING));
    a.connect_activate(move |_, v| {
        if let Some(s) = v.and_then(|v| v.str()) {
            f(s.to_string());
        }
    });
    a
}

fn spawn_logged(what: &'static str, fut: impl std::future::Future<Output = anyhow::Result<()>> + 'static) {
    glib::spawn_future_local(async move {
        if let Err(e) = fut.await {
            log::error!("{what} failed error={e:#}");
        }
    });
}

// ------------------------------------------------------------------ version

const COMMIT: &str = env!("TONEARM_COMMIT");
const VERSION: Option<&str> = option_env!("TONEARM_VERSION");

fn describe_parts(commit: &str) -> Option<(String, String, String)> {
    let re = regex::Regex::new(r"^(.*)-(\d+)-g([0-9a-f]{7})").unwrap();
    let c = re.captures(commit)?;
    Some((c[1].to_string(), c[2].to_string(), c[3].to_string()))
}

pub fn is_stable(version: Option<&str>, commit: &str) -> bool {
    if version.is_some_and(|v| !v.is_empty()) {
        return true;
    }
    matches!(describe_parts(commit), Some((_, off, _)) if off == "0")
}

fn version_prefix(v: &str) -> &'static str {
    let b = v.as_bytes();
    if b.first() == Some(&b'v') {
        if b.get(1) == Some(&b'0') { "α " } else { "" }
    } else if b.first() == Some(&b'0') {
        "α "
    } else {
        "v"
    }
}

pub fn version_number(version: Option<&str>, commit: &str) -> String {
    match version.filter(|v| !v.is_empty()) {
        None => {
            if commit.is_empty() {
                return "local".into();
            }
            if let Some((tag, off, hash)) = describe_parts(commit) {
                let tag = tag.strip_prefix('v').unwrap_or(&tag).to_string();
                let suffix = if off == "0" { "" } else { "β" };
                format!("{}{} {}(r{}.{})", version_prefix(&tag), tag, suffix, off, hash)
            } else {
                format!("α git+{}", &commit[..commit.len().min(7)])
            }
        }
        Some(v) => {
            let suffix = if commit.is_empty() { String::new() } else { format!(" ({})", &commit[..commit.len().min(7)]) };
            let v = v.strip_prefix('v').unwrap_or(v);
            format!("{}{}{}", version_prefix(v), v, suffix)
        }
    }
}

// ------------------------------------------------------------------ build

pub fn new_window(app: &adw::Application) -> adw::ApplicationWindow {
    let window = adw::ApplicationWindow::new(app);
    WINDOW.with(|w| *w.borrow_mut() = Some(window.clone()));

    install_actions(app, &window);
    install_mouse_click_handler(&window);

    window.set_content(Some(&build(app, &window)));
    window.set_title(Some("Tonearm"));
    window.set_icon_name(Some("logo-symbolic"));
    window.set_default_size(settings::window_width(), settings::window_height());
    let save = |w: &adw::ApplicationWindow| {
        let (dw, dh) = w.default_size();
        let width = if w.width() > 0 { w.width() } else { dw };
        let height = if w.height() > 0 { w.height() } else { dh };
        if width > 0 && height > 0 {
            settings::set_window_size(width, height);
        }
    };
    window.connect_default_width_notify(save);
    window.connect_default_height_notify(save);

    router::navigate(&settings::default_page());

    if !is_stable(VERSION, COMMIT) {
        window.add_css_class("devel");
    }
    window
}

fn build(app: &adw::Application, window: &adw::ApplicationWindow) -> gtk::Widget {
    let layout = adw::OverlaySplitView::new();
    layout.set_sidebar(Some(&build_sidebar_layout(app, window)));
    layout.set_content(Some(&build_content_layout()));
    layout.set_sidebar_width_fraction(0.4);
    layout.set_max_sidebar_width(420.0);
    layout.set_min_sidebar_width(320.0);

    let sidebar_action = gio::SimpleAction::new_stateful("toggle-sidebar", None, &true.to_variant());
    let l = layout.downgrade();
    sidebar_action.connect_activate(move |a, _| {
        let new = !a.state().and_then(|s| s.get::<bool>()).unwrap_or(true);
        a.set_state(&new.to_variant());
        if let Some(l) = l.upgrade() {
            l.set_show_sidebar(new);
        }
    });
    window.add_action(&sidebar_action);
    accel(app, "win.toggle-sidebar", &[p("B"), "F9".into()]);

    let mv = gtk::Overlay::new();
    mv.set_child(Some(&layout));
    mv.add_overlay(media_viewer::get().widget());

    let toasts = adw::ToastOverlay::new();
    toasts.set_child(Some(&mv));
    on_signal(player::toasts(), &toasts, |o, title| {
        let t = adw::Toast::new(title);
        t.set_timeout(3);
        o.add_toast(t);
    });
    toasts.upcast()
}

fn loading_view() -> gtk::Widget {
    thread_local! {
        static V: gtk::Widget = clamp(50, &adw::Spinner::new()).upcast();
    }
    V.with(std::clone::Clone::clone)
}

fn build_content_layout() -> gtk::Widget {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&build_content_header());
    on_signal(router::navigation(), &tv, |tv, ev| {
        let lv = loading_view();
        if let Some(p) = lv.parent()
            && let Ok(old) = p.downcast::<adw::ToolbarView>()
        {
            old.set_content(gtk::Widget::NONE);
        }
        if ev.completed {
            let v = ev.result.as_ref().and_then(|r| r.view.clone());
            if let Some(v) = &v
                && let Some(p) = v.parent()
                && p != *tv.upcast_ref::<gtk::Widget>()
            {
                v.unparent();
            }
            tv.set_content(v.as_ref());
        } else {
            tv.set_content(Some(&lv));
        }
    });
    tv.upcast()
}

fn build_content_header() -> gtk::Widget {
    let home = RouteButton::new("home", true).title(&tr("Home")).icon("go-home-symbolic").tooltip(&tr("Navigate to Home"));
    let explore = RouteButton::new("explore", true).title(&tr("Explore")).icon("compass2-symbolic").tooltip(&tr("Navigate to Explore"));
    let collection = RouteButton::new("my-collection", true)
        .title(&tr("Collection"))
        .icon("library-symbolic")
        .tooltip(&tr("Navigate to Collection"));
    let default_toolbar = spacing(hstack(&[home.button.upcast_ref(), explore.button.upcast_ref(), collection.button.upcast_ref()]), 3);

    let header = adw::HeaderBar::new();
    header.set_show_start_title_buttons(false);
    let toggle = gtk::Button::from_icon_name("dock-left-symbolic").tooltip(&tr("Toggle Sidebar"));
    toggle.set_action_name(Some("win.toggle-sidebar"));
    let back = gtk::Button::from_icon_name("left-symbolic").visible(false).tooltip(&tr("Navigate Back"));
    back.set_action_name(Some("win.navigate-back"));
    on_signal(router::history_updated(), &back, |b, n| b.set_visible(*n > 0));
    header.pack_start(&toggle);
    header.pack_start(&back);
    header.set_title_widget(Some(&default_toolbar));

    on_signal(router::navigation(), &header, move |h, ev| {
        let tb = if ev.completed { ev.result.as_ref().and_then(|r| r.toolbar.clone()) } else { None };
        match tb {
            Some(t) => {
                if t.parent().is_some() && t.parent().as_ref() != Some(h.upcast_ref()) {
                    t.unparent();
                }
                h.set_title_widget(Some(&t));
            }
            None => h.set_title_widget(Some(&default_toolbar)),
        }
    });
    header.upcast()
}

fn build_sidebar_header(app: &adw::Application, window: &adw::ApplicationWindow) -> gtk::Widget {
    let title = adw::WindowTitle::new("Tonearm", "");
    let w = window.downgrade();
    on_signal(router::navigation(), &title, move |_, ev| {
        if ev.completed
            && let (Some(w), Some(r)) = (w.upgrade(), ev.result.as_ref())
        {
            w.set_title(Some(&format!("Tonearm - {}", r.page_title)));
        }
    });

    let menu = gio::Menu::new();
    menu.append(Some(&tr("Sign In…")), Some("win.sign-in"));
    menu.append(Some(&tr("Set as Default Page")), Some("win.set-as-default"));
    menu.append(Some(&tr("Keyboard Shortcuts")), Some("app.shortcuts"));
    menu.append(Some(&tr("Preferences")), Some("app.preferences"));
    menu.append(Some(&tr("About Tonearm")), Some("app.about"));
    menu.append(Some(&tr("Quit")), Some("app.quit"));
    on_state(state::signed_in_changed(), &menu, |m, signed_in| {
        m.remove(0);
        if *signed_in {
            m.insert(0, Some(&tr("Sign Out")), Some("win.sign-out"));
        } else {
            m.insert(0, Some(&tr("Sign In…")), Some("win.sign-in"));
        }
    });

    let header = adw::HeaderBar::new();
    header.set_title_widget(Some(&title));
    header.set_show_back_button(false);
    header.set_show_end_title_buttons(false);
    header.set_centering_policy(adw::CenteringPolicy::Strict);
    header.pack_start(&RouteButton::new("search", false).icon("loupe-symbolic").tooltip(&tr("Search")).button);

    let mb = gtk::MenuButton::new();
    mb.set_icon_name("menu-symbolic");
    mb.set_menu_model(Some(&menu));
    mb.set_tooltip_text(Some(&tr("Main Menu")));
    let mbw = mb.downgrade();
    window.add_action(&simple("main-menu", move || {
        if let Some(m) = mbw.upgrade() {
            m.popup();
        }
    }));
    accel(app, "win.main-menu", &["F10".into()]);
    header.pack_end(&mb);
    header.pack_end(&RouteButton::new("feed", false).icon("bell-outline-symbolic").tooltip(&tr("Feed")).button);
    header.upcast()
}

fn build_sidebar_layout(app: &adw::Application, window: &adw::ApplicationWindow) -> gtk::Widget {
    let tv = adw::ToolbarView::new();
    tv.add_top_bar(&build_sidebar_header(app, window));
    let stack = adw::ViewStack::new();
    stack.add_titled_with_icon(&player_view::new_player(), Some("player"), &tr("Player"), "music-note-outline-symbolic");
    stack.add_titled_with_icon(&lyrics::new_lyrics_panel(), Some("lyrics"), &tr("Lyrics"), "chat-bubble-text-symbolic");
    stack.add_titled_with_icon(&queue::new_queue(), Some("queue"), &tr("Queue"), "music-queue-symbolic");
    tv.set_content(Some(&stack));

    let switcher = adw::ViewSwitcher::new();
    switcher.set_policy(adw::ViewSwitcherPolicy::Wide);
    switcher.set_stack(Some(&stack));
    let center = gtk::CenterBox::new().hmargin(7).vmargin(6);
    center.set_center_widget(Some(&switcher));
    tv.add_bottom_bar(&center);
    tv.set_bottom_bar_style(adw::ToolbarStyle::Flat);
    tv.upcast()
}

// ------------------------------------------------------------------ actions

fn queue_param(param: String) {
    let parts: Vec<&str> = param.split('/').collect();
    if parts.len() != 2 {
        log::error!("parameter doesn't follow the format 'type/id' action=win.player.queue parameter={param}");
        return;
    }
    let id = parts[1].to_string();
    match parts[0] {
        "track" => spawn_logged("queue track", async move { player::add_track_to_user_queue(&id).await }),
        "album" => spawn_logged("queue album", async move {
            let t = api::run(move || service::get_album_tracks(&id).get_all()).await?;
            player::add_tracklist_to_user_queue(t);
            Ok(())
        }),
        "playlist" => spawn_logged("queue playlist", async move {
            let t = api::run(move || service::get_playlist_tracks(&id).get_all()).await?;
            player::add_tracklist_to_user_queue(t);
            Ok(())
        }),
        "artist" => spawn_logged("queue artist", async move {
            let t = api::run(move || {
                let page = api::artist_page(&id)?;
                let mut tracks = vec![];
                if let Some(m) = page.items.iter().find(|i| i.module_id == "ARTIST_TOP_TRACKS") {
                    for item in &m.items {
                        if let api::legacy::ItemData::Track(t) = &item.data {
                            tracks.push(model::track_from_v2(t));
                        }
                    }
                }
                Ok(tracks)
            })
            .await?;
            player::add_tracklist_to_user_queue(t);
            Ok(())
        }),
        "my_collection" => match parts[1] {
            "tracks" => spawn_logged("queue my tracks", async move {
                if secrets::user_id().is_empty() {
                    log::warn!("user not logged in, ignoring action");
                    return Ok(());
                }
                let t = api::run(|| service::user_collection_tracks("me").get_all()).await?;
                player::add_tracklist_to_user_queue(t);
                Ok(())
            }),
            _ => log::error!("unknown object type to add to queue type={}", parts[0]),
        },
        _ => log::error!("unknown object type to add to queue type={}", parts[0]),
    }
}

fn sign_in(window: &adw::ApplicationWindow) {
    let w = window.downgrade();
    glib::spawn_future_local(async move {
        let dlc = match api::run(api::request_device_linking_challenge).await {
            Ok(d) => d,
            Err(e) => {
                toast(&tr("Sign in failed or aborted"));
                log::error!("{e:#}");
                return;
            }
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        let c = Arc::clone(&cancelled);
        let win = w.upgrade();
        let dialog = linking::new_linking(win.clone().map(gtk::prelude::Cast::upcast), &dlc.user_code, &dlc.verification_uri_complete, Rc::new(move || c.store(true, Ordering::SeqCst)));
        dialog.present(win.as_ref());
        let c = Arc::clone(&cancelled);
        let r = api::run(move || api::poll_device_linking_status(&dlc.device_code, dlc.interval, &|| c.load(Ordering::SeqCst))).await;
        dialog.force_close();
        match r {
            Err(e) => {
                toast(&tr("Sign in failed or aborted"));
                log::error!("{e:#}");
            }
            Ok(resp) => {
                let token = resp.refresh_token.clone();
                let saved = api::run(move || secrets::set_refresh_token(&token)).await;
                if let Err(e) = saved {
                    log::error!("failed to store refresh token error={e:#}");
                }
                api::reset_user_token();
                state::notify_signed_in_changed();
                toast(&trf("Signed in as %s", &[&resp.user.email]));
                router::refresh();
            }
        }
    });
}

fn install_actions(app: &adw::Application, window: &adw::ApplicationWindow) {
    let w = window.downgrade();
    let wa = move || w.upgrade();

    let ww = wa.clone();
    app.add_action(&simple("about", move || {
        if let Some(w) = ww() {
            present_about(&w);
        }
    }));
    let ww = wa.clone();
    app.add_action(&simple("preferences", move || {
        if let Some(w) = ww() {
            present_preferences(&w);
        }
    }));
    accel(app, "app.preferences", &[p("comma"), p("p")]);
    let ww = wa.clone();
    app.add_action(&simple("shortcuts", move || {
        if let Some(w) = ww() {
            present_shortcuts(&w);
        }
    }));
    accel(app, "app.shortcuts", &[p("question")]);
    let a = app.downgrade();
    app.add_action(&simple("quit", move || {
        if let Some(a) = a.upgrade() {
            a.quit();
        }
    }));
    accel(app, "app.quit", &[p("q")]);

    let ww = wa.clone();
    window.add_action(&simple("close", move || {
        if let Some(w) = ww() {
            w.close();
        }
    }));
    accel(app, "win.close", &[p("w")]);
    window.add_action(&simple("navigate-back", router::back));
    accel(app, "win.navigate-back", &["<Alt>Left".into()]);
    window.add_action(&simple("search", || router::navigate("search")));
    accel(app, "win.search", &[p("f")]);

    window.add_action(&with_string("player.play-track", |id| spawn_logged("play track", async move { player::play_track_id(&id).await })));
    window.add_action(&with_string("player.play-playlist", |id| {
        spawn_logged("play playlist", async move { player::play_playlist(&id, false, 0).await });
    }));
    window.add_action(&simple("player.shuffle", player::toggle_shuffle));
    accel(app, "win.player.shuffle", &[p("s")]);
    window.add_action(&simple("player.next", player::next));
    accel(app, "win.player.next", &[p("Right")]);
    window.add_action(&simple("player.play-pause", player::play_pause));
    window.add_action(&simple("player.previous", player::previous));
    accel(app, "win.player.previous", &[p("Left")]);
    window.add_action(&simple("player.repeat", player::cycle_repeat_mode));
    accel(app, "win.player.repeat", &[p("r")]);
    window.add_action(&with_string("player.queue-track", |id| spawn_logged("queue track", async move { player::add_track_to_user_queue(&id).await })));
    window.add_action(&with_string("player.queue", queue_param));

    window.add_action(&simple("route.home", || router::navigate(&settings::default_page())));
    accel(app, "win.route.home", &[p("h")]);
    window.add_action(&with_string("route.album", |id| router::navigate(&format!("album/{id}"))));
    window.add_action(&with_string("route.playlist", |id| router::navigate(&format!("playlist/{id}"))));
    window.add_action(&with_string("route.artist", |id| router::navigate(&format!("artist/{id}"))));

    let ww = wa.clone();
    window.add_action(&simple("sign-in", move || {
        if let Some(w) = ww() {
            sign_in(&w);
        }
    }));
    window.add_action(&simple("sign-out", || {
        glib::spawn_future_local(async {
            if let Err(e) = api::run(secrets::delete_refresh_token).await {
                log::error!("failed to delete refresh token error={e:#}");
            }
            api::reset_user_token();
            state::notify_signed_in_changed();
            toast(&tr("Signed out"));
            router::refresh();
        });
    }));
    accel(app, "win.sign-out", &["<Primary>l".into()]);
    window.add_action(&simple("set-as-default", || {
        if let Some(c) = router::current() {
            settings::set_default_page(&c.path);
        }
        toast(&tr("Set current page as default"));
    }));
}

fn install_mouse_click_handler(window: &adw::ApplicationWindow) {
    let g = gtk::GestureClick::new();
    g.set_button(0);
    g.set_propagation_phase(gtk::PropagationPhase::Capture);
    let w = window.downgrade();
    g.connect_pressed(move |c, _, _, _| {
        if c.current_button() == 8
            && let Some(w) = w.upgrade()
        {
            let _ = WidgetExt::activate_action(&w, "win.navigate-back", None);
        }
    });
    window.add_controller(g);
}

// ------------------------------------------------------------------ dialogs

pub fn present_secret_service_error(window: &adw::ApplicationWindow, err: &secrets::ServiceError) {
    if settings::should_hide_secret_service_warning() {
        return;
    }
    let ad = adw::AlertDialog::new(Some(&err.title), Some(&err.body));
    ad.add_css_class("no-response");
    let check = gtk::CheckButton::with_label(&tr("Don't show again"));
    check.set_halign(gtk::Align::BaselineCenter);
    check.add_css_class("space-2");
    let cont = gtk::Button::with_label(&tr("Continue")).class("destructive-action").vpadding(10);
    let (c, a) = (check.clone(), ad.downgrade());
    cont.connect_clicked(move |_| {
        if c.is_active() {
            settings::set_hide_secret_service_warning(true);
        }
        if let Some(a) = a.upgrade() {
            a.close();
        }
    });
    ad.set_extra_child(Some(&spacing(vstack(&[check.upcast_ref(), cont.upcast_ref()]), 12)));
    ad.present(Some(window));
}

fn present_shortcuts(window: &adw::ApplicationWindow) {
    let d = adw::ShortcutsDialog::new();
    let basic = adw::ShortcutsSection::new(Some(&tr("Basic Shortcuts")));
    for (t, a) in [
        ("Close", "win.close"),
        ("Quit", "app.quit"),
        ("Side Pane", "win.toggle-sidebar"),
        ("Main Menu", "win.main-menu"),
        ("Keyboard Shortcuts", "app.shortcuts"),
        ("Preferences", "app.preferences"),
    ] {
        basic.add(adw::ShortcutsItem::from_action(&tr(t), a));
    }
    d.add(basic);
    let nav = adw::ShortcutsSection::new(Some(&tr("Navigation")));
    for (t, a) in [("Back", "win.navigate-back"), ("Search", "win.search")] {
        nav.add(adw::ShortcutsItem::from_action(&tr(t), a));
    }
    d.add(nav);
    d.present(Some(window));
}

fn present_about(window: &adw::ApplicationWindow) {
    let a = adw::AboutDialog::new();
    a.set_application_icon("logo");
    a.set_application_name("Tonearm");
    a.set_version(&version_number(VERSION, COMMIT));
    a.set_license_type(gtk::License::Gpl30);
    a.set_developers(&["Nila The Dragon https://github.com/NilaTheDragon", "Dråfølin https://github.com/Drafolin"]);
    a.set_translator_credits(&tr("translator-credits"));
    a.set_copyright("© 2026 Nila The Dragon");
    a.set_website("https://dergs.dev/projects/tonearm");
    a.set_issue_url("https://codeberg.org/dergs/Tonearm/issues");
    a.set_support_url("https://matrix.to/#/%23tonearm:derg.social");
    let mit = gtk::License::MitX11;
    for (name, copy, lic) in [
        ("GTK4 / Libadwaita Bindings (gtk-rs)", "© The gtk-rs Project Developers https://gtk-rs.org", mit),
        ("GStreamer Bindings (gstreamer-rs)", "© Sebastian Dröge https://gitlab.freedesktop.org/gstreamer/gstreamer-rs", mit),
        ("libsecret Bindings (libsecret-rs)", "© The gtk-rs Project Developers https://gitlab.gnome.org/World/Rust/libsecret-rs", mit),
        ("DBus Client (zbus)", "© Zeeshan Ali Khan https://github.com/dbus2/zbus", mit),
        ("HTTP Client (reqwest)", "© Sean McArthur https://github.com/seanmonstar/reqwest", mit),
        ("Serialization (serde, serde_json)", "© Erick Tryzelaar, David Tolnay https://serde.rs", mit),
        ("QR Code Generator (qrcode)", "© kennytm https://github.com/kennytm/qrcode-rust", mit),
        ("Regular Expressions (regex)", "© The Rust Project Developers https://github.com/rust-lang/regex", mit),
        ("Date and Time (chrono)", "© Kang Seonghoon and contributors https://github.com/chronotope/chrono", mit),
        ("Error Handling (anyhow)", "© David Tolnay https://github.com/dtolnay/anyhow", mit),
        ("MD5 (RustCrypto md-5)", "© RustCrypto Developers https://github.com/RustCrypto/hashes", mit),
        ("Logging (log, env_logger)", "© The Rust Project Developers https://github.com/rust-cli/env_logger", mit),
    ] {
        a.add_legal_section(name, Some(copy), lic, None);
    }
    a.present(Some(window));
}

fn switch_row(title: &str, subtitle: &str, s: gio::Settings, key: &str) -> adw::SwitchRow {
    let r = adw::SwitchRow::builder().title(title).subtitle(subtitle).build();
    settings::bind(&s, key, &r, "active");
    r
}

fn entry_row(title: &str, s: gio::Settings, key: &str) -> adw::EntryRow {
    let r = adw::EntryRow::builder().title(title).build();
    settings::bind(&s, key, &r, "text");
    r
}

fn group(title: &str, desc: &str, rows: &[&gtk::Widget]) -> adw::PreferencesGroup {
    let g = adw::PreferencesGroup::builder().title(title).description(desc).build();
    for r in rows {
        g.add(*r);
    }
    g
}

fn page(title: &str, icon: &str, groups: &[adw::PreferencesGroup]) -> adw::PreferencesPage {
    let p = adw::PreferencesPage::builder().title(title).icon_name(icon).build();
    for g in groups {
        p.add(g);
    }
    p
}

fn dialog_toast(d: &glib::WeakRef<adw::PreferencesDialog>, msg: &str) {
    if let Some(d) = d.upgrade() {
        let t = adw::Toast::new(msg);
        t.set_timeout(3);
        d.add_toast(t);
    }
}

fn present_preferences(window: &adw::ApplicationWindow) {
    let d = adw::PreferencesDialog::new();
    let (gen_s, perf, svc, pb, scr) = (settings::general(), settings::performance(), settings::service_tidal(), settings::playback(), settings::scrobbling());

    let history = adw::SpinRow::new(Some(&gtk::Adjustment::new(0.0, 1.0, 100.0, 1.0, 1.0, 1.0)), 1.0, 0);
    history.set_title(&tr("History Length"));
    history.set_subtitle(&tr("Maximum history length before dropping old entries"));
    settings::bind(&perf, "max-router-history-size", &history, "value");
    let base = adw::ExpanderRow::builder().title(tr("Base URLs")).subtitle(tr("Configure the base URLs for TIDAL")).build();
    base.add_row(&entry_row(&tr("API Base URL"), svc.clone(), "api-baseurl"));
    base.add_row(&entry_row(&tr("Auth Base URL"), svc.clone(), "auth-baseurl"));
    base.add_row(&entry_row(&tr("OpenAPI Base URL"), svc.clone(), "openapi-baseurl"));
    base.add_row(&entry_row(&tr("Resources Base URL"), svc, "resources-baseurl"));
    d.add(&page(
        &tr("General"),
        "settings-symbolic",
        &[
            group(
                &tr("Background Activity"),
                &tr("Configure the behaviour of Tonearm when running in the background"),
                &[switch_row(
                    &tr("Allow Background Activity"),
                    &tr("Allow Tonearm to run in the background by hiding the player window instead of quitting the application"),
                    gen_s.clone(),
                    "allow-running-in-background",
                )
                .upcast_ref()],
            ),
            group(
                &tr("Navigation Behaviour"),
                &tr("Configure the behaviour of Tonearm when navigating between pages"),
                &[entry_row(&tr("Default Page"), gen_s, "default-page").upcast_ref(), history.upcast_ref()],
            ),
            group(&tr("TIDAL Settings"), &tr("TIDAL-specific settings for advanced users"), &[base.upcast_ref()]),
        ],
    ));

    let rg = adw::ComboRow::builder()
        .title(tr("Preferred Replay Gain"))
        .subtitle(tr("Choose whether Tonearm should prefer album replay gain, track replay gain or decide automatically"))
        .build();
    let strs = settings::replay_gain_mode_strings();
    let strs: Vec<&str> = strs.iter().map(std::string::String::as_str).collect();
    rg.set_model(Some(&gtk::StringList::new(&strs)));
    rg.set_selected(settings::replay_gain_mode() as u32);
    rg.connect_selected_notify(|r| {
        let m = match r.selected() {
            1 => settings::ReplayGainMode::Album,
            2 => settings::ReplayGainMode::Track,
            _ => settings::ReplayGainMode::Auto,
        };
        settings::set_replay_gain_mode(m);
    });
    d.add(&page(
        &tr("Playback"),
        "media-playback-start-symbolic",
        &[group(
            &tr("Playback"),
            &tr("Configure the behaviour of Tonearm regarding playback"),
            &[
                switch_row(
                    &tr("Enable Autoplay"),
                    &tr("Allow Tonearm to start a mix with the current playing song when you're at the end of an album/queue"),
                    pb.clone(),
                    "allow-autoplay",
                )
                .upcast_ref(),
                switch_row(&tr("Normalize Volume"), &tr("Set the same volume for all tracks"), pb, "normalize-volume").upcast_ref(),
                rg.upcast_ref(),
            ],
        )],
    ));

    d.add(&page(
        &tr("Performance"),
        "speedometer5-symbolic",
        &[group(
            &tr("Images"),
            &tr("Configure the behaviour of Tonearm regarding images"),
            &[
                switch_row(&tr("Allow Media Card Images"), &tr("Allow Tonearm to load images for media card buttons"), perf.clone(), "allow-mediacard-images").upcast_ref(),
                switch_row(&tr("Allow Shortcut Images"), &tr("Allow Tonearm to load images for shortcut buttons"), perf.clone(), "allow-shortcuts-images").upcast_ref(),
                switch_row(&tr("Allow Tracklist Images"), &tr("Allow Tonearm to load images for tracklists with the cover column"), perf.clone(), "allow-tracklist-images")
                    .upcast_ref(),
                switch_row(
                    &tr("Cache Images"),
                    &tr("Allow Tonearm to retain recently loaded images in memory to improve performance and reduce network traffic (estimated 50 MiB limit)"),
                    perf,
                    "cache-images",
                )
                .upcast_ref(),
            ],
        )],
    ));

    // ---- scrobbling
    let lb_token = adw::PasswordEntryRow::builder().title(tr("ListenBrainz API Token")).build();
    settings::bind(&scr, "listenbrainz-token", &lb_token, "text");
    let lfm_row = adw::ActionRow::builder().title(tr("Log In to Last.fm…")).subtitle(tr("You are currently not logged in to Last.fm")).build();
    let lfm_button = gtk::Button::from_icon_name("key-login-symbolic").valign(gtk::Align::Center);
    lfm_row.add_suffix(&lfm_button);
    let logged_in = Rc::new(std::cell::Cell::new(!settings::lastfm_token().is_empty()));
    let refresh: Rc<dyn Fn()> = {
        let (row, btn, li) = (lfm_row.downgrade(), lfm_button.downgrade(), Rc::clone(&logged_in));
        Rc::new(move || {
            let (Some(row), Some(btn)) = (row.upgrade(), btn.upgrade()) else { return };
            if li.get() {
                row.set_title(&tr("Log Out of Last.fm"));
                btn.set_icon_name("system-log-out-symbolic");
                row.set_subtitle(&tr("You are logged in to Last.fm"));
                let r = row.downgrade();
                glib::spawn_future_local(async move {
                    let sk = settings::lastfm_token();
                    match api::run(move || crate::scrobbling::lastfm_self_name(&sk)).await {
                        Ok(name) => {
                            if let Some(r) = r.upgrade() {
                                r.set_subtitle(&trf("You are logged in to Last.fm as %s", &[&name]));
                            }
                        }
                        Err(e) => log::error!("error while fetching Last.fm user error={e:#}"),
                    }
                });
            } else {
                row.set_title(&tr("Log In to Last.fm…"));
                row.set_subtitle(&tr("You are currently not logged in to Last.fm"));
                btn.set_icon_name("key-login-symbolic");
            }
        })
    };
    refresh();
    let dw = d.downgrade();
    let win = window.downgrade();
    lfm_button.connect_clicked(move |_| {
        if logged_in.get() {
            crate::scrobbling::lastfm_unconfigure();
            logged_in.set(false);
            refresh();
            dialog_toast(&dw, &tr("Logged out of Last.fm"));
            return;
        }
        let (dw, li, refresh, win) = (dw.clone(), Rc::clone(&logged_in), Rc::clone(&refresh), win.clone());
        glib::spawn_future_local(async move {
            match lastfm_configure(win.upgrade()).await {
                Err(e) => {
                    log::error!("Last.fm login failed error={e:#}");
                    dialog_toast(&dw, &tr("An error occurred while logging in to Last.fm"));
                }
                Ok(None) => {}
                Ok(Some(name)) => {
                    li.set(true);
                    refresh();
                    dialog_toast(&dw, &trf("Logged in to Last.fm as %s", &[&name]));
                }
            }
        });
    });
    d.add(&page(
        &tr("Scrobbling"),
        "podcast-symbolic",
        &[
            group(
                "ListenBrainz",
                &tr("Configure Tonearm to send scrobbling data to ListenBrainz"),
                &[
                    switch_row(&tr("Enable ListenBrainz"), &tr("Allow Tonearm to send scrobbling data to ListenBrainz"), scr.clone(), "enable-listenbrainz").upcast_ref(),
                    lb_token.upcast_ref(),
                    entry_row(&tr("ListenBrainz API URL"), scr.clone(), "listenbrainz-url").upcast_ref(),
                ],
            ),
            group(
                "Last.fm",
                &tr("Configure Tonearm to send scrobbling data to Last.fm"),
                &[switch_row(&tr("Enable Last.fm"), &tr("Allow Tonearm to send scrobbling data to Last.fm"), scr, "enable-lastfm").upcast_ref(), lfm_row.upcast_ref()],
            ),
        ],
    ));
    d.present(Some(window));
}

/// Port of LastFm.Configure: token, browser auth, poll every 5s for a session.
/// Returns the user name, or `None` when the user cancelled.
async fn lastfm_configure(window: Option<adw::ApplicationWindow>) -> anyhow::Result<Option<String>> {
    let token = api::run(crate::scrobbling::lastfm_get_token).await?;
    let uri = crate::scrobbling::lastfm_auth_url(&token);
    let cancelled = Rc::new(std::cell::Cell::new(false));

    let ad = adw::AlertDialog::new(Some(&tr("Logging in to Last.fm")), Some(&tr("Continue the authentication process in your browser.")));
    ad.add_css_class("no-response");
    let c = Rc::clone(&cancelled);
    ad.connect_closed(move |_| c.set(true));
    let spinner = adw::Spinner::new().size(128, 128);
    let reopen = gtk::Button::with_label(&tr("Reopen Browser")).corner_radius(12).vpadding(10).hpadding(20).min_height(24).min_width(60);
    let (u, w) = (uri.clone(), window.clone());
    reopen.connect_clicked(move |_| gtk::UriLauncher::new(&u).launch(w.as_ref(), gio::Cancellable::NONE, |_| {}));
    let cancel = gtk::Button::with_label(&tr("Cancel")).corner_radius(12).vpadding(10).hpadding(20).min_height(24).min_width(60);
    let a = ad.downgrade();
    cancel.connect_clicked(move |_| {
        if let Some(a) = a.upgrade() {
            a.close();
        }
    });
    ad.set_extra_child(Some(&spacing(vstack(&[spinner.upcast_ref(), reopen.upcast_ref(), cancel.upcast_ref()]), 12)));
    ad.present(window.as_ref());
    gtk::UriLauncher::new(&uri).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});

    let result = loop {
        glib::timeout_future(std::time::Duration::from_secs(5)).await;
        if cancelled.get() {
            log::info!("Last.fm configuration cancelled by user");
            break Ok(None);
        }
        let t = token.clone();
        match api::run(move || crate::scrobbling::lastfm_get_session(&t)).await {
            Ok(None) => continue,
            Ok(Some(sk)) => {
                settings::set_lastfm_token(&sk);
                let name = api::run(move || crate::scrobbling::lastfm_self_name(&sk)).await;
                break name.map(|n| {
                    log::info!("Last.fm configured username={n}");
                    Some(n)
                });
            }
            Err(e) => break Err(e),
        }
    };
    ad.close();
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert_eq!(version_number(None, ""), "local");
        assert_eq!(version_number(None, "v1.2.0-0-gabcdef1"), "v1.2.0 (r0.abcdef1)");
        assert_eq!(version_number(None, "v0.3.0-5-gabcdef1"), "α 0.3.0 β(r5.abcdef1)");
        assert_eq!(version_number(None, "abcdef1234"), "α git+abcdef1");
        assert_eq!(version_number(Some("v1.0.0"), "abcdef1234"), "v1.0.0 (abcdef1)");
        assert!(is_stable(Some("1.0"), ""));
        assert!(is_stable(None, "v1.2.0-0-gabcdef1"));
        assert!(!is_stable(None, "v1.2.0-3-gabcdef1"));
        assert!(!is_stable(None, ""));
    }
}

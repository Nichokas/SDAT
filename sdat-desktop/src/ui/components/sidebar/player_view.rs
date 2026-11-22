//! Port of `components/sidebar/player`.
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::{gdk, gio, pango};

use crate::i18n::tr;
use crate::model::{format_duration, TrackQuality};
use crate::player::{self, PlaybackStatus, RepeatMode};
use crate::signals::State;
use crate::ui::components::{media_viewer, render_cover, squircle_picture, CancelToken};
use crate::ui::dsl::{on_state, W, spacing, vstack, label, hstack, on_state_lazy, spacer};
use crate::{global_signal, imgutil, router, service, settings};

global_signal!(fn is_controllable() -> State<bool> = State::new(false));
global_signal!(fn is_track_loaded() -> State<bool> = State::new(false));

pub fn init() {
    player::playback_state_changed().on(|ps| {
        is_controllable().set(!ps.loading);
        false
    });
    player::track_changed().on_lazy(|t| {
        is_track_loaded().set(t.is_some());
        false
    });
}

fn sensitive_if_loaded<W: IsA<gtk::Widget>>(w: &W) {
    on_state(is_track_loaded(), w.upcast_ref::<gtk::Widget>(), |w, v| w.set_sensitive(*v));
}

pub fn new_player() -> gtk::Box {
    spacing(
        vstack(&[
            playing_from().upcast_ref(),
            track_cover().upcast_ref(),
            track_info().upcast_ref(),
            action_row().upcast_ref(),
            track_timeline().upcast_ref(),
            controls().upcast_ref(),
        ]),
        25,
    )
    .class("player")
    .hpadding(24)
    .valign(gtk::Align::Center)
}

pub fn playing_from() -> gtk::Button {
    let title = label("Nothing").halign(gtk::Align::Start).class("heading");
    title.set_ellipsize(pango::EllipsizeMode::End);
    let img = gtk::Image::from_paintable(Some(&imgutil::missing_album()));
    img.set_pixel_size(33);
    let img = img
        .background("alpha(var(--view-fg-color), 0.1)")
        .halign(gtk::Align::End)
        .hexp(true)
        .valign(gtk::Align::Center)
        .overflow_hidden()
        .corner_radius(5);
    let texts = vstack(&[label(&tr("Playing From")).halign(gtk::Align::Start).class("caption-heading").class("dimmed").upcast_ref(), title.upcast_ref()])
        .valign(gtk::Align::Center)
        .mend(10);
    let b = gtk::Button::new().class("flat").css("button { margin-top: -10px; margin-bottom: -10px; margin-left: -10px; margin-right: -10px; padding-top: 10px; padding-bottom: 10px; }");
    b.set_child(Some(&hstack(&[texts.upcast_ref(), img.upcast_ref()]).halign(gtk::Align::Fill)));
    b.set_sensitive(false);
    let target = Rc::new(std::cell::RefCell::new(String::new()));
    let t2 = Rc::clone(&target);
    b.connect_clicked(move |_| router::navigate(&t2.borrow()));
    let (tw, iw) = (title.downgrade(), img.downgrade());
    let token = Rc::new(Cell::new(0u64));
    on_state_lazy(player::source_changed(), &b, move |b, s| {
        let (Some(title), Some(img)) = (tw.upgrade(), iw.upgrade()) else { return };
        let mine = token.get().wrapping_add(1);
        token.set(mine);
        if let Some(s) = s {
            title.set_text(&s.title());
            let route = s.route();
            b.set_sensitive(!route.is_empty());
            *target.borrow_mut() = route;
            if matches!(s, crate::model::PlaybackSource::MyTracks) {
                img.set_icon_name(Some("heart-filled-symbolic"));
                return;
            }
            img.set_paintable(Some(&imgutil::missing_album()));
            let cover = s.cover(80);
            if !cover.has_static() {
                return;
            }
            let (w, tok) = (img.downgrade(), Rc::clone(&token));
            glib::spawn_future_local(async move {
                match imgutil::fetch_cropped(&cover.static_).await {
                    Some(t) => {
                        if tok.get() == mine
                            && let Some(i) = w.upgrade()
                        {
                            i.set_paintable(Some(&t));
                        }
                    }
                    None => log::error!("failed to load source cover"),
                }
            });
        } else {
            title.set_text("Nothing");
            img.set_paintable(Some(&imgutil::missing_album()));
            b.set_sensitive(false);
            target.borrow_mut().clear();
        }
    });
    b
}

fn track_cover() -> gtk::Picture {
    let p = gtk::Picture::for_paintable(&imgutil::missing_album());
    p.set_hexpand(true);
    p.set_halign(gtk::Align::Center);
    let click = gtk::GestureClick::new();
    let pw = p.downgrade();
    click.connect_pressed(move |_, _, _, _| {
        if let Some(pt) = pw.upgrade().and_then(|p| p.paintable()) {
            media_viewer::get().show_file(&pt);
        }
    });
    p.add_controller(click);
    let token: Rc<std::cell::RefCell<Option<CancelToken>>> = Rc::default();
    on_state(player::track_changed(), &p, move |p, ev| {
        if let Some(t) = token.borrow_mut().take() {
            t.cancel();
        }
        let cover = ev.as_ref().map(|e| e.track.cover(i32::MAX)).unwrap_or_default();
        let w = p.downgrade();
        *token.borrow_mut() = Some(render_cover(cover, move |pt| {
            if let Some(p) = w.upgrade() {
                p.set_paintable(Some(&pt));
            }
        }));
    });
    squircle_picture(p)
}

fn track_info() -> gtk::Box {
    let title = label("").halign(gtk::Align::Center).class("title-4");
    title.set_ellipsize(pango::EllipsizeMode::End);
    let artist = label("").halign(gtk::Align::Center).class("heading").class("dimmed");
    artist.set_ellipsize(pango::EllipsizeMode::End);
    let aw = artist.downgrade();
    on_state(player::track_changed(), &title, move |t, ev| {
        let a = aw.upgrade();
        match ev {
            None => {
                t.set_text(&tr("No Track"));
                if let Some(a) = a {
                    a.set_text(&tr("No Artist"));
                }
            }
            Some(e) => {
                t.set_text(&e.track.formatted_title());
                if let Some(a) = a {
                    a.set_text(&e.track.artist_names().join(", "));
                }
            }
        }
    });
    spacing(vstack(&[title.upcast_ref(), artist.upcast_ref()]), 5)
}

fn volume_popover() -> gtk::Popover {
    let scale = gtk::Scale::with_range(gtk::Orientation::Vertical, 0.0, 1.0, 0.01);
    scale.set_inverted(true);
    let scale = scale.min_height(100).hexp(true).css("scale:active { background-color: transparent; }");
    scale.set_draw_value(false);
    on_state(player::volume_changed(), &scale, |s, v| s.set_value(v.powf(1.0 / 3.0)));
    scale.connect_change_value(|_, _, v| {
        player::set_volume(v.clamp(0.0, 1.0).powi(3));
        glib::Propagation::Proceed
    });
    let p = gtk::Popover::new();
    p.set_child(Some(&scale));
    p
}

fn current_track_id() -> Option<String> {
    player::track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string()))
}

fn favourite_button() -> gtk::Button {
    let (button, set_id) = crate::ui::components::favourite_button::track_favourite_button();
    on_state(player::track_changed(), &button, move |_, event| {
        set_id(event.as_ref().map(|event| event.track.id()));
    });
    button
}

fn action_row() -> gtk::Box {
    let volume = gtk::MenuButton::new().tooltip(&tr("Volume")).class("flat");
    volume.set_popover(Some(&volume_popover()));
    volume.set_icon_name("speakers-symbolic");
    let album = gtk::Button::from_icon_name("cd-symbolic").tooltip(&tr("Navigate to Album")).class("flat");
    sensitive_if_loaded(&album);
    album.connect_clicked(|_| {
        let id = player::track_changed().with(|t| t.as_ref().map(|e| e.track.album.as_ref().map(|a| a.id.clone()).unwrap_or_default()));
        if let Some(id) = id {
            router::navigate(&format!("album/{id}"));
        }
    });
    let artist_bin = adw::Bin::new();
    let single = gtk::Button::from_icon_name("music-artist2-symbolic").tooltip(&tr("Navigate to Artist")).class("flat");
    sensitive_if_loaded(&single);
    let multiple = gtk::MenuButton::new().tooltip(&tr("Navigate to Artist")).class("flat");
    multiple.set_icon_name("music-artist2-symbolic");
    sensitive_if_loaded(&multiple);
    artist_bin.set_child(Some(&single));
    on_state_lazy(player::track_changed(), &artist_bin, move |bin, ev| {
        let Some(ev) = ev else { return };
        let artists = &ev.track.artists;
        if artists.len() > 1 {
            let menu = gio::Menu::new();
            for a in artists {
                menu.append_item(&gio::MenuItem::new(Some(a.title()), Some(&format!("win.route.artist::{}", a.id))));
            }
            multiple.set_menu_model(Some(&menu));
            bin.set_child(Some(&multiple));
        } else if let Some(a) = artists.first() {
            single.set_action_name(Some("win.route.artist"));
            single.set_action_target_value(Some(&a.id.to_variant()));
            bin.set_child(Some(&single));
        }
    });
    let mix = gtk::Button::from_icon_name("compass2-symbolic").tooltip(&tr("Navigate to Track Mix")).class("flat");
    sensitive_if_loaded(&mix);
    mix.connect_clicked(|_| {
        let Some(id) = current_track_id() else { return };
        glib::spawn_future_local(async move {
            match crate::api::run(move || service::get_track_radio(&id)).await {
                Ok(Some(p)) => router::navigate(&format!("playlist/{}", p.id)),
                Ok(None) => player::toasts().notify(&tr("No mix found for the current track")),
                Err(e) => {
                    player::toasts().notify(&tr("An error occurred while fetching the track mix"));
                    log::error!("error while fetching track mix error={e:#}");
                }
            }
        });
    });
    let share = gtk::Button::from_icon_name("share-alt-symbolic").tooltip(&tr("Copy Track URL")).class("flat");
    sensitive_if_loaded(&share);
    share.connect_clicked(|_| {
        let url = player::track_changed().with(|t| t.as_ref().map(|e| e.track.url()));
        if let (Some(url), Some(d)) = (url, gdk::Display::default()) {
            d.clipboard().set_text(&format!("{url}?u"));
            player::toasts().notify(&tr("Copied track URL to clipboard"));
        }
    });
    spacing(
        hstack(&[volume.upcast_ref(), favourite_button().upcast_ref(), album.upcast_ref(), artist_bin.upcast_ref(), mix.upcast_ref(), share.upcast_ref(), super::handoff::send_to_button().upcast_ref()]),
        15,
    )
    .halign(gtk::Align::Center)
    .css("box { margin-top: -8px; margin-bottom: -8px; }")
}

fn truncate_float(f: f64, precision: usize) -> String {
    let mut s = format!("{f:.precision$}");
    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }
    s
}

fn quality_label(q: TrackQuality) -> (String, &'static str) {
    match q {
        TrackQuality::Lossy => (tr("Low (96 kbps)"), "low"),
        TrackQuality::High => (tr("Low (320 kbps)"), "low"),
        TrackQuality::Lossless => (tr("High"), "high"),
        TrackQuality::HighResLossless => (tr("Max"), "max"),
    }
}

fn quality_entry(q: TrackQuality, css: &str, title: &str, details: &str, popover: &gtk::Popover) -> gtk::Button {
    // Go draws a "✓" text label; a small symbolic icon (tinted with the entry's text
    // colour) sits better. Its space is always reserved so labels never shift.
    let check = gtk::Image::from_icon_name("object-select-symbolic");
    check.set_pixel_size(14);
    check.set_valign(gtk::Align::Center);
    check.set_margin_end(2);
    let show = move |c: &gtk::Image| c.set_opacity(if settings::audio_quality() == q { 1.0 } else { 0.0 });
    show(&check);
    let cw = check.downgrade();
    let id = settings::connect_changed(&settings::player(), "audio-quality", move |_| {
        if let Some(c) = cw.upgrade() {
            show(&c);
        }
    });
    let _ = id;
    let texts = vstack(&[label(title).class("heading").halign(gtk::Align::Start).upcast_ref(), label(details).class("caption").halign(gtk::Align::Start).upcast_ref()]);
    let b = gtk::Button::new().class(css);
    b.set_child(Some(&spacing(hstack(&[texts.upcast_ref(), spacer().upcast_ref(), check.upcast_ref()]), 10)));
    let pw = popover.downgrade();
    b.connect_clicked(move |_| {
        settings::set_audio_quality(q);
        if let Some(p) = pw.upgrade() {
            p.popdown();
        }
    });
    b
}

/// Guest scrub: quiet time before the last dragged position is sent.
const SCRUB_DEBOUNCE: Duration = Duration::from_millis(250);

/// Position, (session, host track) it is for, and the pending timer.
type PendingScrub = (Duration, Option<(u64, String)>, glib::SourceId);

thread_local! {
    /// Guest scrub waiting to be sent: position, the session and host track it is for,
    /// and its pending timer.
    static SCRUB: std::cell::RefCell<Option<PendingScrub>> = const { std::cell::RefCell::new(None) };
}

/// Extrapolates playback position between 250 ms GStreamer polls for vsync-smooth scrubbing.
#[derive(Debug, Clone, Copy, Default)]
struct TimelineClock {
    position: Duration,
    duration: Duration,
    at: Option<Instant>,
}

impl TimelineClock {
    // Bounds extrapolation if the 250 ms playback poll pauses or lags.
    const MAX_EXTRAPOLATION: Duration = Duration::from_millis(500);

    fn sync(&mut self, position: Duration, duration: Duration, advancing: bool, now: Instant) {
        self.position = position;
        self.duration = duration;
        self.at = advancing.then_some(now);
    }

    fn position_at(&self, now: Instant) -> Duration {
        let Some(at) = self.at else {
            return self.position;
        };
        let elapsed = now.saturating_duration_since(at).min(Self::MAX_EXTRAPOLATION);
        self.position.saturating_add(elapsed).min(self.duration.max(self.position))
    }

    fn percent_at(&self, now: Instant) -> f64 {
        if self.duration.is_zero() {
            return 0.0;
        }
        (100.0 * self.position_at(now).as_secs_f64() / self.duration.as_secs_f64()).clamp(0.0, 100.0)
    }
}

fn track_timeline() -> gtk::Overlay {
    let scale = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0.0, 100.0, 1.0);
    scale.set_increments(1.0, 1.0);
    scale.set_draw_value(false);
    let scale = scale.hexp(true).background("transparent").hpadding(0);
    on_state(is_controllable(), &scale, |s, v| s.set_sensitive(*v));
    scale.connect_change_value(|_, _, f| {
        // Guests with control ask the host to seek everyone; otherwise
        // refused. A drag emits many values: send only the last one, once
        // the scrubber has been still for a moment (the command budget is
        // small and every host seek flushes its pipeline).
        if crate::live::is_guest() {
            let d = player::playback_state_changed().get().duration;
            SCRUB.with(|s| {
                if let Some((_, _, id)) = s.borrow_mut().take() {
                    id.remove();
                }
                let id = glib::timeout_add_local_once(SCRUB_DEBOUNCE, || {
                    // Dropped if the session or the host's track changed meanwhile.
                    if let Some((to, track, _)) = SCRUB.with(|s| s.borrow_mut().take())
                        && track.is_some()
                        && crate::live::host_track() == track
                    {
                        crate::live::user_seek(to);
                    }
                });
                *s.borrow_mut() = Some((d.mul_f64((f / 100.0).clamp(0.0, 1.0)), crate::live::host_track(), id));
            });
            return glib::Propagation::Stop;
        }
        player::seek_to_percent(f);
        glib::Propagation::Proceed
    });
    let position = label("00:00");
    let duration = label("00:00");
    let dw = duration.downgrade();
    on_state(player::track_changed(), &duration, |d, ev| {
        d.set_text(&ev.as_ref().map_or_else(|| "00:00".into(), |e| format_duration(e.track.duration())));
    });
    let sw = scale.downgrade();
    let clock = Rc::new(Cell::new(TimelineClock::default()));
    let tick = Rc::new(Cell::new(None::<gtk::TickCallbackId>));
    let shown_secs = Rc::new(Cell::new(None));
    on_state(player::playback_state_changed(), &position, move |pos_label, ps| {
        let now = Instant::now();
        let advancing = ps.status == PlaybackStatus::Playing && !ps.loading && !ps.buffering && !ps.duration.is_zero();
        let mut state = clock.get();
        state.sync(ps.position, ps.duration, advancing, now);
        clock.set(state);
        let secs = ps.position.as_secs();
        if shown_secs.get() != Some(secs) {
            pos_label.set_text(&format_duration(ps.position));
            shown_secs.set(Some(secs));
        }
        if !ps.duration.is_zero()
            && let Some(dur_label) = dw.upgrade()
        {
            dur_label.set_text(&format_duration(ps.duration));
        }
        let Some(scale) = sw.upgrade() else {
            if let Some(id) = tick.take() {
                id.remove();
            }
            return;
        };
        // A stop (zero duration) resets even mid-drag: no later update would move the thumb back.
        if ps.duration.is_zero() || !scale.has_css_class("dragging") {
            scale.set_value(state.percent_at(now));
        }
        if advancing {
            let id = tick.take().unwrap_or_else(|| {
                let clock = Rc::clone(&clock);
                let tick = Rc::clone(&tick);
                let shown_secs = Rc::clone(&shown_secs);
                let pos_weak = pos_label.downgrade();
                scale.add_tick_callback(move |scale, _| {
                    let state = clock.get();
                    if state.at.is_none() {
                        tick.set(None);
                        return glib::ControlFlow::Break;
                    }
                    let now = Instant::now();
                    if !scale.has_css_class("dragging") {
                        scale.set_value(state.percent_at(now));
                        let pos = state.position_at(now);
                        let secs = pos.as_secs();
                        if shown_secs.get() != Some(secs)
                            && let Some(pos_label) = pos_weak.upgrade()
                        {
                            pos_label.set_text(&format_duration(pos));
                            shown_secs.set(Some(secs));
                        }
                    }
                    // Polls stopped: the clock is capped, so stop ticking until the next sync restarts it.
                    if state.at.is_some_and(|at| now.saturating_duration_since(at) >= TimelineClock::MAX_EXTRAPOLATION) {
                        tick.set(None);
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                })
            });
            tick.set(Some(id));
        } else if let Some(id) = tick.take() {
            id.remove();
        }
    });
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&vstack(&[scale.upcast_ref(), hstack(&[position.upcast_ref(), spacer().vexp(false).upcast_ref(), duration.upcast_ref()]).upcast_ref()]).mbottom(2)));

    // Quality selector.
    let popover = gtk::Popover::new();
    let entries = spacing(
        vstack(&[
            quality_entry(TrackQuality::Lossy, "low", "Low (96 kbps)", "96 kbps AAC", &popover).upcast_ref(),
            quality_entry(TrackQuality::High, "low", "Low (320 kbps)", "320 kbps AAC", &popover).upcast_ref(),
            quality_entry(TrackQuality::Lossless, "high", "High", "16-bit 44.1kHz FLAC", &popover).upcast_ref(),
            quality_entry(TrackQuality::HighResLossless, "max", "Max", "24-bit 192kHz FLAC", &popover).upcast_ref(),
        ]),
        8,
    )
    .class("selector");
    popover.set_child(Some(&entries));

    let text = label("").class("caption-heading").corner_radius(10).hpadding(8).vpadding(4);
    let selected = Rc::new(std::cell::RefCell::new(quality_label(settings::audio_quality())));
    let actual = Rc::new(std::cell::RefCell::new("N/A".to_string()));
    let details = Rc::new(Cell::new(false));
    let refresh: Rc<dyn Fn()> = {
        let (tw, selected, actual, details) = (text.downgrade(), Rc::clone(&selected), Rc::clone(&actual), Rc::clone(&details));
        Rc::new(move || {
            let Some(t) = tw.upgrade() else { return };
            let (sel, class) = selected.borrow().clone();
            if details.get() { t.set_text(&actual.borrow()) } else { t.set_text(&sel) }
            for c in ["low", "high", "max"] {
                t.remove_css_class(c);
            }
            t.add_css_class(class);
        })
    };
    refresh();
    {
        let (selected, refresh) = (Rc::clone(&selected), Rc::clone(&refresh));
        on_state(player::playback_quality_changed(), &text, move |_, q| {
            *selected.borrow_mut() = quality_label(*q);
            refresh();
        });
    };
    {
        let tw = text.downgrade();
        settings::connect_changed(&settings::player(), "audio-quality", move |_| {
            if tw.upgrade().is_some() {
                player::toasts().notify(&tr("Playback quality saved. Changes will be applied on next track change"));
            }
        })
    };
    {
        let (actual, refresh) = (Rc::clone(&actual), Rc::clone(&refresh));
        on_state(player::audio_stream_quality(), &text, move |_, sq| {
            *actual.borrow_mut() = match sq.codec.0.as_str() {
                player::quality::CODEC_AAC => format!("{} kbps AAC", truncate_float(f64::from(sq.bit_rate) / 1000.0, 1)),
                player::quality::CODEC_FLAC => format!("{}-bit {}kHz FLAC", sq.bit_depth, truncate_float(f64::from(sq.sample_rate) / 1000.0, 1)),
                _ => "N/A".into(),
            };
            refresh();
        });
    };
    let toggle: Rc<dyn Fn()> = {
        let (details, refresh) = (Rc::clone(&details), Rc::clone(&refresh));
        Rc::new(move || {
            details.set(!details.get());
            refresh();
        })
    };
    let mb = gtk::MenuButton::new().class("quality-selector").hexp(false).valign(gtk::Align::End).halign(gtk::Align::Center);
    mb.set_child(Some(&text));
    mb.set_popover(Some(&popover));
    let click = gtk::GestureClick::new();
    click.set_button(3);
    let t1 = Rc::clone(&toggle);
    click.connect_pressed(move |_, _, _, _| t1());
    mb.add_controller(click);
    let long = gtk::GestureLongPress::new();
    let pressed = Rc::new(Cell::new(false));
    let (t2, p2) = (Rc::clone(&toggle), Rc::clone(&pressed));
    long.connect_pressed(move |_, _, _| {
        p2.set(true);
        t2();
    });
    let mbw = mb.downgrade();
    long.connect_end(move |_, _| {
        if pressed.get()
            && let Some(m) = mbw.upgrade()
        {
            m.set_active(false);
        }
        pressed.set(false);
    });
    mb.add_controller(long);
    let key = gtk::EventControllerKey::new();
    key.set_propagation_phase(gtk::PropagationPhase::Capture);
    key.connect_key_pressed(move |_, k, _, _| {
        if k == gdk::Key::space || k == gdk::Key::KP_Space {
            toggle();
            return glib::Propagation::Stop;
        }
        glib::Propagation::Proceed
    });
    mb.add_controller(key);
    overlay.add_overlay(&mb);
    overlay
}

fn control_button(icon: &str) -> gtk::Button {
    gtk::Button::from_icon_name(icon).min_height(34).min_width(34).class("flat").valign(gtk::Align::Center)
}

fn controls() -> gtk::Box {
    let shuffle = control_button("playlist-shuffle-symbolic");
    shuffle.set_action_name(Some("win.player.shuffle"));
    sensitive_if_loaded(&shuffle);
    on_state(player::shuffle_state_changed(), &shuffle, |b, on| {
        if *on {
            b.add_css_class("accent");
            b.set_tooltip_text(Some(&tr("Disable Shuffle")));
        } else {
            b.remove_css_class("accent");
            b.set_tooltip_text(Some(&tr("Enable Shuffle")));
        }
    });
    let prev = control_button("skip-backward-large-symbolic").tooltip(&tr("Previous"));
    sensitive_if_loaded(&prev);
    prev.set_action_name(Some("win.player.previous"));
    let next = control_button("skip-forward-large-symbolic").tooltip(&tr("Next"));
    sensitive_if_loaded(&next);
    next.set_action_name(Some("win.player.next"));
    let repeat = control_button("playlist-repeat-symbolic");
    sensitive_if_loaded(&repeat);
    repeat.set_action_name(Some("win.player.repeat"));
    on_state(player::repeat_mode_changed(), &repeat, |b, m| {
        let (class, icon, tip) = match m {
            RepeatMode::None => (false, "playlist-repeat-symbolic", tr("Repeat Queue")),
            RepeatMode::Queue => (true, "playlist-repeat-symbolic", tr("Repeat Track")),
            RepeatMode::Track => (true, "playlist-repeat-song-symbolic", tr("Disable Repeat")),
        };
        if class {
            b.add_css_class("accent");
        } else {
            b.remove_css_class("accent");
        }
        b.set_icon_name(icon);
        b.set_tooltip_text(Some(&tip));
    });
    let pp = gtk::Button::new().class("suggested-action").corner_radius(21).hpadding(32).vpadding(9);
    pp.set_action_name(Some("win.player.play-pause"));
    let pause_icon = gtk::Image::from_icon_name("pause-symbolic");
    let play_icon = gtk::Image::from_icon_name("play-symbolic");
    let spinner = adw::Spinner::new();
    spinner.set_size_request(16, 16);
    pp.set_child(Some(&play_icon));
    let update = move |b: &gtk::Button, ps: &player::PlaybackState| {
        let icon = if ps.status == PlaybackStatus::Playing {
            b.set_tooltip_text(Some(&tr("Pause")));
            pause_icon.clone().upcast::<gtk::Widget>()
        } else {
            b.set_tooltip_text(Some(&tr("Play")));
            play_icon.clone().upcast()
        };
        if ps.loading {
            b.set_sensitive(false);
            b.set_child(Some(&spinner));
        } else {
            b.set_sensitive(is_track_loaded().get());
            b.set_child(Some(&icon));
        }
    };
    on_state(player::playback_state_changed(), &pp, update);
    spacer_box(&[shuffle.upcast_ref(), prev.upcast_ref(), pp.upcast_ref(), next.upcast_ref(), repeat.upcast_ref()])
}

fn spacer_box(items: &[&gtk::Widget]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 0).hexp(true);
    for (i, w) in items.iter().enumerate() {
        if i > 0 {
            b.append(&spacer().vexp(false));
        }
        b.append(*w);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paused_returns_exact_position() {
        let t0 = Instant::now();
        let mut clock = TimelineClock::default();
        clock.sync(Duration::from_secs(30), Duration::from_secs(120), false, t0);
        assert_eq!(clock.position_at(t0 + Duration::from_secs(5)), Duration::from_secs(30));
        assert!((clock.percent_at(t0 + Duration::from_secs(5)) - 25.0).abs() < 1e-9);
    }

    #[test]
    fn playing_extrapolates_smoothly() {
        let t0 = Instant::now();
        let mut clock = TimelineClock::default();
        clock.sync(Duration::from_secs(10), Duration::from_secs(100), true, t0);
        let at = t0 + Duration::from_millis(125);
        assert_eq!(clock.position_at(at), Duration::from_millis(10_125));
        assert!((clock.percent_at(at) - 10.125).abs() < 1e-9);
    }

    #[test]
    fn extrapolation_clamps_at_max() {
        let t0 = Instant::now();
        let mut clock = TimelineClock::default();
        clock.sync(Duration::from_secs(10), Duration::from_secs(100), true, t0);
        let at = t0 + Duration::from_secs(5);
        assert_eq!(clock.position_at(at), Duration::from_secs(10) + TimelineClock::MAX_EXTRAPOLATION);
    }

    #[test]
    fn extrapolation_clamps_at_duration() {
        let t0 = Instant::now();
        let mut clock = TimelineClock::default();
        clock.sync(Duration::from_millis(99_800), Duration::from_secs(100), true, t0);
        let at = t0 + Duration::from_millis(400);
        assert_eq!(clock.position_at(at), Duration::from_secs(100));
        assert!((clock.percent_at(at) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn zero_duration_yields_zero_percent() {
        let t0 = Instant::now();
        let mut clock = TimelineClock::default();
        clock.sync(Duration::from_secs(5), Duration::ZERO, false, t0);
        assert!(clock.percent_at(t0).abs() < f64::EPSILON);
    }
}

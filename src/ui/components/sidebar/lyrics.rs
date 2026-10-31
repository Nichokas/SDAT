//! Port of `components/sidebar/lyrics`: synced (LRC) lyrics with plain fallback.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::LazyLock;
use std::time::Duration;

use adw::prelude::*;

use crate::i18n::tr;
use crate::player::{self, PlaybackStatus};
use crate::signals::{Subscription, State};
use crate::ui::dsl::{W, on_state, label, spacing, vstack};
use crate::{api, global_signal};

static TIMING_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\[(\d{2}:\d{2}\.\d{2})\](.*)").expect("TIMING_RE is a valid regex"));
static TS_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"(\d{2}):(\d{2})\.(\d{2})").expect("TS_RE is a valid regex"));

/// A parsed lyric line: `None` timing means plain (untimed) text, `text == None` is a blank spacer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Blank,
    Text(String, Option<(Duration, Duration)>),
    /// A timed entry without text (keeps highlighting state but renders nothing).
    Silent(Duration, Duration),
}

pub fn parse_timestamp(ts: &str) -> Option<Duration> {
    let c = TS_RE.captures(ts)?;
    let n = |i: usize| c[i].parse::<u64>().unwrap_or(0);
    Some(Duration::from_secs(n(1) * 60 + n(2)) + Duration::from_millis(n(3) * 10))
}

pub fn parse_lrc(lyrics: &str, track_duration: Duration) -> Vec<Line> {
    let split: Vec<&str> = lyrics.split('\n').collect();
    let mut out = Vec::new();
    for (i, line) in split.iter().enumerate() {
        if line.trim().is_empty() {
            out.push(Line::Blank);
            continue;
        }
        let Some(m) = TIMING_RE.captures(line) else { continue };
        let start = parse_timestamp(&m[1]).unwrap_or_default();
        let mut end = track_duration;
        if i + 1 < split.len() {
            let mut off = 1;
            let mut next = TIMING_RE.captures(split[i + off]);
            while next.is_none() && i + off + 1 < split.len() {
                off += 1;
                next = TIMING_RE.captures(split[i + off]);
            }
            if let Some(n) = next {
                end = parse_timestamp(&n[1]).unwrap_or_default();
            }
        }
        if m[2].is_empty() {
            out.push(Line::Silent(start, end));
            continue;
        }
        out.push(Line::Text(m[2].trim().to_string(), Some((start, end))));
    }
    out
}

pub fn parse_untimed(lyrics: &str) -> Vec<Line> {
    lyrics
        .split('\n')
        .map(|l| if l.trim().is_empty() { Line::Blank } else { Line::Text(l.to_string(), None) })
        .collect()
}

fn get_lyrics(id: &str) -> anyhow::Result<(String, bool)> {
    let t = api::client::track(id, &["lyrics"])?;
    if let Some(l) = t.included.plain_lyrics(&t.data.relationships.lyrics.data).first() {
        if !l.attributes.lrc_text.is_empty() {
            return Ok((l.attributes.lrc_text.clone(), true));
        } else if !l.attributes.text.is_empty() {
            return Ok((l.attributes.text.clone(), false));
        }
    }
    Ok((String::new(), false))
}

struct Timing {
    start: Duration,
    end: Duration,
    button: Option<glib::WeakRef<gtk::Button>>,
}

global_signal!(fn manually_scrolled() -> State<bool> = State::new(false));

struct View {
    scrolled: gtk::ScrolledWindow,
    overlay: gtk::Overlay,
    panel: adw::Bin,
    active: RefCell<Option<glib::WeakRef<gtk::Button>>>,
    lines: RefCell<Vec<gtk::Button>>,
    sub: Cell<Option<Subscription>>,
    generation: Cell<u64>,
}

thread_local! {
    static VIEW: &'static View = Box::leak(Box::new(View::build()));
}

fn view() -> &'static View {
    VIEW.with(|v| *v)
}

fn status(title: &str, icon: Option<&str>, loading: bool) -> gtk::Widget {
    let sp = adw::StatusPage::builder().title(title).vexpand(true).build();
    if let Some(i) = icon {
        sp.set_icon_name(Some(i));
    }
    if loading {
        sp.set_paintable(Some(&adw::SpinnerPaintable::new(Some(&sp))));
    }
    sp.upcast()
}

impl View {
    fn build() -> Self {
        let scrolled = gtk::ScrolledWindow::new().hpadding(14);
        scrolled.set_policy(gtk::PolicyType::Never, gtk::PolicyType::External);
        let sc = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        sc.connect_scroll(|_, _, _| {
            manually_scrolled().set(true);
            glib::Propagation::Proceed
        });
        scrolled.add_controller(sc);
        let kc = gtk::EventControllerKey::new();
        kc.connect_key_pressed(|_, _, code, _| {
            if [23u32, 110, 111, 112, 115, 116, 117].contains(&code) {
                manually_scrolled().set(true);
            }
            glib::Propagation::Proceed
        });
        scrolled.add_controller(kc);
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(&scrolled));
        let sync = gtk::Button::new().halign(gtk::Align::End).valign(gtk::Align::End).margin(7).tooltip(&tr("Sync with Track"));
        sync.set_child(Some(&gtk::Image::from_icon_name("arrow-circular-top-right-symbolic")));
        on_state(manually_scrolled(), &sync, |b, v| b.set_visible(*v));
        sync.connect_clicked(|_| {
            let v = view();
            let a = v.active.borrow().as_ref().and_then(glib::WeakRef::upgrade);
            scroll_to(a.as_ref());
            manually_scrolled().set(false);
        });
        overlay.add_overlay(&sync);
        let panel = adw::Bin::new();
        panel.set_child(Some(&status(&tr("No Song Currently Playing"), Some("no-track-symbolic"), false)));
        View {
            scrolled,
            overlay,
            panel,
            active: RefCell::new(None),
            lines: RefCell::new(Vec::new()),
            sub: Cell::new(None),
            generation: Cell::new(0),
        }
    }
}

fn scroll_to(w: Option<&gtk::Button>) {
    let v = view();
    let adj = v.scrolled.vadjustment();
    let Some(w) = w else {
        adj.set_value(0.0);
        return;
    };
    let Some(parent) = w.parent() else { return };
    let Some(b) = w.compute_bounds(&parent) else { return };
    let center = f64::from(b.y() + b.height() / 2.0);
    let target = (center - f64::from(v.scrolled.height() / 2)).clamp(adj.lower(), (adj.upper() - adj.page_size()).max(adj.lower()));
    adj.set_value(target);
}

fn set_active(b: Option<&gtk::Button>) {
    let v = view();
    let cur = v.active.borrow().as_ref().and_then(glib::WeakRef::upgrade);
    if cur.as_ref() == b {
        return;
    }
    for l in v.lines.borrow().iter() {
        if Some(l) == b {
            l.remove_css_class("dimmed");
        } else if !l.has_css_class("dimmed") && l.has_css_class("timed") {
            l.add_css_class("dimmed");
        }
    }
    *v.active.borrow_mut() = b.map(gtk::prelude::ObjectExt::downgrade);
    if b.is_some() && !manually_scrolled().get() {
        let b = b.cloned();
        glib::idle_add_local_once(move || scroll_to(b.as_ref()));
    }
}

fn line_button(text: &str, timing: Option<(Duration, Duration)>) -> gtk::Button {
    let l = label(text).halign(gtk::Align::Center).valign(gtk::Align::Center).class("title-2");
    l.set_wrap(true);
    l.set_justify(gtk::Justification::Center);
    let b = gtk::Button::new().hexp(true).ptop(24).pbottom(24).pstart(16).pend(16).corner_radius(12);
    b.set_child(Some(&l));
    if let Some((start, _)) = timing {
        // "timed" is an internal marker only (no style rules use it).
        b.add_css_class("dimmed");
        b.add_css_class("timed");
        b.connect_clicked(move |b| {
            set_active(Some(b));
            manually_scrolled().set(false);
            player::seek_to_position(start, true);
        });
    }
    b
}

fn set_lines(lines: Vec<Line>, timed: bool) {
    let v = view();
    v.scrolled.set_policy(gtk::PolicyType::Never, if timed { gtk::PolicyType::External } else { gtk::PolicyType::Automatic });
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 12).hexp(true).vexp(true);
    let mut timings = Vec::new();
    let mut buttons = Vec::new();
    for l in lines {
        match l {
            Line::Blank => vbox.append(&gtk::Box::new(gtk::Orientation::Vertical, 0)),
            Line::Silent(start, end) => timings.push(Timing { start, end, button: None }),
            Line::Text(t, tm) => {
                let b = line_button(&t, tm);
                vbox.append(&b);
                if let Some((start, end)) = tm {
                    timings.push(Timing { start, end, button: Some(b.downgrade()) });
                }
                buttons.push(b);
            }
        }
    }
    *v.lines.borrow_mut() = buttons;
    v.scrolled.set_child(Some(&vbox));
    if !timed {
        return;
    }
    let timings = Rc::new(timings);
    let generation = v.generation.get();
    let sub = player::playback_state_changed().on(move |ps| {
        if view().generation.get() != generation {
            return true;
        }
        if ps.status != PlaybackStatus::Playing {
            return false;
        }
        let mut has_active = false;
        for t in timings.iter() {
            if ps.position > t.end {
                continue;
            }
            let Some(b) = t.button.as_ref() else { continue };
            if t.start <= ps.position {
                if let Some(b) = b.upgrade() {
                    set_active(Some(&b));
                }
                has_active = true;
                continue;
            }
            if t.start <= ps.position + Duration::from_millis(250) {
                let (b, wait) = (b.clone(), t.start.saturating_sub(ps.position));
                glib::timeout_add_local_once(wait, move || {
                    if view().generation.get() == generation
                        && let Some(b) = b.upgrade()
                    {
                        set_active(Some(&b));
                    }
                });
            }
        }
        if !has_active && view().active.borrow().is_some() {
            set_active(None);
        }
        false
    });
    v.sub.set(Some(sub));
}

pub fn init() {
    player::track_changed().on_lazy(|ev| {
        let v = view();
        v.generation.set(v.generation.get() + 1);
        if let Some(s) = v.sub.take() {
            player::playback_state_changed().unsubscribe(s);
        }
        v.scrolled.vadjustment().set_value(0.0);
        manually_scrolled().set(false);
        v.panel.set_child(Some(&status(&tr("Loading Lyrics"), None, true)));
        v.scrolled.set_child(gtk::Widget::NONE);
        v.lines.borrow_mut().clear();
        *v.active.borrow_mut() = None;
        let Some(ev) = ev.clone() else {
            v.panel.set_child(Some(&status(&tr("No Song Currently Playing"), Some("no-track-symbolic"), false)));
            return false;
        };
        let generation = v.generation.get();
        glib::spawn_future_local(async move {
            let id = ev.track.id().to_string();
            let r = api::run(move || get_lyrics(&id)).await;
            let v = view();
            if v.generation.get() != generation {
                return;
            }
            match r {
                Err(e) => {
                    log::error!("Error while fetching lyrics error={e:#}");
                    v.panel.set_child(Some(&status(&tr("Error Fetching Lyrics"), Some("dialog-error-symbolic"), false)));
                }
                Ok((text, _)) if text.is_empty() => {
                    v.panel.set_child(Some(&status(&tr("No Lyrics Available"), Some("no-lyrics-symbolic"), false)));
                }
                Ok((text, timed)) => {
                    let lines = if timed { parse_lrc(&text, ev.track.duration()) } else { parse_untimed(&text) };
                    set_lines(lines, timed);
                    v.panel.set_child(Some(&v.overlay));
                }
            }
        });
        false
    });
}

pub fn new_lyrics_panel() -> gtk::Box {
    let mini = super::mini_player();
    on_state(player::track_changed(), &mini, |m, t| m.set_visible(t.is_some()));
    spacing(vstack(&[mini.upcast_ref(), view().panel.upcast_ref()]), 7).class("lyrics-panel")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps() {
        assert_eq!(parse_timestamp("01:02.50"), Some(Duration::from_millis(62_500)));
        assert_eq!(parse_timestamp("bad"), None);
    }

    #[test]
    fn lrc() {
        let l = parse_lrc("[00:01.00] Hello\n\n[00:03.00]\n[00:05.00] World", Duration::from_secs(10));
        assert_eq!(l[0], Line::Text("Hello".into(), Some((Duration::from_secs(1), Duration::from_secs(3)))));
        assert_eq!(l[1], Line::Blank);
        assert_eq!(l[2], Line::Silent(Duration::from_secs(3), Duration::from_secs(5)));
        assert_eq!(l[3], Line::Text("World".into(), Some((Duration::from_secs(5), Duration::from_secs(10)))));
    }

    #[test]
    fn untimed() {
        assert_eq!(parse_untimed("a\n \nb"), vec![Line::Text("a".into(), None), Line::Blank, Line::Text("b".into(), None)]);
    }
}

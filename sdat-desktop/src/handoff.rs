//! Continue playback on another device: glue between the core's
//! `tonearm_core::handoff` (TIDAL's per-account playback state) and the
//! `GStreamer` player.

use std::cell::{Cell, RefCell};
use std::sync::Arc;
use std::time::Duration;

use gtk::gio;
use tonearm_core::handoff::{Device, Elsewhere, Handoff, HandoffEvent, HandoffListener, LocalPlayback, Repeat, Resume, Upcoming};

use crate::i18n::{tr, trf};
use crate::model::PlaybackSource;
use crate::player::{self, PlaybackStatus, RepeatMode};
use crate::signals::{State, CONTINUE};
use crate::{global_signal, settings, state};

// What another device plays or played; `None` hides the banner.
global_signal!(pub fn banner() -> State<Option<Elsewhere>> = State::new(None));
// The account's other players, for "Send to".
global_signal!(pub fn devices() -> State<Vec<Device>> = State::new(Vec::new()));

thread_local! {
    static HANDOFF: RefCell<Option<Arc<Handoff>>> = const { RefCell::new(None) };
    /// Bumped on every start and stop; late results and events are dropped.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
    static ELSEWHERE: RefCell<Option<Elsewhere>> = const { RefCell::new(None) };
    static CONTINUING: Cell<bool> = const { Cell::new(false) };
    static LAST_TRACK: RefCell<Option<String>> = const { RefCell::new(None) };
    static LAST_STATUS: Cell<Option<(PlaybackStatus, bool, bool)>> = const { Cell::new(None) };
    /// Bumped on every track start, repeats included: a send or continue
    /// that finishes after a newer start leaves that playback alone.
    static STARTS: Cell<u64> = const { Cell::new(0) };
    /// The latest `Upcoming` not applied yet.
    static UPCOMING: RefCell<Option<Upcoming>> = const { RefCell::new(None) };
    static RESUMED: RefCell<Option<Resumed>> = const { RefCell::new(None) };
}

const RETRY: Duration = Duration::from_mins(1);

fn current_generation() -> u64 {
    GENERATION.with(Cell::get)
}

fn next_generation() -> u64 {
    GENERATION.with(|g| {
        g.set(g.get() + 1);
        g.get()
    })
}

fn handoff() -> Option<Arc<Handoff>> {
    HANDOFF.with(|h| h.borrow().clone())
}

// ---------------------------------------------------------------- lifecycle

/// Runs while signed in. Call once, on the main thread.
pub fn init() {
    install_hooks();
    state::signed_in_changed().on(|signed_in| {
        if *signed_in {
            start();
        } else {
            stop();
        }
        CONTINUE
    });
}

/// Attaches this device as a player of the account; a failure retries later.
pub fn start() {
    if handoff().is_some() {
        return;
    }
    let Some(key) = settings::handoff_installation_key() else { return };
    let generation = next_generation();
    let l = listener(generation);
    let name = device_name();
    glib::spawn_future_local(async move {
        let r = gio::spawn_blocking(move || Handoff::start(&key, &name, l)).await;
        if generation != current_generation() {
            return; // stopped meanwhile; dropping the handoff detaches it
        }
        match r {
            Ok(Ok(h)) => {
                log::info!("handoff: started");
                HANDOFF.with(|s| *s.borrow_mut() = Some(Arc::new(h)));
                LAST_TRACK.with(RefCell::take);
                report();
            }
            Ok(Err(e)) => {
                log::warn!("handoff: start failed, retrying in {}s error={e}", RETRY.as_secs());
                retry(generation);
            }
            Err(_) => {
                log::error!("handoff: start panicked");
                retry(generation);
            }
        }
    });
}

fn retry(generation: u64) {
    glib::timeout_add_local_once(RETRY, move || {
        if generation == current_generation() && state::signed_in_changed().get() {
            start();
        }
    });
}

/// Detaches this device (sign-out, quit).
pub fn stop() {
    next_generation();
    if let Some(h) = HANDOFF.with(|s| s.borrow_mut().take()) {
        h.stop();
    }
    ELSEWHERE.with(RefCell::take);
    CONTINUING.with(|c| c.set(false));
    forget_resume();
    banner().set(None);
    devices().set(Vec::new());
}

/// Shown to the user's other devices as "Send to <name>".
fn device_name() -> String {
    std::fs::read_to_string("/etc/machine-info")
        .ok()
        .and_then(|s| pretty_hostname(&s))
        .unwrap_or_else(|| glib::host_name().to_string())
}

/// `PRETTY_HOSTNAME` from `/etc/machine-info` (shell-style quoting).
pub fn pretty_hostname(machine_info: &str) -> Option<String> {
    let v = machine_info.lines().find_map(|l| l.trim().strip_prefix("PRETTY_HOSTNAME="))?.trim();
    let v = match (v.chars().next(), v.chars().last()) {
        (Some(q @ ('"' | '\'')), Some(e)) if v.len() > 1 && q == e => &v[1..v.len() - 1],
        _ => v,
    };
    let mut out = String::new();
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        out.push(if c == '\\' { chars.next().unwrap_or(c) } else { c });
    }
    let out = out.trim().to_string();
    (!out.is_empty()).then_some(out)
}

// ---------------------------------------------------------------- events

#[derive(Debug)]
struct Listener {
    generation: u64,
    tx: async_channel::Sender<(u64, HandoffEvent)>,
}

impl HandoffListener for Listener {
    fn on_event(&self, ev: HandoffEvent) {
        drop(self.tx.try_send((self.generation, ev)));
    }
}

fn listener(generation: u64) -> Arc<dyn HandoffListener> {
    let (tx, rx) = async_channel::unbounded();
    glib::spawn_future_local(async move {
        while let Ok((g, ev)) = rx.recv().await {
            if g == current_generation() {
                on_event(ev);
            }
        }
    });
    Arc::new(Listener { generation, tx })
}

fn on_event(ev: HandoffEvent) {
    match ev {
        HandoffEvent::Devices(d) => devices().set(d),
        HandoffEvent::Elsewhere(e) => {
            ELSEWHERE.with(|s| s.borrow_mut().clone_from(&e));
            if !CONTINUING.with(Cell::get) {
                banner().set(e);
            }
        }
        // A Listen Together guest follows its host.
        HandoffEvent::Arrived(_) | HandoffEvent::MovedTo(_) | HandoffEvent::Upcoming(_) if crate::live::is_guest() => {}
        HandoffEvent::Arrived(r) => {
            let from = name(&r.from);
            forget_resume();
            match play(r) {
                Ok(()) => player::toasts().notify(&trf("Continued from %s", &[&from])),
                Err(e) => {
                    log::error!("handoff: could not play what arrived error={e:#}");
                    player::toasts().notify(&tr("Could not play the music sent from another device"));
                }
            }
        }
        HandoffEvent::MovedTo(d) => {
            // What follows a resume here no longer starts.
            forget_resume();
            // Also when paused: a pause cancels a resume that would start playing later.
            player::pause();
            player::toasts().notify(&trf("Playing on %s", &[&name(&d)]));
        }
        HandoffEvent::Upcoming(u) => {
            UPCOMING.with(|s| *s.borrow_mut() = Some(u));
            apply_upcoming();
        }
    }
}

/// The playback the last resume started.
struct Resumed {
    id: String,
    source: Option<PlaybackSource>,
    /// `player::play_requests()` right after its start: another start ends it.
    requests: u64,
    /// `player::pauses()` then: a pause or stop since keeps an ended track stopped.
    pauses: u64,
    loaded: bool,
}

#[derive(Debug, PartialEq, Eq)]
enum Step {
    Apply,
    /// The resumed track ended waiting for them: start them.
    Start,
    Wait,
    Drop,
}

/// What to do with the tracks after the resumed track. `owned`: nothing else
/// started since the resume; `current`: the resumed track still is the current
/// one; `ended`: playback stopped by itself since it loaded; `base_empty`:
/// nothing follows in the list.
fn upcoming_step(continuing: bool, owned: bool, loaded: bool, current: bool, ended: bool, base_empty: bool) -> Step {
    if !owned {
        Step::Drop
    } else if continuing || !loaded {
        // Its result decides, or its load does.
        Step::Wait
    } else if ended {
        Step::Start
    } else if current && base_empty {
        Step::Apply
    } else {
        // A filled list: the user queued something else, or a duplicate.
        Step::Drop
    }
}

fn apply_upcoming() {
    let Some(u) = UPCOMING.with(RefCell::take) else { return };
    let playing = player::current_track_id();
    let (owned, loaded, pauses, source) = RESUMED.with(|r| match r.borrow().as_ref() {
        Some(r) if r.id == u.current && r.requests == player::play_requests() => (true, r.loaded, r.pauses, r.source.clone()),
        _ => (false, false, 0, None),
    });
    let ended = player::playback_state_changed().get().status == PlaybackStatus::Stopped && pauses == player::pauses();
    match upcoming_step(CONTINUING.with(Cell::get), owned, loaded, playing.as_deref() == Some(u.current.as_str()), ended, player::base_queue().is_empty()) {
        // The base queue hook reports it, which ends the core's wait for it.
        Step::Apply => player::set_list_rest(u.tracks),
        Step::Start => {
            if let Err(e) = player::play_at(source, u.tracks, Duration::ZERO) {
                log::warn!("handoff: could not play what follows the resumed track error={e:#}");
            }
        }
        Step::Wait => UPCOMING.with(|s| *s.borrow_mut() = Some(u)),
        Step::Drop => {}
    }
}

/// The resumed track loaded: from now on a stop is its end.
fn resumed_loaded() {
    let playing = player::current_track_id();
    RESUMED.with(|r| {
        if let Some(r) = r.borrow_mut().as_mut()
            && Some(&r.id) == playing.as_ref()
            && r.requests == player::play_requests()
        {
            r.loaded = true;
        }
    });
}

fn forget_resume() {
    UPCOMING.with(RefCell::take);
    RESUMED.with(RefCell::take);
}

pub fn name(d: &Device) -> String {
    if d.name.is_empty() { tr("another device") } else { d.name.clone() }
}

/// Loads playback taken over from another device and plays it.
fn play(r: Resume) -> anyhow::Result<()> {
    let source = (!r.from.name.is_empty()).then(|| PlaybackSource::List { title: r.from.name.clone(), route: String::new() });
    let id = r.current.id().to_string();
    // Repeat-one would make the load pick the old track again: set it after.
    let before = player::repeat_mode_changed().get();
    player::set_repeat_mode(RepeatMode::None);
    let played = player::play_at(source.clone(), vec![r.current], Duration::from_millis(u64::try_from(r.position_ms).unwrap_or(0)));
    if played.is_ok() {
        let resumed = Resumed { id, source, requests: player::play_requests(), pauses: player::pauses(), loaded: false };
        RESUMED.with(|s| *s.borrow_mut() = Some(resumed));
    }
    // A failed load keeps the user's own repeat mode.
    player::set_repeat_mode(if played.is_ok() { repeat_mode(r.repeat) } else { before });
    played
}

// ---------------------------------------------------------------- actions

/// "Continue Here" on the banner.
pub fn continue_here() {
    if crate::live::guest_refuses() {
        return;
    }
    let Some(h) = handoff() else { return };
    let generation = current_generation();
    CONTINUING.with(|c| c.set(true));
    forget_resume();
    banner().set(None);
    let started = (starts(), player::play_requests(), player::pauses(), player::plays());
    glib::spawn_future_local(async move {
        let r = crate::api::run(move || h.continue_here()).await;
        if generation != current_generation() {
            return;
        }
        CONTINUING.with(|c| c.set(false));
        // Joined Listen Together meanwhile (the host drives playback now), or
        // the user played, started or paused something: that wins.
        if crate::live::is_guest() || (starts(), player::play_requests(), player::pauses(), player::plays()) != started {
            // The banner hid for this request; the other device's playback is still
            // there. Not for a guest, who can't continue it.
            if !crate::live::is_guest() {
                banner().set(ELSEWHERE.with(|s| s.borrow().clone()));
            }
            UPCOMING.with(RefCell::take);
            return;
        }
        match r.and_then(play) {
            Ok(()) => apply_upcoming(),
            Err(e) => {
                UPCOMING.with(RefCell::take);
                log::error!("handoff: continue here failed error={e:#}");
                player::toasts().notify(&tr("Could not continue playback here"));
                banner().set(ELSEWHERE.with(|s| s.borrow().clone()));
            }
        }
    });
}

/// "Send to <device>": hands this device's playback over and pauses here.
pub fn send_to(device_id: String) {
    if crate::live::guest_refuses() {
        return;
    }
    let Some(h) = handoff() else { return };
    let target = devices().with(|d| d.iter().find(|d| d.id == device_id).map(name)).unwrap_or_else(|| tr("another device"));
    h.report(local_now(None));
    let generation = current_generation();
    let sent = (starts(), player::play_requests(), player::plays());
    glib::spawn_future_local(async move {
        let r = crate::api::run(move || h.send_to(&device_id)).await;
        if generation != current_generation() {
            return;
        }
        match r {
            Ok(()) => {
                // Not what the user (or a joined Listen Together host) started while the send ran.
                if (starts(), player::play_requests(), player::plays()) == sent && !crate::live::is_guest() {
                    player::pause();
                }
                player::toasts().notify(&trf("Playing on %s", &[&target]));
            }
            Err(e) => {
                log::error!("handoff: send to failed error={e:#}");
                player::toasts().notify(&trf("Could not send playback to %s", &[&target]));
            }
        }
    });
}

// ---------------------------------------------------------------- report

fn ids(tracks: &[crate::model::Track]) -> Vec<String> {
    tracks.iter().map(|t| t.id().to_string()).collect()
}

/// The player as the core sees it: everything after the current track in
/// play order, the user queue ("play next") first.
pub fn local(current: Option<String>, user: Vec<String>, base: Vec<String>, position: Duration, playing: bool, repeat: RepeatMode) -> LocalPlayback {
    LocalPlayback {
        current,
        future: user.into_iter().chain(base).collect(),
        position_ms: i64::try_from(position.as_millis()).unwrap_or(i64::MAX),
        playing,
        repeat: match repeat {
            RepeatMode::None => Repeat::Off,
            RepeatMode::Track => Repeat::One,
            RepeatMode::Queue => Repeat::All,
        },
    }
}

pub fn repeat_mode(r: Repeat) -> RepeatMode {
    match r {
        Repeat::Off => RepeatMode::None,
        Repeat::One => RepeatMode::Track,
        Repeat::All => RepeatMode::Queue,
    }
}

/// `seek_target`: a seek just asked for; the pipeline may still report the
/// position before it.
fn local_now(seek_target: Option<Duration>) -> LocalPlayback {
    let ps = player::playback_state_changed().get();
    LAST_STATUS.with(|l| l.set(Some((ps.status, ps.loading, ps.buffering))));
    let current = player::current_track_id();
    let changed = LAST_TRACK.with(|l| {
        let changed = crate::live::track_change_position_is_stale(l.borrow().as_deref(), current.as_deref());
        if current.is_some() {
            l.borrow_mut().clone_from(&current);
        }
        changed
    });
    let position = if changed { Duration::ZERO } else { seek_target.or_else(player::position).unwrap_or(ps.position) };
    local(
        current,
        player::user_queue().entries().with(|t| ids(t)),
        player::base_queue().entries().with(|t| ids(t)),
        position,
        // Not while buffering: the position stands still then.
        ps.status == PlaybackStatus::Playing && !ps.loading && !ps.buffering,
        player::repeat_mode_changed().get(),
    )
}

fn report_at(seek_target: Option<Duration>) {
    // The host drives a Listen Together guest's player.
    if crate::live::is_guest() {
        return;
    }
    if let Some(h) = handoff() {
        h.report(local_now(seek_target));
    }
}

fn report() {
    report_at(None);
}

/// Player subscriptions, installed once; they only act while started.
fn starts() -> u64 {
    STARTS.with(Cell::get)
}

fn install_hooks() {
    player::track_changed().on_lazy(|_| {
        STARTS.with(|s| s.set(s.get() + 1));
        report();
        resumed_loaded();
        apply_upcoming();
        CONTINUE
    });
    // A repeat-one restart moves the position back to 0 without a track change.
    player::track_repeated().on(|_| {
        STARTS.with(|s| s.set(s.get() + 1));
        report();
        CONTINUE
    });
    player::user_queue().entries().on_lazy(|_| {
        report();
        CONTINUE
    });
    player::base_queue().entries().on_lazy(|_| {
        report();
        CONTINUE
    });
    player::repeat_mode_changed().on_lazy(|_| {
        report();
        CONTINUE
    });
    player::playback_state_changed().on_lazy(|ps| {
        // Position updates fire often; only status changes are reported.
        if LAST_STATUS.with(Cell::get) != Some((ps.status, ps.loading, ps.buffering)) {
            report();
        }
        CONTINUE
    });
    player::seeked().on(|to| {
        report_at(Some(*to));
        CONTINUE
    });
}

// ---------------------------------------------------------------- banner text

/// "2 h ago" for a Unix ms time.
pub fn ago(now_ms: i64, at_ms: i64) -> String {
    let mins = now_ms.saturating_sub(at_ms).max(0) / 60_000;
    match mins {
        0 => tr("just now"),
        1..60 => trf("%d min ago", &[&mins]),
        60..1440 => trf("%d h ago", &[&(mins / 60)]),
        _ => trf("%d d ago", &[&(mins / 1440)]),
    }
}

/// "Playing on <device>" or "Paused on <device> · <when>".
pub fn status_line(e: &Elsewhere, now_ms: i64) -> String {
    if e.playing {
        trf("Playing on %s", &[&name(&e.device)])
    } else {
        trf("Paused on %s · %s", &[&name(&e.device), &ago(now_ms, e.updated_at_ms)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MIN: i64 = 60_000;

    #[test]
    fn future_is_play_next_then_the_rest() {
        let s = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        let l = local(Some("1".into()), s(&["2", "3"]), s(&["4"]), Duration::from_millis(12_345), true, RepeatMode::Queue);
        assert_eq!(l, LocalPlayback { current: Some("1".into()), future: s(&["2", "3", "4"]), position_ms: 12_345, playing: true, repeat: Repeat::All });
        assert_eq!(local(None, vec![], vec![], Duration::ZERO, false, RepeatMode::None), LocalPlayback::default());
    }

    #[test]
    fn repeat_round_trips() {
        for m in [RepeatMode::None, RepeatMode::Track, RepeatMode::Queue] {
            assert_eq!(repeat_mode(local(None, vec![], vec![], Duration::ZERO, false, m).repeat), m);
        }
    }

    #[test]
    fn upcoming_follows_only_the_resumed_playback() {
        // (continuing, owned, loaded, current, ended, base_empty)
        assert_eq!(upcoming_step(false, true, true, true, false, true), Step::Apply);
        // Continue here still running, or the resumed track still loading.
        assert_eq!(upcoming_step(true, true, true, true, false, true), Step::Wait);
        assert_eq!(upcoming_step(false, true, false, false, false, true), Step::Wait);
        // It ended waiting for them.
        assert_eq!(upcoming_step(false, true, true, false, true, true), Step::Start);
        // Another start since, even of the same track.
        assert_eq!(upcoming_step(false, false, true, true, false, true), Step::Drop);
        assert_eq!(upcoming_step(true, false, false, false, false, true), Step::Drop);
        // A duplicate, or the user queued something meanwhile.
        assert_eq!(upcoming_step(false, true, true, true, false, false), Step::Drop);
    }

    #[test]
    fn relative_times() {
        let now = 1_000 * MIN;
        assert_eq!(ago(now, now - 30_000), "just now");
        assert_eq!(ago(now, now + MIN), "just now");
        assert_eq!(ago(now, now - 5 * MIN), "5 min ago");
        assert_eq!(ago(now, now - 59 * MIN), "59 min ago");
        assert_eq!(ago(now, now - 125 * MIN), "2 h ago");
        assert_eq!(ago(now, now - 3 * 1440 * MIN), "3 d ago");
    }

    #[test]
    fn status_lines() {
        let mut e = Elsewhere {
            device: Device { id: "a".into(), name: "iPhone".into() },
            track: crate::model::Track::default(),
            position_ms: 0,
            playing: true,
            updated_at_ms: 0,
        };
        assert_eq!(status_line(&e, 0), "Playing on iPhone");
        e.playing = false;
        assert_eq!(status_line(&e, 120 * MIN), "Paused on iPhone · 2 h ago");
        e.device.name.clear();
        assert_eq!(status_line(&e, 0), "Paused on another device · just now");
    }

    #[test]
    fn pretty_hostnames() {
        assert_eq!(pretty_hostname("CHASSIS=laptop\nPRETTY_HOSTNAME=\"Nico's Laptop\"\n").as_deref(), Some("Nico's Laptop"));
        assert_eq!(pretty_hostname("PRETTY_HOSTNAME='Desk'").as_deref(), Some("Desk"));
        assert_eq!(pretty_hostname("PRETTY_HOSTNAME=\"Say \\\"hi\\\"\"").as_deref(), Some("Say \"hi\""));
        assert_eq!(pretty_hostname("PRETTY_HOSTNAME=plain").as_deref(), Some("plain"));
        assert_eq!(pretty_hostname("PRETTY_HOSTNAME=\"\""), None);
        assert_eq!(pretty_hostname("CHASSIS=desktop"), None);
    }
}

//! Listen Together on the desktop: glue between the shared core
//! (`tonearm_core::live`, iroh transport) and the `GStreamer` player.
//!
//! The host publishes its queue and playback anchors; guests load the host's
//! track through their own TIDAL session and follow the anchors with the
//! core's drift policy with seeks only: the playback rate is never changed.
//! Only metadata travels.

pub mod link;
pub mod state;

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gtk::gio;
use gtk::gio::prelude::*;
use tonearm_core::live::drift::{Correction, SEEK_MS};
use tonearm_core::live::net::{LiveEvent, LiveGuest, LiveHost, LiveListener, NetConfig};
use tonearm_core::live::proto::GuestCommand;
use tonearm_core::live::session::{GuestEvent, HostCommand};
use tonearm_core::live::queue::QueueItem;
use tonearm_core::live::MAX_QUEUE;

use crate::i18n::tr;
use crate::player::{self, PlaybackStatus};
use crate::signals::{State, CONTINUE};
use crate::{global_signal, settings};
pub use state::{Ended, Phase};

global_signal!(pub fn phase() -> State<Phase> = State::new(Phase::Idle));
// Guest playback control: the host's toggle while hosting, the host's
// permission while listening. Off otherwise.
global_signal!(pub fn guest_control() -> State<bool> = State::new(false));

enum Session {
    Host(Arc<LiveHost>),
    Guest(Arc<LiveGuest>),
}

thread_local! {
    static SESSION: RefCell<Option<Session>> = const { RefCell::new(None) };
    /// Bumped for every new or ended session; stale events are dropped.
    static GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Host: entry id of the current track in the shared queue.
    static CURRENT_ITEM: RefCell<Option<String>> = const { RefCell::new(None) };
    /// Host: last published (status, loading).
    static LAST_STATUS: Cell<Option<(PlaybackStatus, bool)>> = const { Cell::new(None) };
    /// Guest: track loads started to follow the host.
    static LOADS: RefCell<LoadGate> = RefCell::new(LoadGate::default());
    static HOOKED: Cell<bool> = const { Cell::new(false) };
    static OUTPUT_DELAY: Cell<i32> = const { Cell::new(0) };
    /// Host: accepted guest track ids waiting to be fetched, in arrival order.
    static HOST_ENQUEUE: RefCell<VecDeque<(u64, String)>> = const { RefCell::new(VecDeque::new()) };
    static HOST_ENQUEUE_BUSY: Cell<bool> = const { Cell::new(false) };
    /// Guest: enqueue chunks paced against the command budget.
    static OUTBOX: RefCell<Outbox<GuestCommand>> = RefCell::new(Outbox::default());
    static OUTBOX_BUSY: Cell<bool> = const { Cell::new(false) };
}

const OUTPUT_DELAY_KEY: &str = "live-output-delay";
const GUEST_TICK: Duration = Duration::from_millis(250);
const HOST_REFRESH: Duration = Duration::from_secs(5);

fn current_generation() -> u64 {
    GENERATION.with(Cell::get)
}

fn next_generation() -> u64 {
    GENERATION.with(|g| {
        g.set(g.get() + 1);
        g.get()
    })
}

fn host() -> Option<Arc<LiveHost>> {
    SESSION.with(|s| match &*s.borrow() {
        Some(Session::Host(h)) => Some(Arc::clone(h)),
        _ => None,
    })
}

fn guest() -> Option<Arc<LiveGuest>> {
    SESSION.with(|s| match &*s.borrow() {
        Some(Session::Guest(g)) => Some(Arc::clone(g)),
        _ => None,
    })
}

// ---------------------------------------------------------------- settings

fn has_delay_key() -> bool {
    settings::player().settings_schema().is_some_and(|s| s.has_key(OUTPUT_DELAY_KEY))
}

/// Extra delay (ms) a guest adds for its audio output (Bluetooth etc.).
/// Falls back to an in-memory value when an older schema is installed.
pub fn output_delay_ms() -> i32 {
    if has_delay_key() { settings::player().int(OUTPUT_DELAY_KEY) } else { OUTPUT_DELAY.with(Cell::get) }
}

pub fn set_output_delay_ms(ms: i32) {
    if has_delay_key() {
        if let Err(e) = settings::player().set_int(OUTPUT_DELAY_KEY, ms) {
            log::warn!("failed to store the Listen Together output delay error={e}");
        }
    } else {
        OUTPUT_DELAY.with(|d| d.set(ms));
    }
}

// ---------------------------------------------------------------- events

#[derive(Debug)]
struct Listener {
    generation: u64,
    tx: async_channel::Sender<(u64, LiveEvent)>,
}

impl LiveListener for Listener {
    fn on_event(&self, ev: LiveEvent) {
        // Called from the transport's threads; hand over to the main loop.
        drop(self.tx.try_send((self.generation, ev)));
    }
}

fn listener(generation: u64) -> Arc<dyn LiveListener> {
    let (tx, rx) = async_channel::unbounded();
    glib::spawn_future_local(async move {
        while let Ok((g, ev)) = rx.recv().await {
            if g == current_generation() {
                on_event(&ev);
            }
        }
    });
    Arc::new(Listener { generation, tx })
}

fn on_event(ev: &LiveEvent) {
    match ev {
        LiveEvent::Command(c) => {
            apply_guest_command(c);
            return;
        }
        LiveEvent::Guest(GuestEvent::ControlChanged(allowed)) if guest().is_some() => guest_control().set(*allowed),
        _ => {}
    }
    let current = phase().get();
    if let Some(next) = state::apply_event(&current, ev) {
        if let Phase::Ended(e) = &next {
            player::toasts().notify(&state::ended_message(e));
            teardown();
        } else if next == Phase::Idle {
            teardown();
        }
        phase().set(next);
    }
    if guest().is_some() {
        guest_tick();
    }
}

/// Name shown to the other side. The login name, not the account's real
/// (GECOS) name: invitations can come from anyone, so joining must not
/// disclose the user's full name.
fn display_name() -> String {
    glib::user_name().to_string_lossy().trim().to_string()
}

// ---------------------------------------------------------------- lifecycle

/// Host: opens a session and starts publishing the local player.
pub fn start_host() {
    if phase().get().is_active() {
        return;
    }
    let generation = next_generation();
    phase().set(Phase::Starting);
    let l = listener(generation);
    let name = display_name();
    glib::spawn_future_local(async move {
        let r = gio::spawn_blocking(move || LiveHost::start(name, &NetConfig::default(), l)).await;
        if generation != current_generation() {
            if let Ok(Ok(h)) = r {
                drop(gio::spawn_blocking(move || h.end()));
            }
            return;
        }
        match r {
            Ok(Ok(h)) => {
                let h = Arc::new(h);
                let url = h.ticket_url();
                SESSION.with(|s| *s.borrow_mut() = Some(Session::Host(h)));
                install_hooks();
                guest_control().set(false);
                phase().set(Phase::Hosting { url, guests: Vec::new() });
                publish_queue();
                start_host_refresh(generation);
            }
            Ok(Err(e)) => fail(&format!("{e}")),
            Err(_) => fail("host start panicked"),
        }
    });
}

/// Guest: joins from an `sdat://join/<ticket>` link.
pub fn join(url: &str) {
    match phase().get() {
        p if p.is_host() => {
            player::toasts().notify(&tr("End your session before joining another one"));
            return;
        }
        p if p.is_active() => end(),
        _ => {}
    }
    let generation = next_generation();
    phase().set(Phase::Joining);
    match LiveGuest::join(url, display_name(), &NetConfig::default(), listener(generation)) {
        Ok(g) => {
            // Local "play next" requests made before joining must not reach the host.
            player::invalidate_play_next();
            SESSION.with(|s| *s.borrow_mut() = Some(Session::Guest(Arc::new(g))));
            install_hooks();
            start_guest_tick(generation);
        }
        Err(e) => fail(&format!("{e}")),
    }
}

fn fail(err: &str) {
    log::error!("Listen Together failed error={err}");
    teardown();
    let e = Ended::Failed;
    player::toasts().notify(&state::ended_message(&e));
    phase().set(Phase::Ended(e));
}

/// Ends (host) or leaves (guest) the current session.
pub fn end() {
    match SESSION.with(|s| s.borrow_mut().take()) {
        Some(Session::Host(h)) => drop(gio::spawn_blocking(move || h.end())),
        Some(Session::Guest(g)) => g.leave(),
        None => {}
    }
    teardown();
    phase().set(Phase::Idle);
}

/// Clears an "ended" message.
pub fn dismiss() {
    if matches!(phase().get(), Phase::Ended(_)) {
        phase().set(Phase::Idle);
    }
}

/// Stops everything tied to the current session; bumping the generation
/// also stops its timers and drops its late events.
fn teardown() {
    next_generation();
    player::invalidate_play_next();
    let was_guest = matches!(SESSION.with(|s| s.borrow_mut().take()), Some(Session::Guest(_))) || phase().get().is_guest();
    CURRENT_ITEM.with(|c| c.borrow_mut().take());
    if guest_control().get() {
        guest_control().set(false);
    }
    let loading = LOADS.with(RefCell::take).is_loading();
    LAST_STATUS.with(|l| l.set(None));
    if was_guest {
        // A host track still loading would start after we left: the player
        // handles triggers in order, so this stop lands after that load.
        if loading {
            player::stop();
        }
    }
}

// ---------------------------------------------------------------- guest control

/// Host: lets guests play/pause, skip and add tracks (off by default).
pub fn set_allow_guest_control(allowed: bool) {
    let Some(h) = host() else { return };
    h.set_guest_control(allowed);
    guest_control().set(h.guest_control());
}

/// Host: applies an accepted guest command through the normal player path;
/// the usual queue and playback publishing then updates every guest.
fn apply_guest_command(c: &HostCommand) {
    let Some(h) = host() else { return };
    if !h.guest_control() {
        return;
    }
    log::info!("applying a Listen Together guest command peer={} cmd={:?}", c.peer, c.cmd);
    match &c.cmd {
        GuestCommand::Play => player::play(),
        GuestCommand::Pause => player::pause(),
        GuestCommand::Next => player::next(),
        GuestCommand::Enqueue { track_ids } => {
            let generation = current_generation();
            HOST_ENQUEUE.with(|q| q.borrow_mut().extend(track_ids.iter().map(|id| (generation, id.clone()))));
            pump_host_enqueue();
        }
        GuestCommand::Unknown => {}
    }
}

/// `true` while the host session that accepted a guest request is still
/// live and still lets guests control playback.
fn guest_request_still_valid(generation: u64) -> bool {
    generation == current_generation() && host().is_some_and(|h| h.guest_control())
}

/// Host: fetches queued guest tracks one at a time so they land in the order
/// they were sent, and drops them once the session ends or control is
/// turned off (checked again after every fetch).
fn pump_host_enqueue() {
    if HOST_ENQUEUE_BUSY.with(|b| b.replace(true)) {
        return;
    }
    glib::spawn_future_local(async {
        while let Some((generation, id)) = HOST_ENQUEUE.with(|q| q.borrow_mut().pop_front()) {
            if !guest_request_still_valid(generation) {
                continue;
            }
            let fetch_id = id.clone();
            match crate::api::run(move || crate::service::get_track(&fetch_id)).await {
                Ok(track) if guest_request_still_valid(generation) => player::add_tracklist_to_user_queue(vec![track]),
                Ok(_) => {}
                Err(e) => log::warn!("failed to queue a guest track id={id} error={e}"),
            }
        }
        HOST_ENQUEUE_BUSY.with(|b| b.set(false));
    });
}

/// Guest commands waiting for room in the command budget, tagged with the
/// session they belong to.
#[derive(Debug)]
pub struct Outbox<T> {
    items: VecDeque<(u64, T)>,
    failures: u32,
}

impl<T> Default for Outbox<T> {
    fn default() -> Self {
        Self { items: VecDeque::new(), failures: 0 }
    }
}

/// What the outbox needs next.
#[derive(Debug, PartialEq, Eq)]
pub enum Pump {
    /// Everything was sent (or belonged to an old session).
    Empty,
    /// The head could not be sent yet; try again after a short wait.
    Wait,
    /// The host stayed unreachable; the rest was dropped.
    GaveUp,
}

/// Retries for one blocked item, `OUTBOX_RETRY` apart (about 10 s).
pub const OUTBOX_RETRIES: u32 = 20;
const OUTBOX_RETRY: Duration = Duration::from_millis(500);

impl<T: Clone> Outbox<T> {
    pub fn push(&mut self, generation: u64, item: T) {
        self.items.push_back((generation, item));
    }

    /// Sends items in order until one is refused; stale items are skipped.
    pub fn pump(&mut self, generation: u64, mut send: impl FnMut(T) -> bool) -> Pump {
        while let Some((g, item)) = self.items.front().cloned() {
            if g != generation {
                self.items.pop_front();
                continue;
            }
            if !send(item) {
                self.failures += 1;
                if self.failures >= OUTBOX_RETRIES {
                    self.items.clear();
                    self.failures = 0;
                    return Pump::GaveUp;
                }
                return Pump::Wait;
            }
            self.failures = 0;
            self.items.pop_front();
        }
        Pump::Empty
    }
}

fn pump_outbox() {
    if OUTBOX_BUSY.with(|b| b.replace(true)) {
        return;
    }
    glib::spawn_future_local(async {
        loop {
            let generation = current_generation();
            let g = guest();
            let step = OUTBOX.with(|o| {
                o.borrow_mut().pump(generation, |cmd| g.as_ref().is_some_and(|g| guest_control().get() && g.send_command(cmd)))
            });
            match step {
                Pump::Empty => break,
                Pump::Wait => glib::timeout_future(OUTBOX_RETRY).await,
                Pump::GaveUp => {
                    if guest().is_some() {
                        player::toasts().notify(&tr("Could not reach the host, try again in a moment"));
                    }
                    break;
                }
            }
        }
        OUTBOX_BUSY.with(|b| b.set(false));
    });
}

/// What a user action does while listening to someone else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route {
    /// Not a guest: act locally.
    Local,
    /// Send this to the host.
    Send(GuestCommand),
    /// A guest cannot do this now.
    Locked,
}

/// Pure routing of a guest's play/pause: with permission it asks the host
/// for the opposite of the host's state.
pub fn route_play_pause(is_guest: bool, allowed: bool, host_playing: bool) -> Route {
    match (is_guest, allowed) {
        (false, _) => Route::Local,
        (true, false) => Route::Locked,
        (true, true) => Route::Send(if host_playing { GuestCommand::Pause } else { GuestCommand::Play }),
    }
}

pub fn route(is_guest: bool, allowed: bool, cmd: GuestCommand) -> Route {
    match (is_guest, allowed) {
        (false, _) => Route::Local,
        (true, false) => Route::Locked,
        (true, true) => Route::Send(cmd),
    }
}

/// Runs a routed action; returns `true` when it was handled here (sent or
/// refused), `false` when the caller should act locally.
fn run(r: Route) -> bool {
    match r {
        Route::Local => false,
        Route::Locked => {
            player::toasts().notify(&tr("The host controls playback"));
            true
        }
        Route::Send(cmd) => {
            if !guest().is_some_and(|g| g.send_command(cmd)) {
                player::toasts().notify(&tr("Could not reach the host, try again in a moment"));
            }
            true
        }
    }
}

pub(crate) fn is_guest() -> bool {
    guest().is_some()
}

/// UI play/pause. Guests ask the host (or are refused).
pub fn user_play_pause() {
    let playing = guest().and_then(|g| g.target(0)).is_some_and(|t| t.playing);
    if !run(route_play_pause(is_guest(), guest_control().get(), playing)) {
        player::play_pause();
    }
}

pub fn user_play() {
    if !run(route(is_guest(), guest_control().get(), GuestCommand::Play)) {
        player::play();
    }
}

pub fn user_pause() {
    if !run(route(is_guest(), guest_control().get(), GuestCommand::Pause)) {
        player::pause();
    }
}

pub fn user_next() {
    if !run(route(is_guest(), guest_control().get(), GuestCommand::Next)) {
        player::next();
    }
}

/// Actions guests can never take (previous, seek, stop, shuffle, repeat):
/// `true` when refused.
pub fn guest_refuses() -> bool {
    is_guest() && run(Route::Locked)
}

/// UI "add to queue". Guests send the track ids to the host in chunks the
/// protocol accepts; `false` means act locally.
pub fn user_enqueue(track_ids: Vec<String>) -> bool {
    if !is_guest() {
        return false;
    }
    if !guest_control().get() {
        return run(Route::Locked);
    }
    // Chunks wait for room in the command budget instead of being dropped.
    let generation = current_generation();
    OUTBOX.with(|o| {
        let mut o = o.borrow_mut();
        for chunk in track_ids.chunks(tonearm_core::live::session::MAX_ENQUEUE) {
            o.push(generation, GuestCommand::Enqueue { track_ids: chunk.to_vec() });
        }
    });
    pump_outbox();
    true
}

// ---------------------------------------------------------------- host

/// Track ids shared with guests: current track first, then the user queue
/// and the rest of the playing list. Ids outside the protocol's rule
/// (≤ 40 bytes, `[A-Za-z0-9-_:.]`) are left out, and the list is capped.
pub fn shared_track_ids(current: Option<&str>, user: &[String], base: &[String]) -> Vec<String> {
    current
        .into_iter()
        .map(str::to_string)
        .chain(user.iter().cloned())
        .chain(base.iter().cloned())
        .filter(|id| QueueItem { item_id: "x".into(), track_id: id.clone() }.is_valid())
        .take(MAX_QUEUE)
        .collect()
}

fn ids(tracks: &[crate::model::Track]) -> Vec<String> {
    tracks.iter().map(|t| t.id().to_string()).collect()
}

fn publish_queue() {
    let Some(h) = host() else { return };
    let current = player::current_track_id();
    let user = player::user_queue().entries().with(|t| ids(t));
    let base = player::base_queue().entries().with(|t| ids(t));
    let shared = shared_track_ids(current.as_deref(), &user, &base);
    let current_valid = current.as_ref().is_some_and(|c| shared.first() == Some(c));
    match h.replace_queue(shared) {
        Ok(item_ids) => CURRENT_ITEM.with(|c| *c.borrow_mut() = if current_valid { item_ids.into_iter().next() } else { None }),
        Err(e) => log::warn!("failed to share the queue error={e}"),
    }
    publish_playback();
}

fn publish_playback() {
    let Some(h) = host() else { return };
    let ps = player::playback_state_changed().get();
    LAST_STATUS.with(|l| l.set(Some((ps.status, ps.loading))));
    let pos = player::position().unwrap_or(ps.position);
    let playing = ps.status == PlaybackStatus::Playing && !ps.loading;
    let item = CURRENT_ITEM.with(|c| c.borrow().clone());
    let track = if item.is_some() { player::current_track_id() } else { None };
    h.set_playback(item, track, i64::try_from(pos.as_millis()).unwrap_or(i64::MAX), playing);
}

fn start_host_refresh(generation: u64) {
    glib::timeout_add_local(HOST_REFRESH, move || {
        if generation != current_generation() {
            return glib::ControlFlow::Break;
        }
        publish_playback();
        glib::ControlFlow::Continue
    });
}

/// Player subscriptions, installed once; they only act while hosting.
fn install_hooks() {
    if HOOKED.with(|h| h.replace(true)) {
        return;
    }
    player::track_changed().on(|_| {
        publish_queue();
        CONTINUE
    });
    player::user_queue().entries().on(|_| {
        publish_queue();
        CONTINUE
    });
    player::base_queue().entries().on(|_| {
        publish_queue();
        CONTINUE
    });
    player::playback_state_changed().on(|ps| {
        // Position updates fire often; only status changes are anchors.
        if host().is_some() && LAST_STATUS.with(Cell::get) != Some((ps.status, ps.loading)) {
            publish_playback();
        }
        CONTINUE
    });
    player::seeked().on(|_| {
        publish_playback();
        CONTINUE
    });
}

// ---------------------------------------------------------------- guest

fn start_guest_tick(generation: u64) {
    glib::timeout_add_local(GUEST_TICK, move || {
        if generation != current_generation() {
            return glib::ControlFlow::Break;
        }
        guest_tick();
        glib::ControlFlow::Continue
    });
}

fn ms(d: Duration) -> i64 {
    i64::try_from(d.as_millis()).unwrap_or(i64::MAX)
}

/// While paused, guests only seek when they are clearly elsewhere.
pub fn paused_needs_seek(target_ms: i64, actual_ms: i64) -> bool {
    (target_ms - actual_ms).abs() > SEEK_MS
}

/// How long a track the guest's account could not play is left alone.
const RETRY_FAILED: Duration = Duration::from_secs(15);
/// A started load that has not reached the player after this long is
/// treated as failed.
const LOAD_TIMEOUT: Duration = Duration::from_secs(20);
/// The same while the player reports it is still fetching a stream.
const LOAD_TIMEOUT_STREAMING: Duration = Duration::from_mins(1);

/// Guest: which host tracks are being loaded or failed recently, so a tick
/// never starts a second load of the same track and an unavailable track is
/// not retried (and toasted) every tick.
#[derive(Debug, Default)]
pub struct LoadGate {
    pending: Option<(String, Instant)>,
    /// Bumped by every started load; a result belongs to the current
    /// attempt only if it still carries this number.
    attempt: u64,
    failed: Option<(String, Instant)>,
}

impl LoadGate {
    /// Whether a load of `track` should start now.
    #[cfg(test)]
    pub fn begin(&mut self, track: &str, now: Instant) -> bool {
        self.begin_while(track, now, false)
    }
    /// Like [`Self::begin`]; `player_loading`: the player is fetching a
    /// stream, so our own pending load gets [`LOAD_TIMEOUT_STREAMING`]
    /// before it counts as failed (a stuck stream still times out).
    pub fn begin_while(&mut self, track: &str, now: Instant, player_loading: bool) -> bool {
        if let Some((t, at)) = &self.pending
            && t == track
        {
            let limit = if player_loading { LOAD_TIMEOUT_STREAMING } else { LOAD_TIMEOUT };
            if now.duration_since(*at) < limit {
                return false;
            }
            // The metadata loaded but the player never got the track (e.g.
            // its stream request failed): count it as a failure so it is
            // retried after RETRY_FAILED instead of never.
            self.failed(track, now);
            return false;
        }
        if let Some((t, at)) = &self.failed
            && t == track
            && now.duration_since(*at) < RETRY_FAILED
        {
            return false;
        }
        self.pending = Some((track.to_string(), now));
        self.attempt += 1;
        true
    }
    /// The number of the load started by the last successful [`Self::begin`].
    pub fn attempt(&self) -> u64 {
        self.attempt
    }
    /// Whether `attempt` is still the pending load of `track`: late or
    /// superseded results must not start playback.
    pub fn is_current(&self, track: &str, attempt: u64) -> bool {
        self.attempt == attempt && self.pending.as_ref().is_some_and(|(t, _)| t == track)
    }
    /// Whether a host track load is in flight.
    pub fn is_loading(&self) -> bool {
        self.pending.is_some()
    }
    /// The player now has the host's track.
    pub fn loaded(&mut self) {
        self.pending = None;
        self.failed = None;
    }
    /// A load ended without playing (failed or superseded).
    /// Returns true on the first failure of this track (worth a toast).
    pub fn failed(&mut self, track: &str, now: Instant) -> bool {
        if self.pending.as_ref().is_some_and(|(t, _)| t == track) {
            self.pending = None;
        }
        let first = self.failed.as_ref().is_none_or(|(t, _)| t != track);
        self.failed = Some((track.to_string(), now));
        first
    }
    /// A load was dropped because the host moved on.
    pub fn abandon(&mut self, track: &str) {
        if self.pending.as_ref().is_some_and(|(t, _)| t == track) {
            self.pending = None;
        }
    }
}

/// Fetches the host's track with the guest's own TIDAL session, then plays it
/// only if the same session still wants it, honouring the host's pause.
fn load_host_track(track: String, attempt: u64) {
    let generation = current_generation();
    glib::spawn_future_local(async move {
        let id = track.clone();
        let r = crate::api::run(move || crate::service::get_track(&id)).await;
        if generation != current_generation() {
            return; // session ended or changed meanwhile; teardown reset the gate
        }
        if !LOADS.with(|l| l.borrow().is_current(&track, attempt)) {
            return; // timed out and retried, or superseded: a newer attempt owns it
        }
        let latency = i64::from(output_delay_ms());
        let want = guest().and_then(|g| g.target(latency));
        if want.as_ref().and_then(|t| t.track_id.as_deref()) != Some(track.as_str()) {
            LOADS.with(|l| l.borrow_mut().abandon(&track));
            return;
        }
        match r {
            Ok(t) => {
                player::play_track(t);
                if want.is_some_and(|t| !t.playing) {
                    player::pause();
                }
            }
            Err(e) => {
                log::error!("failed to load the host's track error={e:#}");
                if LOADS.with(|l| l.borrow_mut().failed(&track, Instant::now())) {
                    player::toasts().notify(&tr("Could not play the host's track with your account"));
                }
            }
        }
    });
}

/// Moves the local player towards the host's anchor.
fn guest_tick() {
    let Some(g) = guest() else { return };
    let latency = i64::from(output_delay_ms());
    let Some(t) = g.target(latency) else { return };
    let ps = player::playback_state_changed().get();
    let local_playing = ps.status == PlaybackStatus::Playing;
    let Some(track) = t.track_id else {
        if local_playing {
            player::pause();
        }
        return;
    };
    if player::current_track_id().as_deref() != Some(track.as_str()) {
        let started = LOADS.with(|l| {
            let mut l = l.borrow_mut();
            l.begin_while(&track, Instant::now(), ps.loading).then(|| l.attempt())
        });
        if let Some(attempt) = started {
            load_host_track(track, attempt);
        }
        return;
    }
    LOADS.with(|l| l.borrow_mut().loaded());
    if ps.loading {
        return;
    }
    if t.playing != local_playing {
        if t.playing { player::play() } else { player::pause() }
        return;
    }
    let Some(actual) = player::position().map(ms) else { return };
    if !t.playing {
        if paused_needs_seek(t.position_ms, actual) {
            player::seek_to_position(Duration::from_millis(u64::try_from(t.position_ms.max(0)).unwrap_or(0)), true);
        }
        return;
    }
    match g.correction(latency, actual) {
        // Never change the playback speed: a rate answer is ignored.
        Correction::None | Correction::Rate(_) => {}
        Correction::Seek(to) => player::seek_to_position(Duration::from_millis(u64::try_from(to.max(0)).unwrap_or(0)), true),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_actions_follow_the_host_permission() {
        assert_eq!(route(false, false, GuestCommand::Next), Route::Local, "not a guest: local player");
        assert_eq!(route(true, false, GuestCommand::Next), Route::Locked, "locked by default");
        assert_eq!(route(true, true, GuestCommand::Next), Route::Send(GuestCommand::Next));
        assert_eq!(route_play_pause(true, true, true), Route::Send(GuestCommand::Pause));
        assert_eq!(route_play_pause(true, true, false), Route::Send(GuestCommand::Play));
        assert_eq!(route_play_pause(true, false, true), Route::Locked);
        assert_eq!(route_play_pause(false, false, true), Route::Local);
    }

    #[test]
    fn outbox_waits_for_budget_and_keeps_order() {
        let mut o = Outbox::default();
        for i in 0..6 {
            o.push(1, i);
        }
        let mut sent = Vec::new();
        let mut budget = 5;
        let send = |i: i32, budget: &mut i32, sent: &mut Vec<i32>| {
            if *budget == 0 {
                return false;
            }
            *budget -= 1;
            sent.push(i);
            true
        };
        assert_eq!(o.pump(1, |i| send(i, &mut budget, &mut sent)), Pump::Wait, "sixth chunk waits");
        budget = 5;
        assert_eq!(o.pump(1, |i| send(i, &mut budget, &mut sent)), Pump::Empty);
        assert_eq!(sent, vec![0, 1, 2, 3, 4, 5], "nothing dropped, order kept");
    }

    #[test]
    fn outbox_drops_old_sessions_and_gives_up() {
        let mut o = Outbox::default();
        o.push(1, "old");
        o.push(2, "new");
        let mut sent = Vec::new();
        assert_eq!(o.pump(2, |i| { sent.push(i); true }), Pump::Empty);
        assert_eq!(sent, vec!["new"]);
        o.push(2, "x");
        for _ in 1..OUTBOX_RETRIES {
            assert_eq!(o.pump(2, |_| false), Pump::Wait);
        }
        assert_eq!(o.pump(2, |_| false), Pump::GaveUp);
        assert_eq!(o.pump(2, |_| true), Pump::Empty, "cleared after giving up");
    }

    fn v(ids: &[&str]) -> Vec<String> {
        ids.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn current_track_goes_first() {
        assert_eq!(shared_track_ids(Some("1"), &v(&["2"]), &v(&["3", "4"])), v(&["1", "2", "3", "4"]));
        assert_eq!(shared_track_ids(None, &[], &v(&["3"])), v(&["3"]));
    }

    #[test]
    fn invalid_ids_are_skipped() {
        let long = "9".repeat(41);
        assert_eq!(shared_track_ids(Some("a b"), &v(&[&long, "ok-1"]), &v(&["é"])), v(&["ok-1"]));
    }

    #[test]
    fn queue_is_capped() {
        let base: Vec<String> = (0..MAX_QUEUE + 10).map(|i| i.to_string()).collect();
        assert_eq!(shared_track_ids(Some("x"), &[], &base).len(), MAX_QUEUE);
    }

    #[test]
    fn load_gate_starts_each_track_once() {
        let now = Instant::now();
        let mut g = LoadGate::default();
        assert!(g.begin("1", now));
        assert!(!g.begin("1", now));
        assert!(g.begin("2", now), "a new host track starts its own load");
        g.loaded();
        assert!(g.begin("2", now));
    }

    #[test]
    fn load_gate_backs_off_failed_tracks() {
        let now = Instant::now();
        let mut g = LoadGate::default();
        assert!(g.begin("1", now));
        assert!(g.failed("1", now), "first failure is reported");
        assert!(!g.begin("1", now + Duration::from_secs(1)));
        assert!(g.begin("1", now + RETRY_FAILED));
        assert!(!g.failed("1", now + RETRY_FAILED), "repeated failure is not reported again");
        assert!(g.begin("2", now), "other tracks are not blocked");
    }

    #[test]
    fn load_that_never_reaches_the_player_is_retried() {
        let now = Instant::now();
        let mut g = LoadGate::default();
        assert!(g.begin("1", now));
        assert!(g.is_loading());
        assert!(!g.begin("1", now + LOAD_TIMEOUT - Duration::from_secs(1)), "still loading");
        assert!(!g.begin("1", now + LOAD_TIMEOUT), "timed out: backs off first");
        assert!(!g.is_loading());
        assert!(g.begin("1", now + LOAD_TIMEOUT + RETRY_FAILED), "then retried");
    }

    #[test]
    fn late_result_of_a_timed_out_attempt_is_rejected() {
        let now = Instant::now();
        let mut g = LoadGate::default();
        assert!(g.begin("1", now));
        let first = g.attempt();
        assert!(g.is_current("1", first));
        assert!(!g.begin("1", now + LOAD_TIMEOUT));
        assert!(!g.is_current("1", first), "timed out");
        assert!(g.begin("1", now + LOAD_TIMEOUT + RETRY_FAILED));
        assert!(!g.is_current("1", first), "the retry owns the track now");
        assert!(g.is_current("1", g.attempt()));
    }

    #[test]
    fn streaming_load_gets_longer_but_still_times_out() {
        let now = Instant::now();
        let mut g = LoadGate::default();
        assert!(g.begin("1", now));
        assert!(!g.begin_while("1", now + LOAD_TIMEOUT, true));
        assert!(g.is_loading(), "a stream still loading is not timed out at LOAD_TIMEOUT");
        assert!(!g.begin_while("1", now + LOAD_TIMEOUT_STREAMING, true));
        assert!(!g.is_loading(), "a stuck stream eventually counts as failed");
        assert!(g.begin_while("2", now, true), "another track is never blocked by the player loading");
    }

    #[test]
    fn abandoned_load_can_restart() {
        let now = Instant::now();
        let mut g = LoadGate::default();
        assert!(g.begin("1", now));
        g.abandon("1");
        assert!(g.begin("1", now));
    }

    #[test]
    fn paused_seek_threshold() {
        assert!(!paused_needs_seek(10_000, 10_000 - SEEK_MS));
        assert!(paused_needs_seek(10_000, 10_000 - SEEK_MS - 1));
        assert!(paused_needs_seek(0, 5_000));
    }
}

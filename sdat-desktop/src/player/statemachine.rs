//! Port of `internal/player/statemachine` plus the transition table from
//! `internal/player/statemachine.go`. The Go code used qmuntal/stateless in
//! queued firing mode; here the table is an explicit `match` and the queue is
//! drained by a single main-loop future so async API calls inside entry
//! actions never interleave with other transitions.

use anyhow::{anyhow, Result};
use gst::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::model::{Track, TrackManifest, TrackQuality};
use crate::signals::{Signal, State as Sig};
use crate::{api, global_signal, service, settings};

// ---------------------------------------------------------------- signals

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackStatus {
    Playing,
    Paused,
    #[default]
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlaybackState {
    pub duration: Duration,
    pub position: Duration,
    pub status: PlaybackStatus,
    /// A track is being fetched or started; its controls are locked.
    pub loading: bool,
    /// The playing stream ran dry and waits, paused, for its buffer to fill.
    pub buffering: bool,
}

#[derive(Debug, Clone)]
pub struct TrackChangedEvent {
    pub track: Track,
    pub skip_history: bool,
}

global_signal!(pub fn playback_state_changed() -> Sig<PlaybackState> = Sig::new(PlaybackState::default()));
global_signal!(pub fn playback_quality_changed() -> Sig<TrackQuality> = Sig::new(TrackQuality::HighResLossless));
global_signal!(pub fn track_changed() -> Sig<Option<TrackChangedEvent>> = Sig::new(None));
// Gapless repeat-one replays the same track without a `track_changed`.
global_signal!(pub fn track_repeated() -> Signal<Track> = Signal::default());

// ---------------------------------------------------------------- gapless gate

#[derive(Debug, Default)]
struct WaitState {
    blocking_ops: usize,
    generation: u64,
    waiting: bool,
    done: bool,
}

/// Coordinates `GStreamer`'s streaming-thread `about-to-finish` wait with
/// main-thread playbin operations (`set_state(Ready | Null)`, flushing seeks)
/// that acquire `GST_PAD_STREAM_LOCK`, preventing a 15 s deadlock.
#[derive(Debug, Default)]
pub struct GaplessGate {
    state: Mutex<WaitState>,
    cond: Condvar,
}

#[derive(Debug)]
pub struct GaplessToken {
    gate: Arc<GaplessGate>,
    generation: u64,
    finished: AtomicBool,
}

#[derive(Debug)]
pub struct BlockingOpGuard<'a> {
    gate: &'a GaplessGate,
}

impl GaplessGate {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, WaitState> {
        self.state.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn generation(&self) -> u64 {
        self.lock().generation
    }

    /// Wakes any waiting streaming thread and stops new `about-to-finish`
    /// callbacks from blocking until the guard drops.
    pub fn enter_blocking_op(&self) -> BlockingOpGuard<'_> {
        self.begin_queued_cancel();
        BlockingOpGuard { gate: self }
    }

    /// Like [`Self::enter_blocking_op`], but held from when a stop or start is
    /// queued until [`Self::end_queued_cancel`] after it is handled, so a wait
    /// that begins in between cannot outlive it.
    pub fn begin_queued_cancel(&self) {
        let mut st = self.lock();
        st.blocking_ops += 1;
        st.generation = st.generation.wrapping_add(1);
        if st.waiting {
            st.done = true;
            st.waiting = false;
            self.cond.notify_all();
        }
    }

    pub fn end_queued_cancel(&self) {
        let mut st = self.lock();
        st.blocking_ops = st.blocking_ops.saturating_sub(1);
    }

    /// Mints a non-blocking token when `about-to-finish` fires on the main thread.
    pub fn begin_nonblocking(self: &Arc<Self>) -> Option<GaplessToken> {
        let mut st = self.lock();
        if st.blocking_ops > 0 {
            return None;
        }
        st.generation = st.generation.wrapping_add(1);
        let generation = st.generation;
        Some(GaplessToken {
            gate: Arc::clone(self),
            generation,
            finished: AtomicBool::new(false),
        })
    }

    /// Dispatches `dispatch` with a cancellation-aware token and blocks until
    /// the token finishes, a main-thread op cancels it, or `timeout` elapses.
    /// Returns `true` only when the wait timed out.
    pub fn wait(self: &Arc<Self>, timeout: Duration, dispatch: impl FnOnce(GaplessToken)) -> bool {
        let generation = {
            let mut st = self.lock();
            if st.blocking_ops > 0 {
                return false;
            }
            if st.waiting {
                st.done = true;
                self.cond.notify_all();
            }
            st.generation = st.generation.wrapping_add(1);
            st.waiting = true;
            st.done = false;
            st.generation
        };
        dispatch(GaplessToken {
            gate: Arc::clone(self),
            generation,
            finished: AtomicBool::new(false),
        });
        let deadline = Instant::now() + timeout;
        let mut st = self.lock();
        while !st.done && st.generation == generation {
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            let (next, res) = self
                .cond
                .wait_timeout(st, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            st = next;
            if res.timed_out() && !st.done && st.generation == generation {
                break;
            }
        }
        let timed_out = !st.done && st.generation == generation;
        if timed_out {
            st.generation = st.generation.wrapping_add(1);
            st.waiting = false;
        } else if st.generation == generation {
            st.waiting = false;
        }
        timed_out
    }
}

impl Drop for BlockingOpGuard<'_> {
    fn drop(&mut self) {
        self.gate.end_queued_cancel();
    }
}

impl GaplessToken {
    pub fn is_cancelled(&self) -> bool {
        let st = self.gate.lock();
        st.generation != self.generation || st.blocking_ops > 0
    }

    pub fn finish(&self) {
        if self.finished.swap(true, Ordering::AcqRel) {
            return;
        }
        let mut st = self.gate.lock();
        if st.generation == self.generation && st.waiting {
            st.done = true;
            st.waiting = false;
            self.gate.cond.notify_all();
        }
    }
}

impl Drop for GaplessToken {
    fn drop(&mut self) {
        self.finish();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GaplessSource {
    RepeatTrack,
    UserQueue,
    BaseQueue { rewound: bool },
}

#[derive(Debug, Clone)]
pub struct GaplessEntry {
    pub track: Track,
    pub did_repeat: bool,
    pub source: GaplessSource,
}

#[derive(Debug, Clone)]
enum GaplessSlot {
    Preparing(GaplessEntry),
    Armed { entry: GaplessEntry, manifest: TrackManifest },
}

impl GaplessSlot {
    fn entry(&self) -> &GaplessEntry {
        match self {
            Self::Preparing(entry) | Self::Armed { entry, .. } => entry,
        }
    }

    fn entry_mut(&mut self) -> &mut GaplessEntry {
        match self {
            Self::Preparing(entry) | Self::Armed { entry, .. } => entry,
        }
    }
}

// ---------------------------------------------------------------- playbin

/// The subset of playbin used by the state machine (mockable in tests).
pub trait Playbin {
    fn set_state(&self, state: gst::State) -> Result<()>;
    fn query_duration(&self) -> Option<Duration>;
    fn query_position(&self) -> Option<Duration>;
    fn seek(&self, position: Duration, flags: gst::SeekFlags) -> bool;
    fn set_uri(&self, uri: &str);
    fn set_volume(&self, volume: f64) -> Result<()>;
    /// Drops the messages still waiting on the bus.
    fn flush_bus(&self);
}

impl Playbin for gst::Element {
    fn set_state(&self, state: gst::State) -> Result<()> {
        ElementExt::set_state(self, state).map(|_| ()).map_err(|e| anyhow!("failed to set playbin state {state:?}: {e}"))
    }
    fn query_duration(&self) -> Option<Duration> {
        <Self as ElementExtManual>::query_duration::<gst::ClockTime>(self).map(|t| Duration::from_nanos(t.nseconds()))
    }
    fn query_position(&self) -> Option<Duration> {
        <Self as ElementExtManual>::query_position::<gst::ClockTime>(self).map(|t| Duration::from_nanos(t.nseconds()))
    }
    fn seek(&self, position: Duration, flags: gst::SeekFlags) -> bool {
        self.seek_simple(flags, gst::ClockTime::from_nseconds(position.as_nanos() as u64)).is_ok()
    }
    fn set_uri(&self, uri: &str) {
        self.set_property("uri", uri);
    }
    fn set_volume(&self, volume: f64) -> Result<()> {
        self.set_property("volume", volume);
        Ok(())
    }
    fn flush_bus(&self) {
        if let Some(bus) = self.bus() {
            bus.set_flushing(true);
            bus.set_flushing(false);
        }
    }
}

// ---------------------------------------------------------------- worker

thread_local! {
    static WORKER: RefCell<Option<glib::SourceId>> = const { RefCell::new(None) };
}

fn start_worker(playbin: Rc<dyn Playbin>) {
    stop_worker();
    #[cfg(not(test))]
    {
        let id = glib::timeout_add_local(Duration::from_millis(250), move || {
            playback_state_changed().notify(|mut s| {
                if let Some(p) = playbin.query_position() {
                    s.position = p;
                }
                s
            });
            glib::ControlFlow::Continue
        });
        WORKER.with(|w| *w.borrow_mut() = Some(id));
    }
    #[cfg(test)]
    drop(playbin);
}

pub fn stop_worker() {
    WORKER.with(|w| {
        if let Some(id) = w.borrow_mut().take() {
            id.remove();
        }
    });
}

// ---------------------------------------------------------------- state actions

pub fn on_stopped(playbin: &dyn Playbin) -> Result<()> {
    playbin.set_state(gst::State::Null)?;
    playback_state_changed().notify(|mut s| {
        s.status = PlaybackStatus::Stopped;
        s.loading = false;
        s.buffering = false;
        s
    });
    Ok(())
}

pub fn on_stopped_stop() {
    track_changed().set(None);
    playback_state_changed().notify(|mut s| {
        s.duration = Duration::ZERO;
        s.position = Duration::ZERO;
        s
    });
}

pub fn on_paused(playbin: &dyn Playbin) -> Result<()> {
    playbin.set_state(gst::State::Paused)?;
    playback_state_changed().notify(|mut s| {
        s.status = PlaybackStatus::Paused;
        s
    });
    Ok(())
}

/// A track that finished preparing while a pause was pending: paused and
/// loaded, published as one state change.
pub fn on_paused_started(playbin: &dyn Playbin) -> Result<()> {
    playbin.set_state(gst::State::Paused)?;
    playback_state_changed().notify(|mut s| {
        s.status = PlaybackStatus::Paused;
        s.loading = false;
        s
    });
    Ok(())
}

pub fn on_preparing() {
    playback_state_changed().notify(|mut s| {
        s.loading = true;
        s.buffering = false;
        // Otherwise the old track's position is published (MPRIS, seek bar)
        // until the new stream plays.
        s.position = Duration::ZERO;
        s
    });
}

/// `buffering`: the stream is still filling its buffer; the pipeline stays
/// paused until it is full.
pub fn on_playing(playbin: Rc<dyn Playbin>, buffering: bool) -> Result<()> {
    playbin.set_state(if buffering { gst::State::Paused } else { gst::State::Playing })?;
    playback_state_changed().notify(|mut s| {
        if let Some(d) = playbin.query_duration() {
            s.duration = d;
        }
        s.loading = false;
        s.buffering = buffering;
        s.status = PlaybackStatus::Playing;
        s
    });
    start_worker(playbin);
    Ok(())
}

pub fn on_playing_exit() {
    stop_worker();
}

pub fn on_playing_gapless_exit(track: Track, manifest: &TrackManifest, did_repeat: bool) {
    playback_state_changed().notify(|mut s| {
        s.duration = track.duration();
        s.position = Duration::ZERO;
        s
    });
    if did_repeat {
        track_repeated().notify(&track);
        return;
    }
    track_changed().set(Some(TrackChangedEvent { track, skip_history: false }));
    playback_quality_changed().set(manifest.quality);
}

pub fn on_playing_retry_exit(playbin: &dyn Playbin, position: Duration) {
    if playbin.seek(position, gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE) {
        playback_state_changed().notify(|mut s| {
            s.position = position;
            s
        });
    }
}

async fn fetch_track_manifest(track_id: &str) -> Result<TrackManifest> {
    let id = track_id.to_string();
    api::run(move || service::get_track_manifest(&id))
        .await
        .map_err(|e| anyhow!("failed to fetch and enqueue track: {e}"))
}

fn cancels_gapless(t: &Trigger) -> bool {
    matches!(
        t,
        Trigger::Stop | Trigger::PlayTrack(..) | Trigger::PlayTrackId(..) | Trigger::Error(_) | Trigger::TokenExpired
    )
}

fn enqueue_playbin(playbin: &dyn Playbin, manifest: &TrackManifest, volume: f64) -> Result<()> {
    playbin.set_uri(&manifest.url);
    playbin.set_volume(volume)
}

async fn fetch_and_enqueue_track(playbin: &dyn Playbin, track_id: &str) -> Result<TrackManifest> {
    let manifest = fetch_track_manifest(track_id).await?;
    enqueue_playbin(playbin, &manifest, settings::volume())?;
    super::replaygain::set_current(Some(&manifest));
    Ok(manifest)
}

thread_local! {
    /// A track whose next start loads paused (a handoff seeks it first).
    static START_PAUSED: RefCell<Option<(String, u64)>> = const { RefCell::new(None) };
    static START_PAUSED_TOKEN: Cell<u64> = const { Cell::new(0) };
}

/// The next start loads paused instead of playing, if it is `track_id`.
/// Returns the request's token for [`drop_start_paused`].
pub fn start_paused(track_id: &str) -> u64 {
    let token = START_PAUSED_TOKEN.with(|t| {
        t.set(t.get() + 1);
        t.get()
    });
    START_PAUSED.with(|s| *s.borrow_mut() = Some((track_id.to_string(), token)));
    token
}

/// Drops the request `token` if no start took it; a newer request stays.
pub fn drop_start_paused(token: u64) {
    START_PAUSED.with(|s| drop(s.borrow_mut().take_if(|(_, t)| *t == token)));
}

pub async fn on_play_track(playbin: &dyn Playbin, track: Track, skip_history: bool) -> Result<()> {
    // Only the start it was asked for takes it, before its fetch, so a failed
    // fetch leaves nothing behind. A start that never comes is cleared by `play_at`.
    let paused = START_PAUSED.with(|s| s.borrow_mut().take_if(|(id, _)| id == track.id()).is_some());
    let manifest = fetch_and_enqueue_track(playbin, track.id()).await?;
    playbin.set_state(if paused { gst::State::Paused } else { gst::State::Playing })?;
    let duration = track.duration();
    track_changed().set(Some(TrackChangedEvent { track, skip_history }));
    playback_state_changed().notify(|mut s| {
        s.duration = duration;
        s
    });
    playback_quality_changed().set(manifest.quality);
    Ok(())
}

// ---------------------------------------------------------------- machine

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Stopped,
    Paused,
    Preparing,
    Playing,
    PlayingGapless,
    PlayingRetrying,
}

impl State {
    fn is_playing(self) -> bool {
        matches!(self, State::Playing | State::PlayingGapless | State::PlayingRetrying)
    }
}

#[expect(clippy::large_enum_variant, reason = "triggers are short-lived and queued rarely; boxing Track adds noise")]
pub enum Trigger {
    #[expect(dead_code, reason = "TRIGGER_PLAY_TRACK_ID is part of the Go state machine; no caller fires it yet")]
    PlayTrackId(String, bool),
    PlayTrack(Track, bool),
    /// Runs a queue action (next / previous) once the triggers queued before
    /// it were handled, so it sees the track their start made current.
    Deferred(Box<dyn FnOnce()>),
    /// Reports a playback error (track id, message) once the deferred
    /// next / previous queued before it turned out to start nothing.
    ReportError(Option<String>, String),
    Pause,
    Resume,
    Stop,
    /// Sent from playbin's streaming thread; `done` is signalled once the
    /// next URI has been enqueued (or enqueuing failed or was canceled).
    GaplessHint(Option<GaplessToken>),
    TokenExpired,
    #[cfg_attr(not(test), expect(dead_code, reason = "kept in the transition table for parity with Go; gapless failure resets inline"))]
    Failed,
    /// A playback error other than an expired token, with its message.
    Error(String),
    /// The stream started (`true`) or finished (`false`) buffering.
    Buffering(bool),
    StreamStart,
    Eos,
}

impl std::fmt::Debug for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Trigger::PlayTrackId(..) => "TRIGGER_PLAY_TRACK_ID",
            Trigger::PlayTrack(..) => "TRIGGER_PLAY_TRACK",
            Trigger::Deferred(_) => "deferred",
            Trigger::ReportError(..) => "report-error",
            Trigger::Pause => "TRIGGER_PAUSE",
            Trigger::Resume => "TRIGGER_RESUME",
            Trigger::Stop => "TRIGGER_STOP",
            Trigger::GaplessHint(_) => "TRIGGER_GAPLESS_HINT",
            Trigger::TokenExpired => "TRIGGER_TOKEN_EXPIRED",
            Trigger::Failed => "TRIGGER_FAILED",
            Trigger::Error(_) => "error",
            Trigger::Buffering(_) => "buffering",
            Trigger::StreamStart => "stream-start",
            Trigger::Eos => "eos",
        };
        f.write_str(s)
    }
}

/// Hooks into the player module (queues, `ReplayGain`, autoplay) that the machine needs.
pub struct Hooks {
    pub peek_gapless: fn() -> Option<GaplessEntry>,
    pub on_gapless_arm: fn(&TrackManifest),
    pub on_gapless_promote: fn(&GaplessEntry),
    pub on_gapless_cancel: fn(),
    pub on_eos_stopped: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>,
    pub clear_source: fn(),
    /// Playback of the track (its id) stopped on an error.
    pub on_error: fn(Option<&str>, &str),
}

#[cfg(test)]
impl Default for Hooks {
    fn default() -> Self {
        Self {
            peek_gapless: || None,
            on_gapless_arm: |_| {},
            on_gapless_promote: |_| {},
            on_gapless_cancel: || {},
            on_eos_stopped: || Box::pin(async {}),
            clear_source: || {},
            on_error: |_, _| {},
        }
    }
}

pub struct Machine {
    state: Cell<State>,
    queue: RefCell<VecDeque<Trigger>>,
    running: Cell<bool>,
    /// Whether the trigger being handled right now starts a track.
    handling_start: Cell<bool>,
    playbin: Rc<dyn Playbin>,
    gate: Arc<GaplessGate>,
    gapless: RefCell<Option<GaplessSlot>>,
    retry_position: Cell<Option<Duration>>,
    /// A pause arrived while the track was still preparing: the stream
    /// starts paused instead of playing (Listen Together aligned start).
    pause_on_start: Cell<bool>,
    /// The stream ran dry and holds the pipeline paused until its buffer is
    /// full again.
    buffering: Cell<bool>,
    /// The token expired while paused: the stream was torn down and is
    /// fetched again on resume.
    reload_on_resume: Cell<bool>,
    hooks: Hooks,
}

impl Machine {
    #[cfg(test)]
    pub fn new(playbin: Rc<dyn Playbin>, hooks: Hooks) -> Rc<Self> {
        Self::with_gate(playbin, Arc::new(GaplessGate::new()), hooks)
    }

    pub fn with_gate(playbin: Rc<dyn Playbin>, gate: Arc<GaplessGate>, hooks: Hooks) -> Rc<Self> {
        Rc::new(Machine {
            state: Cell::new(State::Stopped),
            queue: RefCell::new(VecDeque::new()),
            running: Cell::new(false),
            handling_start: Cell::new(false),
            playbin,
            gate,
            gapless: RefCell::new(None),
            retry_position: Cell::new(None),
            pause_on_start: Cell::new(false),
            buffering: Cell::new(false),
            reload_on_resume: Cell::new(false),
            hooks,
        })
    }

    fn has_armed_gapless(&self) -> bool {
        matches!(&*self.gapless.borrow(), Some(GaplessSlot::Armed { .. }))
    }

    /// Currently preparing or armed gapless handoff entry, if any.
    pub fn gapless_entry(&self) -> Option<GaplessEntry> {
        self.gapless.borrow().as_ref().map(|slot| slot.entry().clone())
    }

    /// Updates the pending gapless entry when Play Next moves it from the base
    /// queue to the head of the user queue.
    pub fn move_gapless_to_user_queue(&self) {
        if let Some(slot) = self.gapless.borrow_mut().as_mut() {
            slot.entry_mut().source = GaplessSource::UserQueue;
        }
    }

    fn begin_gapless(&self, entry: GaplessEntry) {
        self.cancel_gapless();
        *self.gapless.borrow_mut() = Some(GaplessSlot::Preparing(entry));
    }

    fn arm_gapless(&self, manifest: TrackManifest, volume: f64) -> Result<bool> {
        if !matches!(&*self.gapless.borrow(), Some(GaplessSlot::Preparing(_))) {
            return Ok(false);
        }
        (self.hooks.on_gapless_arm)(&manifest);
        if let Err(e) = enqueue_playbin(&*self.playbin, &manifest, volume) {
            self.cancel_gapless();
            return Err(e);
        }
        let mut slot = self.gapless.borrow_mut();
        let Some(GaplessSlot::Preparing(entry)) = slot.take() else {
            drop(slot);
            (self.hooks.on_gapless_cancel)();
            return Ok(false);
        };
        *slot = Some(GaplessSlot::Armed { entry, manifest });
        Ok(true)
    }

    fn cancel_gapless(&self) {
        if self.gapless.borrow_mut().take().is_some() {
            (self.hooks.on_gapless_cancel)();
        }
    }

    fn promote_gapless(&self) {
        if let Some(GaplessSlot::Armed { entry, manifest }) = self.gapless.borrow_mut().take() {
            (self.hooks.on_gapless_promote)(&entry);
            on_playing_gapless_exit(entry.track, &manifest, entry.did_repeat);
        }
    }

    fn has_queued_stop_or_start(&self) -> bool {
        self.queue.borrow().iter().any(|t| matches!(t, Trigger::Stop | Trigger::PlayTrack(..) | Trigger::PlayTrackId(..)))
    }

    /// Whether a Stop is waiting behind the trigger being handled.
    pub fn has_queued_stop(&self) -> bool {
        self.queue.borrow().iter().any(|t| matches!(t, Trigger::Stop))
    }

    /// Whether a track start (or a deferred next / previous) is queued or
    /// being handled.
    pub fn start_pending(&self) -> bool {
        self.handling_start.get()
            || self.queue.borrow().iter().any(|t| matches!(t, Trigger::PlayTrack(..) | Trigger::PlayTrackId(..) | Trigger::Deferred(_)))
    }

    /// Whether the machine is stopped and nothing is about to start playback.
    pub fn stopped_without_pending_start(&self) -> bool {
        self.state.get() == State::Stopped && !self.start_pending()
    }

    /// Queues a trigger; transitions run in order on the main loop.
    pub fn fire(self: &Rc<Self>, t: Trigger) {
        if cancels_gapless(&t) {
            self.gate.begin_queued_cancel();
        }
        self.queue.borrow_mut().push_back(t);
        if self.running.replace(true) {
            return;
        }
        let this = Rc::clone(self);
        glib::spawn_future_local(async move {
            loop {
                let next = this.queue.borrow_mut().pop_front();
                let Some(t) = next else { break };
                this.handling_start.set(matches!(t, Trigger::PlayTrack(..) | Trigger::PlayTrackId(..)));
                let cancels = cancels_gapless(&t);
                this.handle(t).await;
                if cancels {
                    this.gate.end_queued_cancel();
                }
                this.handling_start.set(false);
            }
            this.running.set(false);
        });
    }

    #[expect(clippy::match_same_arms, reason = "one arm per (state, trigger) pair, mirroring the Go Permit table")]
    fn target(&self, t: &Trigger) -> Option<State> {
        use State::{Stopped, Paused, Playing, PlayingGapless, PlayingRetrying, Preparing};
        let s = self.state.get();
        match (s, t) {
            // From Preparing too: the newest start wins over one still loading.
            (_, Trigger::PlayTrackId(..) | Trigger::PlayTrack(..)) => Some(Preparing),
            (Paused, Trigger::Resume) if self.reload_on_resume.get() || self.retry_position.get().is_some() => Some(PlayingRetrying),
            (Paused, Trigger::Resume) if self.has_armed_gapless() => Some(PlayingGapless),
            (Paused, Trigger::Resume) => Some(Playing),
            (Paused | Preparing, Trigger::Stop) => Some(Stopped),
            (Preparing, Trigger::TokenExpired) => Some(Stopped),
            (Preparing, Trigger::StreamStart) => Some(Playing),
            (PlayingGapless | PlayingRetrying, Trigger::StreamStart) => Some(Playing),
            (PlayingGapless, Trigger::Failed) => Some(Playing),
            (PlayingRetrying, Trigger::TokenExpired) => Some(Stopped),
            (Paused | Preparing | Playing | PlayingGapless | PlayingRetrying, Trigger::Error(_)) => Some(Stopped),
            (_, _) if s.is_playing() => match t {
                Trigger::Pause => Some(Paused),
                Trigger::Stop | Trigger::Eos => Some(Stopped),
                Trigger::GaplessHint(tok) if s == Playing && !tok.as_ref().is_some_and(GaplessToken::is_cancelled) => Some(PlayingGapless),
                Trigger::TokenExpired => Some(PlayingRetrying),
                _ => None,
            },
            _ => None,
        }
    }

    async fn handle(self: &Rc<Self>, t: Trigger) {
        if let Trigger::Deferred(action) = t {
            // A Stop pressed after it wins; don't take a track for nothing.
            if self.has_queued_stop() {
                return;
            }
            // What the action fires belongs at its place in the queue, ahead
            // of a Pause pressed after it.
            let before = self.queue.borrow().len();
            action();
            let mut q = self.queue.borrow_mut();
            let fired = q.split_off(before);
            for t in fired.into_iter().rev() {
                q.push_front(t);
            }
            return;
        }
        let from = self.state.get();
        if let Trigger::Buffering(b) = t {
            self.on_buffering(b);
            return;
        }
        if let Trigger::ReportError(track, e) = t {
            if from == State::Stopped {
                self.report_error(track.as_deref(), &e);
            }
            return;
        }
        let Some(mut to) = self.target(&t) else {
            match t {
                Trigger::Pause if from == State::Preparing => {
                    self.pause_on_start.set(true);
                    if let Err(e) = self.playbin.set_state(gst::State::Paused) {
                        log::error!("failed to pause a preparing track: {e:#}");
                    }
                }
                Trigger::Resume if from == State::Preparing && self.pause_on_start.replace(false) => {
                    let state = if self.buffering.get() { gst::State::Paused } else { gst::State::Playing };
                    if let Err(e) = self.playbin.set_state(state) {
                        log::error!("failed to resume a preparing track: {e:#}");
                    }
                }
                // Ignore(): the hint is canceled, retrying, or already handled.
                Trigger::GaplessHint(done) => drop(done),
                Trigger::TokenExpired if from == State::Paused => {
                    log::warn!("playback token expired while paused, reloading on resume");
                    if self.retry_position.get().is_none() {
                        let pos = self.playbin.query_position().unwrap_or_else(|| playback_state_changed().get().position);
                        self.retry_position.set(Some(pos));
                    }
                    self.cancel_gapless();
                    // Tear down now so the stream error that follows the
                    // 401/404 does not stop playback.
                    if let Err(e) = self.playbin.set_state(gst::State::Ready) {
                        log::error!("failed to reset the pipeline after an expired token: {e:#}");
                    }
                    self.drop_stream_messages();
                    self.reload_on_resume.set(true);
                }
                Trigger::StreamStart if from == State::Paused && self.retry_position.get().is_some() => {
                    if let Some(pos) = self.retry_position.take() {
                        on_playing_retry_exit(&*self.playbin, pos);
                    }
                }
                Trigger::StreamStart if from == State::Paused && self.has_armed_gapless() => {
                    self.promote_gapless();
                }
                Trigger::Stop if from == State::Stopped => log::debug!("already stopped"),
                Trigger::Error(e) if from == State::Stopped => log::debug!("playback error while stopped: {e}"),
                Trigger::StreamStart if from == State::Playing => {}
                _ => log::error!("Failed Transition state={from:?} trigger={t:?}"),
            }
            return;
        };
        let needs_reload = from == State::Paused && self.reload_on_resume.get();
        // An accepted new start or stop drops a pause meant for the old one.
        if matches!(to, State::Preparing | State::Stopped | State::PlayingRetrying) {
            self.reload_on_resume.set(false);
            self.cancel_gapless();
        }
        // The deferred next / previous started a track: the failure it
        // waited on is superseded.
        if to == State::Preparing {
            self.queue.borrow_mut().retain(|t| !matches!(t, Trigger::ReportError(..)));
        }
        if matches!(to, State::Preparing | State::Stopped) {
            self.pause_on_start.set(false);
            self.buffering.set(false);
            self.retry_position.set(None);
        }
        let started_paused = from == State::Preparing && to == State::Playing && self.pause_on_start.replace(false);
        if started_paused {
            to = State::Paused;
        }
        log::debug!("Transitioning source={from:?} destination={to:?} trigger={t:?}");

        // Exit actions.
        let sub_to_parent = matches!(from, State::PlayingGapless | State::PlayingRetrying) && to == State::Playing;
        let into_sub = from == State::Playing && matches!(to, State::PlayingGapless | State::PlayingRetrying);
        if from == State::PlayingGapless && matches!(t, Trigger::StreamStart) {
            self.promote_gapless();
        } else if from == State::PlayingGapless && matches!(t, Trigger::Failed) {
            self.cancel_gapless();
        }
        if from == State::PlayingRetrying && matches!(t, Trigger::StreamStart)
            && let Some(pos) = self.retry_position.take()
        {
            on_playing_retry_exit(&*self.playbin, pos);
        }
        if from.is_playing() && !to.is_playing() {
            on_playing_exit();
        }
        self.state.set(to);

        // Entry actions.
        let res: Result<()> = match (to, t) {
            (State::Stopped, t) => {
                let r = on_stopped(&*self.playbin);
                self.drop_stream_messages();
                match t {
                    Trigger::Stop => {
                        // `player::stop` clears the source synchronously. The
                        // trigger runs later on the main loop, after a play
                        // call has already set the new source, so clearing
                        // here would wipe "Playing From" for every new queue.
                        on_stopped_stop();
                    }
                    Trigger::TokenExpired => {
                        log::error!(
                            "track was enqueued successfully, but playback was rejected by remote server. this may be due to a network issue, account restrictions or a temporary outage."
                        );
                        (self.hooks.clear_source)();
                        on_stopped_stop();
                    }
                    Trigger::Eos => (self.hooks.on_eos_stopped)().await,
                    Trigger::Error(e) => {
                        let track = track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string()));
                        self.stopped_on_error(track, e);
                    }
                    _ => {}
                }
                r
            }
            (State::Paused, _) if started_paused => on_paused_started(&*self.playbin),
            (State::Paused, _) => on_paused(&*self.playbin),
            (State::Preparing, t) => {
                on_preparing();
                let id = match &t {
                    Trigger::PlayTrackId(id, _) => Some(id.clone()),
                    Trigger::PlayTrack(track, _) => Some(track.id().to_string()),
                    _ => None,
                };
                let r = match self.playbin.set_state(gst::State::Ready).map(|()| self.drop_stream_messages()) {
                    Err(e) => Err(e),
                    Ok(()) => match t {
                    Trigger::PlayTrackId(id, skip) => {
                        let id2 = id.clone();
                        match api::run(move || service::get_track(&id2)).await {
                            Ok(track) => on_play_track(&*self.playbin, track, skip).await,
                            Err(e) => Err(e),
                        }
                    }
                    Trigger::PlayTrack(track, skip) => on_play_track(&*self.playbin, track, skip).await,
                    _ => Ok(()),
                    },
                };
                if let Err(e) = &r {
                    // Deviation from Go: don't stay stuck in PREPARING with a spinner.
                    log::error!("failed to start playback: {e:#}");
                    self.state.set(State::Stopped);
                    if let Err(e) = on_stopped(&*self.playbin) {
                        log::error!("failed to reset the pipeline after a playback error: {e:#}");
                    }
                    self.stopped_on_error(id, e.to_string());
                }
                r
            }
            (State::Playing, _) if sub_to_parent => Ok(()),
            (State::Playing, _) | (State::PlayingGapless, Trigger::Resume) => {
                on_playing(Rc::clone(&self.playbin), self.buffering.get())
            }
            (State::PlayingGapless, Trigger::GaplessHint(done)) => {
                let _ = into_sub;
                let gate_gen = self.gate.generation();
                let cancelled = |tok: &Option<GaplessToken>| {
                    tok.as_ref().is_some_and(GaplessToken::is_cancelled)
                        || self.gate.generation() != gate_gen
                        || self.has_queued_stop_or_start()
                };
                let r: Result<bool> = async {
                    if cancelled(&done) {
                        return Ok(false);
                    }
                    let Some(entry) = (self.hooks.peek_gapless)() else {
                        return Ok(false);
                    };
                    let track_id = entry.track.id().to_string();
                    self.begin_gapless(entry);
                    let manifest = fetch_track_manifest(&track_id).await?;
                    if cancelled(&done) {
                        return Ok(false);
                    }
                    self.arm_gapless(manifest, settings::volume())
                }
                .await;
                if let Err(e) = &r {
                    log::error!("failed to enqueue next track: {e:#}");
                }
                if !matches!(r, Ok(true)) {
                    self.cancel_gapless();
                    self.state.set(State::Playing);
                }
                drop(done);
                Ok(())
            }
            (State::PlayingRetrying, _) => {
                let r = if from == State::Paused && !needs_reload {
                    // Resumed while an earlier retry was already loading: wait for
                    // its StreamStart instead of tearing down and fetching again.
                    on_playing(Rc::clone(&self.playbin), self.buffering.get())
                } else {
                    let mut r = self.retry().await;
                    // Resumed after the token expired while paused: also publish
                    // the playing state that a retry from Playing already has.
                    if r.is_ok() && from == State::Paused {
                        r = on_playing(Rc::clone(&self.playbin), self.buffering.get());
                    }
                    r
                };
                if let Err(e) = &r {
                    // Stuck in PlayingRetrying, Play would do nothing.
                    log::error!("failed to reload the stream: {e:#}");
                    let track = track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string()));
                    self.retry_position.set(None);
                    self.state.set(State::Stopped);
                    on_playing_exit();
                    if let Err(e) = on_stopped(&*self.playbin) {
                        log::error!("failed to reset the pipeline after a playback error: {e:#}");
                    }
                    self.stopped_on_error(track, e.to_string());
                }
                r
            }
            _ => Ok(()),
        };
        if let Err(e) = res {
            log::error!("state machine action failed: {e:#}");
        }
        log::debug!("Transition completed destination={:?}", self.state.get());
    }

    /// Holds the pipeline paused while the stream buffers, so the position
    /// does not run on over silence, and resumes it once the buffer is full.
    fn on_buffering(&self, buffering: bool) {
        let state = self.state.get();
        if state == State::Stopped {
            return;
        }
        // The gapless next track buffers while the current one still plays.
        if state == State::PlayingGapless && buffering {
            return;
        }
        if self.buffering.replace(buffering) == buffering {
            return;
        }
        log::debug!("buffering={buffering} state={state:?}");
        // Only playback that should be audible is held and resumed; a user
        // pause (or a paused start) keeps the pipeline paused.
        let held = match state {
            State::Playing | State::PlayingGapless | State::PlayingRetrying => true,
            State::Preparing => !self.pause_on_start.get(),
            State::Paused | State::Stopped => false,
        };
        if held && let Err(e) = self.playbin.set_state(if buffering { gst::State::Paused } else { gst::State::Playing }) {
            log::error!("failed to change the pipeline state for buffering: {e:#}");
        }
        // Published in every state: a Listen Together guest loads paused and
        // waits for the buffer before its aligned start.
        playback_state_changed().notify(|mut s| {
            s.buffering = buffering;
            s
        });
    }

    /// Clears a track that stopped on an error and reports it, unless a
    /// queued stop or start already replaced its source. A queued next /
    /// previous may replace it, so then the report waits until it has run.
    fn stopped_on_error(&self, track: Option<String>, e: String) {
        on_stopped_stop();
        let mut queue = self.queue.borrow_mut();
        let replaced = queue.iter().any(|t| matches!(t, Trigger::Stop | Trigger::PlayTrack(..) | Trigger::PlayTrackId(..)));
        match queue.iter().rposition(|t| matches!(t, Trigger::Deferred(_))) {
            _ if replaced => {}
            Some(i) => queue.insert(i + 1, Trigger::ReportError(track, e)),
            None => {
                drop(queue);
                self.report_error(track.as_deref(), &e);
            }
        }
    }

    fn report_error(&self, track: Option<&str>, e: &str) {
        (self.hooks.clear_source)();
        (self.hooks.on_error)(track, e);
    }

    /// Called once the old stream is torn down: what it posted (a follow-up
    /// error after an expired token, a late stream start or buffering) must
    /// not act on the next stream.
    fn drop_stream_messages(&self) {
        self.playbin.flush_bus();
        self.queue.borrow_mut().retain(|t| {
            !matches!(
                t,
                Trigger::Error(_)
                    | Trigger::Buffering(_)
                    | Trigger::TokenExpired
                    | Trigger::StreamStart
                    | Trigger::Eos
                    | Trigger::GaplessHint(_)
            )
        });
    }

    async fn retry(&self) -> Result<()> {
        let pb = &*self.playbin;
        if self.buffering.replace(false) {
            playback_state_changed().notify(|mut s| {
                s.buffering = false;
                s
            });
        }
        // Capture live position before State::Ready resets the pipeline clock.
        let pos = self
            .retry_position
            .get()
            .or_else(|| pb.query_position())
            .unwrap_or_else(|| playback_state_changed().get().position);
        self.retry_position.set(Some(pos));
        pb.set_state(gst::State::Ready)?;
        self.drop_stream_messages();
        let id = track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string())).unwrap_or_default();
        fetch_and_enqueue_track(pb, &id).await?;
        pb.set_state(gst::State::Playing)
    }
}

// ---------------------------------------------------------------- tests

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::signals::UNSUBSCRIBE;

    #[derive(Default)]
    #[expect(clippy::struct_excessive_bools, reason = "test double with one failure switch per method")]
    pub struct Testbin {
        pub duration: Cell<Duration>,
        pub duration_should_error: bool,
        pub position: Cell<Duration>,
        pub position_should_error: bool,
        pub reset_position_on_ready: bool,
        pub volume: Cell<f64>,
        pub volume_should_error: bool,
        pub state: Cell<Option<gst::State>>,
        pub state_should_error: bool,
        pub uri: RefCell<String>,
        pub seeks: RefCell<Vec<Duration>>,
    }

    impl Playbin for Testbin {
        fn set_state(&self, state: gst::State) -> Result<()> {
            if self.state_should_error {
                anyhow::bail!("set state failed");
            }
            if self.reset_position_on_ready && matches!(state, gst::State::Ready | gst::State::Null) {
                self.position.set(Duration::ZERO);
            }
            self.state.set(Some(state));
            Ok(())
        }
        fn query_duration(&self) -> Option<Duration> {
            (!self.duration_should_error).then(|| self.duration.get())
        }
        fn query_position(&self) -> Option<Duration> {
            if self.position_should_error {
                return None;
            }
            if self.reset_position_on_ready && matches!(self.state.get(), Some(gst::State::Ready | gst::State::Null)) {
                return None;
            }
            Some(self.position.get())
        }
        fn seek(&self, position: Duration, _: gst::SeekFlags) -> bool {
            self.seeks.borrow_mut().push(position);
            self.position.set(position);
            !self.position_should_error
        }
        fn set_uri(&self, uri: &str) {
            *self.uri.borrow_mut() = uri.into();
        }
        fn flush_bus(&self) {}
        fn set_volume(&self, volume: f64) -> Result<()> {
            if self.volume_should_error {
                anyhow::bail!("set volume failed");
            }
            self.volume.set(volume);
            Ok(())
        }
    }

    fn capture_next_state() -> Rc<RefCell<Option<PlaybackState>>> {
        let seen = Rc::new(RefCell::new(None));
        let s = Rc::clone(&seen);
        playback_state_changed().on_lazy(move |st| {
            *s.borrow_mut() = Some(*st);
            UNSUBSCRIBE
        });
        seen
    }

    fn secs(s: u64) -> Duration {
        Duration::from_secs(s)
    }

    #[test]
    fn on_paused_sets_playbin_state() {
        let pb = Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() };
        on_paused(&pb).unwrap();
        assert_eq!(pb.state.get(), Some(gst::State::Paused));
    }

    #[test]
    fn on_paused_updates_playback_state() {
        playback_state_changed().set(PlaybackState { status: PlaybackStatus::Playing, ..Default::default() });
        let seen = capture_next_state();
        on_paused(&Testbin::default()).unwrap();
        assert_eq!(seen.borrow().expect("not notified").status, PlaybackStatus::Paused);
    }

    #[test]
    fn on_playing_sets_playbin_state() {
        let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Paused)), ..Default::default() });
        on_playing(Rc::clone(&pb) as Rc<dyn Playbin>, false).unwrap();
        assert_eq!(pb.state.get(), Some(gst::State::Playing));
        on_playing_exit();
    }

    #[test]
    fn on_playing_emits_playback_state() {
        let expected = secs(192);
        let pb = Rc::new(Testbin { duration: Cell::new(expected), ..Default::default() });
        let initial = PlaybackState { duration: secs(30), position: secs(12), status: PlaybackStatus::Paused, loading: true, buffering: false };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        on_playing(pb, false).unwrap();
        on_playing_exit();
        let want = PlaybackState { duration: expected, status: PlaybackStatus::Playing, loading: false, ..initial };
        assert_eq!(seen.borrow().expect("not notified"), want);
    }

    #[test]
    fn on_playing_returns_state_error_without_updating_playback_state() {
        let initial = PlaybackState { duration: secs(60), position: secs(15), status: PlaybackStatus::Paused, loading: true, buffering: false };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        assert!(on_playing(Rc::new(Testbin { state_should_error: true, ..Default::default() }), false).is_err());
        assert!(seen.borrow().is_none());
        assert_eq!(playback_state_changed().get(), initial);
        stop_worker();
    }

    #[test]
    fn on_playing_preserves_duration_when_query_fails() {
        let initial = PlaybackState { duration: secs(120), ..Default::default() };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        on_playing(Rc::new(Testbin { duration: Cell::new(secs(300)), duration_should_error: true, ..Default::default() }), false).unwrap();
        on_playing_exit();
        assert_eq!(seen.borrow().expect("not notified").duration, initial.duration);
    }

    #[test]
    fn on_stopped_sets_playbin_state() {
        let pb = Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() };
        on_stopped(&pb).unwrap();
        assert_eq!(pb.state.get(), Some(gst::State::Null));
    }

    #[test]
    fn on_stopped_emits_playback_state() {
        let initial = PlaybackState { duration: secs(180), position: secs(45), status: PlaybackStatus::Playing, loading: true, buffering: false };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        on_stopped(&Testbin::default()).unwrap();
        let want = PlaybackState { status: PlaybackStatus::Stopped, loading: false, ..initial };
        assert_eq!(seen.borrow().expect("not notified"), want);
    }

    #[test]
    fn on_stopped_returns_state_error_without_updating_playback_state() {
        let initial = PlaybackState { duration: secs(60), position: secs(15), status: PlaybackStatus::Playing, loading: true, buffering: false };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        assert!(on_stopped(&Testbin { state_should_error: true, ..Default::default() }).is_err());
        assert!(seen.borrow().is_none());
        assert_eq!(playback_state_changed().get(), initial);
    }

    #[test]
    fn on_stopped_stop_clears_current_track() {
        track_changed().set(Some(TrackChangedEvent { track: Track::default(), skip_history: false }));
        let seen = Rc::new(Cell::new(None::<bool>));
        let s = Rc::clone(&seen);
        track_changed().on_lazy(move |t| {
            s.set(Some(t.is_none()));
            UNSUBSCRIBE
        });
        on_stopped_stop();
        assert_eq!(seen.get(), Some(true), "TrackChanged not notified with None");
    }

    #[test]
    fn gapless_exit_respects_repeat() {
        let mut t = Track::default();
        t.info.duration = secs(200);
        track_changed().set(None);
        playback_state_changed().set(PlaybackState { position: secs(190), ..Default::default() });
        let m = TrackManifest { quality: TrackQuality::Lossless, ..Default::default() };
        on_playing_gapless_exit(t.clone(), &m, true);
        assert!(track_changed().get().is_none());
        assert_eq!(playback_state_changed().get().position, Duration::ZERO);
        on_playing_gapless_exit(t, &m, false);
        assert!(track_changed().get().is_some());
        assert_eq!(playback_quality_changed().get(), TrackQuality::Lossless);
    }

    #[test]
    fn transition_table_matches_go() {
        use State::*;
        let m = Machine::new(Rc::new(Testbin::default()), Hooks::default());
        let play = || Trigger::PlayTrack(Track::default(), false);
        let cases: Vec<(State, Trigger, Option<State>)> = vec![
            (Stopped, play(), Some(Preparing)),
            (Stopped, Trigger::Resume, None),
            (Stopped, Trigger::Pause, None),
            (Paused, Trigger::Resume, Some(Playing)),
            (Paused, Trigger::Stop, Some(Stopped)),
            (Paused, play(), Some(Preparing)),
            (Preparing, Trigger::StreamStart, Some(Playing)),
            (Preparing, Trigger::TokenExpired, Some(Stopped)),
            (Preparing, play(), Some(Preparing)),
            (Playing, Trigger::Pause, Some(Paused)),
            (Playing, Trigger::Eos, Some(Stopped)),
            (Playing, Trigger::GaplessHint(None), Some(PlayingGapless)),
            (Playing, Trigger::TokenExpired, Some(PlayingRetrying)),
            (Playing, Trigger::StreamStart, None),
            (PlayingGapless, Trigger::StreamStart, Some(Playing)),
            (PlayingGapless, Trigger::Failed, Some(Playing)),
            (PlayingGapless, Trigger::Pause, Some(Paused)),
            (PlayingGapless, Trigger::GaplessHint(None), None),
            (PlayingRetrying, Trigger::GaplessHint(None), None),
            (PlayingRetrying, Trigger::StreamStart, Some(Playing)),
            (PlayingRetrying, Trigger::Stop, Some(Stopped)),
            (Stopped, Trigger::Error(String::new()), None),
            (Paused, Trigger::Error(String::new()), Some(Stopped)),
            (Preparing, Trigger::Error(String::new()), Some(Stopped)),
            (Playing, Trigger::Error(String::new()), Some(Stopped)),
            (PlayingGapless, Trigger::Error(String::new()), Some(Stopped)),
            (PlayingRetrying, Trigger::Error(String::new()), Some(Stopped)),
        ];
        for (from, t, want) in cases {
            m.state.set(from);
            assert_eq!(m.target(&t), want, "{from:?} + {t:?}");
        }
    }

    #[test]
    fn eos_from_playing_targets_stopped_and_is_ignored_when_stopped() {
        let m = Machine::new(Rc::new(Testbin::default()), Hooks::default());
        assert_eq!(m.target(&Trigger::Eos), None);
        for s in [State::Playing, State::PlayingGapless, State::PlayingRetrying] {
            m.state.set(s);
            assert_eq!(m.target(&Trigger::Eos), Some(State::Stopped), "{s:?}");
        }
    }

    thread_local! {
        static CLEARED: Cell<usize> = const { Cell::new(0) };
    }

    /// Regression: starting a new queue (another row of the same list, another
    /// list, an album or a playlist) goes through `player::stop()` (clears the
    /// source synchronously, queues `Stop`) and then sets the new source. The
    /// queued `Stop` runs later and must not clear the new source, whatever
    /// state the previous playback was in.
    #[test]
    fn queued_stop_does_not_clear_the_next_source() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            for from in [State::Stopped, State::Playing, State::Paused, State::PlayingGapless, State::PlayingRetrying] {
                CLEARED.with(|c| c.set(0));
                let m = Machine::new(
                    Rc::new(Testbin::default()),
                    Hooks { clear_source: || CLEARED.with(|c| c.set(c.get() + 1)), ..Hooks::default() },
                );
                m.state.set(from);
                m.fire(Trigger::Stop);
                // ...the new source is set here, before the trigger runs...
                while ctx.iteration(false) {}
                assert_eq!(m.state.get(), State::Stopped, "{from:?}");
                assert_eq!(CLEARED.with(Cell::get), 0, "the queued Stop cleared the source (from {from:?})");
            }
        })
        .unwrap();
    }

    thread_local! {
        static DEFERRED_RAN: Cell<usize> = const { Cell::new(0) };
    }

    /// A deferred next / previous counts as a pending start (so nothing else
    /// starts the queue meanwhile) and runs once it is reached.
    #[test]
    fn deferred_action_is_a_pending_start_and_runs() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let m = Machine::new(Rc::new(Testbin::default()), Hooks::default());
            assert!(m.stopped_without_pending_start());
            m.queue.borrow_mut().push_back(Trigger::Deferred(Box::new(|| DEFERRED_RAN.with(|c| c.set(c.get() + 1)))));
            assert!(!m.stopped_without_pending_start());
            m.queue.borrow_mut().clear();
            m.fire(Trigger::Deferred(Box::new(|| DEFERRED_RAN.with(|c| c.set(c.get() + 1)))));
            while ctx.iteration(false) {}
            assert_eq!(DEFERRED_RAN.with(Cell::get), 1);
            assert_eq!(m.state.get(), State::Stopped);
            assert!(m.stopped_without_pending_start());
        })
        .unwrap();
    }

    thread_local! {
        static ORDER: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    }

    /// What a deferred action fires runs before triggers queued after it (a
    /// Pause pressed after a quick Next applies to its track); a later Stop
    /// cancels it.
    #[test]
    fn deferred_action_fires_ahead_of_later_triggers() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let m = Machine::new(Rc::new(Testbin::default()), Hooks::default());
            let inner = Rc::clone(&m);
            m.queue.borrow_mut().push_back(Trigger::Deferred(Box::new(move || {
                inner.fire(Trigger::Deferred(Box::new(|| ORDER.with(|o| o.borrow_mut().push("fired")))));
            })));
            m.queue.borrow_mut().push_back(Trigger::Deferred(Box::new(|| ORDER.with(|o| o.borrow_mut().push("later")))));
            m.fire(Trigger::Pause);
            while ctx.iteration(false) {}
            assert_eq!(ORDER.with(|o| o.borrow().clone()), ["fired", "later"]);
            // A Stop pressed after a deferred action cancels it.
            m.queue.borrow_mut().push_back(Trigger::Deferred(Box::new(|| ORDER.with(|o| o.borrow_mut().push("cancelled")))));
            m.fire(Trigger::Stop);
            while ctx.iteration(false) {}
            assert_eq!(ORDER.with(|o| o.borrow().clone()), ["fired", "later"]);
        })
        .unwrap();
    }

    /// A pause while the track is still preparing (Listen Together guest
    /// loading a host track paused) holds the pipeline and makes the stream
    /// start paused instead of playing from 0:00.
    #[test]
    fn pause_while_preparing_starts_paused() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = Machine::new(Rc::clone(&pb) as Rc<dyn Playbin>, Hooks::default());
            m.state.set(State::Preparing);
            m.fire(Trigger::Pause);
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Paused));
            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert_eq!(pb.state.get(), Some(gst::State::Paused));
            // Without a pending pause the stream starts playing as usual.
            m.state.set(State::Preparing);
            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Playing);
        })
        .unwrap();
    }

    fn test_machine(pb: &Rc<Testbin>) -> Rc<Machine> {
        Machine::new(
            Rc::clone(pb) as Rc<dyn Playbin>,
            Hooks {
                on_error: |_, e| ERRORS.with(|v| v.borrow_mut().push(e.to_string())),
                ..Hooks::default()
            },
        )
    }

    thread_local! {
        static ERRORS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    /// #77: a stream that runs dry holds the pipeline paused (so the position
    /// stops) and shows as loading until its buffer is full again.
    #[test]
    fn buffering_holds_playback() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin::default());
            let m = test_machine(&pb);
            m.state.set(State::Playing);
            playback_state_changed().set(PlaybackState { status: PlaybackStatus::Playing, ..Default::default() });
            m.fire(Trigger::Buffering(true));
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Paused));
            assert!(playback_state_changed().get().buffering);
            assert!(!playback_state_changed().get().loading, "buffering locked the controls");
            assert_eq!(playback_state_changed().get().status, PlaybackStatus::Playing);
            m.fire(Trigger::Buffering(false));
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Playing));
            assert!(!playback_state_changed().get().buffering);

            // Paused by the user while buffering: buffering ends without
            // resuming, and a resume before it ends keeps waiting.
            m.fire(Trigger::Buffering(true));
            m.fire(Trigger::Pause);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert!(playback_state_changed().get().buffering, "a paused stream still reports its buffering");
            m.fire(Trigger::Resume);
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Paused));
            assert!(playback_state_changed().get().buffering);
            m.fire(Trigger::Buffering(false));
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Playing));
            m.fire(Trigger::Pause);
            m.fire(Trigger::Buffering(true));
            m.fire(Trigger::Buffering(false));
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Paused), "buffering resumed a paused track");
            on_playing_exit();
        })
        .unwrap();
    }

    #[test]
    fn paused_start_reports_buffering() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin::default());
            let m = test_machine(&pb);
            m.state.set(State::Preparing);
            m.fire(Trigger::Pause);
            m.fire(Trigger::Buffering(true));
            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert!(playback_state_changed().get().buffering);
            m.fire(Trigger::Buffering(false));
            while ctx.iteration(false) {}
            assert!(!playback_state_changed().get().buffering);
            assert_eq!(pb.state.get(), Some(gst::State::Paused));
        })
        .unwrap();
    }

    #[test]
    fn gapless_next_track_buffering_does_not_pause() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::PlayingGapless);
            m.fire(Trigger::Buffering(true));
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Playing));
        })
        .unwrap();
    }

    /// #80: an error other than an expired token stops playback and tells
    /// the user once.
    #[test]
    fn error_stops_playback() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            ERRORS.with(|v| v.borrow_mut().clear());
            for from in [State::Preparing, State::Playing, State::Paused, State::PlayingGapless] {
                let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
                let m = test_machine(&pb);
                m.state.set(from);
                track_changed().set(Some(TrackChangedEvent { track: Track::default(), skip_history: false }));
                m.fire(Trigger::Error("Could not read from resource.".into()));
                m.fire(Trigger::Error("Internal data stream error.".into()));
                while ctx.iteration(false) {}
                assert_eq!(m.state.get(), State::Stopped, "{from:?}");
                assert_eq!(pb.state.get(), Some(gst::State::Null), "{from:?}");
                assert!(track_changed().get().is_none(), "{from:?}");
                assert!(!playback_state_changed().get().loading, "{from:?}");
            }
            let errors = ERRORS.with(|v| v.borrow().clone());
            assert_eq!(errors, vec!["Could not read from resource."; 4]);

            // The user already picked another track: no error for the old one.
            let pb = Rc::new(Testbin::default());
            let m = test_machine(&pb);
            m.state.set(State::Playing);
            m.fire(Trigger::Error("Could not read from resource.".into()));
            m.queue.borrow_mut().push_back(Trigger::Stop);
            while ctx.iteration(false) {}
            assert_eq!(ERRORS.with(|v| v.borrow().len()), 4, "a superseded error was reported");
        })
        .unwrap();
    }

    /// A message the old stream posted before it was torn down (here the
    /// follow-up error after an expired token) does not reach the next one.
    #[test]
    fn teardown_drops_the_old_streams_messages() {
        let pb = Rc::new(Testbin::default());
        let m = test_machine(&pb);
        for t in [Trigger::Error("Internal data stream error.".into()), Trigger::Buffering(true), Trigger::StreamStart, Trigger::Pause, Trigger::Eos, Trigger::Stop] {
            m.queue.borrow_mut().push_back(t);
        }
        m.drop_stream_messages();
        let left: Vec<String> = m.queue.borrow().iter().map(|t| format!("{t:?}")).collect();
        assert_eq!(left, ["TRIGGER_PAUSE", "TRIGGER_STOP"]);
    }

    /// An expired token while paused tears the stream down at once, so its
    /// follow-up stream error does not stop playback, and reloads on resume.
    #[test]
    fn token_expired_while_paused_reloads_on_resume() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            ERRORS.with(|v| v.borrow_mut().clear());
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Paused)), ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::Paused);
            m.fire(Trigger::TokenExpired);
            m.fire(Trigger::Error("Internal data stream error.".into()));
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert_eq!(pb.state.get(), Some(gst::State::Ready));
            assert!(ERRORS.with(|v| v.borrow().is_empty()));
            assert_eq!(m.target(&Trigger::Resume), Some(State::PlayingRetrying));
        })
        .unwrap();
    }

    /// An error with a next / previous queued behind it is reported only
    /// if that action starts no other track.
    #[test]
    fn error_waits_for_a_queued_next() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            ERRORS.with(|v| v.borrow_mut().clear());
            let pb = Rc::new(Testbin::default());
            let m = test_machine(&pb);
            m.state.set(State::Playing);
            m.fire(Trigger::Error("Could not read from resource.".into()));
            m.queue.borrow_mut().push_back(Trigger::Deferred(Box::new(|| {})));
            while ctx.iteration(false) {}
            assert_eq!(ERRORS.with(|v| v.borrow().len()), 1, "a next that started nothing hid the error");

            m.state.set(State::Playing);
            m.fire(Trigger::Error("Could not read from resource.".into()));
            let started = Rc::clone(&m);
            // Stands in for the next track starting.
            m.queue.borrow_mut().push_back(Trigger::Deferred(Box::new(move || started.state.set(State::Preparing))));
            while ctx.iteration(false) {}
            assert_eq!(ERRORS.with(|v| v.borrow().len()), 1, "the error was reported over the next track");
        })
        .unwrap();
    }

    #[test]
    fn failed_start_clears_and_reports() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            ERRORS.with(|v| v.borrow_mut().clear());
            let pb = Rc::new(Testbin { state_should_error: true, ..Default::default() });
            let m = test_machine(&pb);
            track_changed().set(Some(TrackChangedEvent { track: Track::default(), skip_history: false }));
            m.fire(Trigger::PlayTrack(Track::default(), false));
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Stopped);
            assert!(track_changed().get().is_none());
            assert_eq!(ERRORS.with(|v| v.borrow().len()), 1);
        })
        .unwrap();
    }

    /// A start accepted after a deferred report drops it: when that start
    /// fails too, only its own failure is reported.
    #[test]
    fn accepted_start_drops_a_deferred_report() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            ERRORS.with(|v| v.borrow_mut().clear());
            let pb = Rc::new(Testbin { state_should_error: true, ..Default::default() });
            let m = test_machine(&pb);
            m.fire(Trigger::PlayTrack(Track::default(), false));
            m.queue.borrow_mut().push_back(Trigger::ReportError(Some("a".into()), "old".into()));
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Stopped);
            assert_eq!(ERRORS.with(|v| v.borrow().len()), 1);
            assert_ne!(ERRORS.with(|v| v.borrow()[0].clone()), "old");
        })
        .unwrap();
    }

    /// A failed reload stops and reports instead of leaving the machine in
    /// `PlayingRetrying`, where Play does nothing.
    #[test]
    fn failed_reload_stops() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            ERRORS.with(|v| v.borrow_mut().clear());
            let pb = Rc::new(Testbin { state_should_error: true, ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::Paused);
            m.reload_on_resume.set(true);
            m.fire(Trigger::Resume);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Stopped);
            assert_eq!(ERRORS.with(|v| v.borrow().len()), 1);
        })
        .unwrap();
    }

    #[test]
    fn buffering_after_stop_is_ignored() {
        let pb = Rc::new(Testbin::default());
        let m = test_machine(&pb);
        playback_state_changed().set(PlaybackState::default());
        m.on_buffering(true);
        assert!(!playback_state_changed().get().buffering);
        assert_eq!(pb.state.get(), None);
    }

    /// #99: the old track's position is not published while the next loads.
    #[test]
    fn preparing_resets_position() {
        playback_state_changed().set(PlaybackState { position: secs(150), status: PlaybackStatus::Playing, ..Default::default() });
        on_preparing();
        assert_eq!(playback_state_changed().get().position, Duration::ZERO);
    }

    #[test]
    fn gapless_gate_wakes_immediately_on_blocking_op() {
        let gate = Arc::new(GaplessGate::new());
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter_gate = Arc::clone(&gate);
        let handle = std::thread::spawn(move || {
            let started = Instant::now();
            let timed_out = waiter_gate.wait(Duration::from_secs(15), move |token| {
                tx.send(token).unwrap();
            });
            (timed_out, started.elapsed())
        });

        let token = rx.recv().unwrap();
        let guard = gate.enter_blocking_op();
        let (timed_out, elapsed) = handle.join().unwrap();
        assert!(!timed_out, "canceling the gate must not report a timeout");
        assert!(elapsed < Duration::from_secs(1), "waiter stalled for {elapsed:?}");
        assert!(token.is_cancelled());

        // While a blocking op is active, new waits return immediately without dispatching.
        let dispatched = Arc::new(AtomicBool::new(false));
        let d = Arc::clone(&dispatched);
        assert!(!gate.wait(Duration::from_secs(15), move |_| d.store(true, Ordering::SeqCst)));
        assert!(!dispatched.load(Ordering::SeqCst));
        drop(guard);

        // Normal completion wakes immediately and keeps the token uncanceled.
        let (tx, rx) = std::sync::mpsc::channel();
        let waiter_gate = Arc::clone(&gate);
        let handle = std::thread::spawn(move || {
            let started = Instant::now();
            let timed_out = waiter_gate.wait(Duration::from_secs(15), move |token| {
                tx.send(token).unwrap();
            });
            (timed_out, started.elapsed())
        });
        let token = rx.recv().unwrap();
        assert!(!token.is_cancelled());
        drop(token);
        let (timed_out, elapsed) = handle.join().unwrap();
        assert!(!timed_out);
        assert!(elapsed < Duration::from_secs(1), "drop wake stalled for {elapsed:?}");
    }

    #[test]
    fn cancelled_gapless_hint_does_not_transition_or_enqueue() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let gate = Arc::new(GaplessGate::new());
            let m = Machine::with_gate(Rc::clone(&pb) as Rc<dyn Playbin>, Arc::clone(&gate), Hooks::default());
            m.state.set(State::Playing);
            let token = gate.begin_nonblocking().unwrap();
            drop(gate.enter_blocking_op());
            assert!(token.is_cancelled());
            m.fire(Trigger::GaplessHint(Some(token)));
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Playing);
            assert!(pb.uri.borrow().is_empty());
        })
        .unwrap();
    }

    #[test]
    fn queued_stop_cancels_a_wait_that_starts_before_it_is_handled() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let gate = Arc::new(GaplessGate::new());
            let m = Machine::with_gate(Rc::clone(&pb) as Rc<dyn Playbin>, Arc::clone(&gate), Hooks::default());
            m.state.set(State::Playing);
            m.fire(Trigger::Stop);
            let start = Instant::now();
            assert!(!gate.wait(Duration::from_secs(5), |_| {}));
            assert!(start.elapsed() < Duration::from_secs(1));
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Stopped);
            assert!(gate.begin_nonblocking().is_some(), "handled stop must release the gate");
        })
        .unwrap();
    }

    thread_local! {
        static COMMITTED: RefCell<Vec<(String, bool, GaplessSource)>> = const { RefCell::new(Vec::new()) };
    }

    #[test]
    fn playing_gapless_pause_resume_preserves_and_commits_armed_entry() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            COMMITTED.with(|c| c.borrow_mut().clear());
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = Machine::new(
                Rc::clone(&pb) as Rc<dyn Playbin>,
                Hooks {
                    on_gapless_promote: |entry| {
                        COMMITTED.with(|c| c.borrow_mut().push((entry.track.id().to_string(), entry.did_repeat, entry.source)));
                    },
                    ..Hooks::default()
                },
            );
            let mut next = Track::default();
            next.info.id = "track-b".into();
            next.info.duration = secs(210);
            let manifest = TrackManifest { quality: TrackQuality::Lossless, ..Default::default() };
            m.state.set(State::PlayingGapless);
            m.begin_gapless(GaplessEntry {
                track: next,
                did_repeat: false,
                source: GaplessSource::BaseQueue { rewound: false },
            });
            assert!(m.arm_gapless(manifest, 1.0).unwrap());
            track_changed().set(None);

            m.fire(Trigger::Pause);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert_eq!(pb.state.get(), Some(gst::State::Paused));
            assert!(m.has_armed_gapless());

            m.fire(Trigger::Resume);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::PlayingGapless);
            assert_eq!(pb.state.get(), Some(gst::State::Playing));

            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Playing);
            assert!(!m.has_armed_gapless());
            assert_eq!(
                COMMITTED.with(|c| c.borrow().clone()),
                vec![("track-b".into(), false, GaplessSource::BaseQueue { rewound: false })]
            );
            assert_eq!(track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string())).as_deref(), Some("track-b"));
            assert_eq!(playback_quality_changed().get(), TrackQuality::Lossless);
            on_playing_exit();
        })
        .unwrap();
    }

    #[test]
    fn playing_retrying_seeks_only_on_stream_start_and_preserves_across_pause() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            // Non-StreamStart exits clear retry_position without seeking.
            for trigger in [Trigger::Stop, Trigger::Eos, Trigger::Error("boom".into()), Trigger::TokenExpired] {
                let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
                let m = test_machine(&pb);
                m.state.set(State::PlayingRetrying);
                m.retry_position.set(Some(secs(42)));
                m.fire(trigger);
                while ctx.iteration(false) {}
                assert_eq!(m.state.get(), State::Stopped);
                assert!(pb.seeks.borrow().is_empty(), "non-StreamStart exit sought unexpectedly");
                assert_eq!(m.retry_position.get(), None);
            }

            // Starting a different track clears retry_position without seeking.
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), state_should_error: true, ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::PlayingRetrying);
            m.retry_position.set(Some(secs(55)));
            m.fire(Trigger::PlayTrack(Track::default(), false));
            while ctx.iteration(false) {}
            assert!(pb.seeks.borrow().is_empty());
            assert_eq!(m.retry_position.get(), None);

            // Pausing before StreamStart preserves the retry seek until StreamStart arrives.
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::PlayingRetrying);
            m.retry_position.set(Some(secs(73)));
            m.fire(Trigger::Pause);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert!(pb.seeks.borrow().is_empty());
            assert_eq!(m.retry_position.get(), Some(secs(73)));

            m.fire(Trigger::Resume);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::PlayingRetrying);
            assert_eq!(pb.state.get(), Some(gst::State::Playing));
            assert!(pb.seeks.borrow().is_empty());

            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Playing);
            assert_eq!(*pb.seeks.borrow(), vec![secs(73)]);
            assert_eq!(m.retry_position.get(), None);

            // StreamStart arriving while paused also applies the saved retry seek once.
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::PlayingRetrying);
            m.retry_position.set(Some(secs(91)));
            m.fire(Trigger::Pause);
            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert_eq!(m.state.get(), State::Paused);
            assert_eq!(*pb.seeks.borrow(), vec![secs(91)]);
            assert_eq!(m.retry_position.get(), None);
            on_playing_exit();
        })
        .unwrap();
    }

    #[test]
    fn retry_and_paused_token_expired_capture_position_before_ready() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let pb = Rc::new(Testbin {
                state: Cell::new(Some(gst::State::Paused)),
                position: Cell::new(secs(88)),
                reset_position_on_ready: true,
                ..Default::default()
            });
            playback_state_changed().set(PlaybackState { position: secs(10), status: PlaybackStatus::Paused, ..Default::default() });
            let m = test_machine(&pb);
            m.state.set(State::Paused);
            m.fire(Trigger::TokenExpired);
            while ctx.iteration(false) {}
            assert_eq!(pb.state.get(), Some(gst::State::Ready));
            assert_eq!(pb.query_position(), None);
            assert_eq!(m.retry_position.get(), Some(secs(88)));

            // `retry()` also captures position before `set_state(Ready)`.
            let pb = Rc::new(Testbin {
                state: Cell::new(Some(gst::State::Playing)),
                position: Cell::new(secs(64)),
                reset_position_on_ready: true,
                state_should_error: true,
                ..Default::default()
            });
            playback_state_changed().set(PlaybackState { position: secs(5), status: PlaybackStatus::Playing, ..Default::default() });
            let m = test_machine(&pb);
            assert!(ctx.block_on(m.retry()).is_err());
            assert_eq!(m.retry_position.get(), Some(secs(64)));
        })
        .unwrap();
    }

    thread_local! {
        static GAPLESS_EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    #[test]
    fn gapless_lifecycle_hooks_coordinate_arm_promote_and_cancel() {
        let ctx = glib::MainContext::new();
        ctx.with_thread_default(|| {
            let hooks = || Hooks {
                on_gapless_arm: |m| GAPLESS_EVENTS.with(|e| e.borrow_mut().push(format!("arm:{}", m.url))),
                on_gapless_promote: |entry| {
                    GAPLESS_EVENTS.with(|e| e.borrow_mut().push(format!("promote:{}", entry.track.id())));
                },
                on_gapless_cancel: || GAPLESS_EVENTS.with(|e| e.borrow_mut().push("cancel".into())),
                ..Hooks::default()
            };
            let entry = |id: &str| {
                let mut track = Track::default();
                track.info.id = id.into();
                GaplessEntry { track, did_repeat: false, source: GaplessSource::BaseQueue { rewound: false } }
            };

            // Canceling when idle is a no-op.
            GAPLESS_EVENTS.with(|e| e.borrow_mut().clear());
            let pb = Rc::new(Testbin::default());
            let m = Machine::new(Rc::clone(&pb) as Rc<dyn Playbin>, hooks());
            m.cancel_gapless();
            assert!(GAPLESS_EVENTS.with(|e| e.borrow().is_empty()));

            // Failed arm rolls back the slot and notifies cancel once.
            let failing_pb = Rc::new(Testbin { volume_should_error: true, ..Default::default() });
            let m = Machine::new(Rc::clone(&failing_pb) as Rc<dyn Playbin>, hooks());
            m.begin_gapless(entry("b"));
            let manifest = TrackManifest { url: "https://example.invalid/b".into(), ..Default::default() };
            assert_eq!(m.arm_gapless(manifest, 1.0).unwrap_err().to_string(), "set volume failed");
            assert!(m.gapless_entry().is_none());
            assert!(!m.has_armed_gapless());
            assert_eq!(GAPLESS_EVENTS.with(|e| e.borrow().clone()), ["arm:https://example.invalid/b", "cancel"]);

            // Stop cancels an armed gapless handoff once.
            GAPLESS_EVENTS.with(|e| e.borrow_mut().clear());
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = Machine::new(Rc::clone(&pb) as Rc<dyn Playbin>, hooks());
            m.state.set(State::PlayingGapless);
            m.begin_gapless(entry("c"));
            assert!(m.arm_gapless(TrackManifest { url: "https://example.invalid/c".into(), ..Default::default() }, 0.75).unwrap());
            assert!(m.has_armed_gapless());
            assert_eq!(*pb.uri.borrow(), "https://example.invalid/c");
            assert!((pb.volume.get() - 0.75).abs() < f64::EPSILON);
            m.fire(Trigger::Stop);
            while ctx.iteration(false) {}
            assert!(!m.has_armed_gapless());
            assert_eq!(GAPLESS_EVENTS.with(|e| e.borrow().clone()), ["arm:https://example.invalid/c", "cancel"]);

            // StreamStart promotes an armed gapless handoff without firing cancel.
            GAPLESS_EVENTS.with(|e| e.borrow_mut().clear());
            let pb = Rc::new(Testbin { state: Cell::new(Some(gst::State::Playing)), ..Default::default() });
            let m = Machine::new(Rc::clone(&pb) as Rc<dyn Playbin>, hooks());
            m.state.set(State::PlayingGapless);
            m.begin_gapless(entry("d"));
            assert!(m.arm_gapless(TrackManifest { url: "https://example.invalid/d".into(), ..Default::default() }, 1.0).unwrap());
            m.fire(Trigger::StreamStart);
            while ctx.iteration(false) {}
            assert!(!m.has_armed_gapless());
            assert_eq!(GAPLESS_EVENTS.with(|e| e.borrow().clone()), ["arm:https://example.invalid/d", "promote:d"]);
            on_playing_exit();
        })
        .unwrap();
    }
}

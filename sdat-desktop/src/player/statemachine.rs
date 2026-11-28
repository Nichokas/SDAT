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
use std::sync::Arc;
use std::time::Duration;

use super::gapless_gate::{GaplessGate, GaplessToken};

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
    let quality = settings::audio_quality();
    api::run(move || {
        if let Some(m) = super::audio_cache::manifest(&id, quality) {
            return Ok(m);
        }
        tonearm_core::audio_cache::cancel(&id);
        service::get_track_manifest(&id)
    })
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
    // Volume first: a failure must not leave a next URI that plays without its queue commit.
    playbin.set_volume(volume)?;
    playbin.set_uri(&manifest.url);
    Ok(())
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
    /// Runs once the triggers queued before it were handled, even when a Stop waits behind it: cleanup, not a start.
    AfterStop(Box<dyn FnOnce()>),
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
            Trigger::AfterStop(_) => "after-stop",
            Trigger::ReportError(..) => "report-error",
            Trigger::Pause => "TRIGGER_PAUSE",
            Trigger::Resume => "TRIGGER_RESUME",
            Trigger::Stop => "TRIGGER_STOP",
            Trigger::GaplessHint(_) => "TRIGGER_GAPLESS_HINT",
            Trigger::TokenExpired => "TRIGGER_TOKEN_EXPIRED",
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
    /// The trigger being handled is a stop or start (see [`Machine::sync_gate_hold`]).
    handling_cancel: Cell<bool>,
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
    /// The retry's cached copy failed and is gone: one more retry streams instead of stopping.
    retry_streamed: Cell<bool>,
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
            handling_cancel: Cell::new(false),
            playbin,
            gate,
            gapless: RefCell::new(None),
            retry_position: Cell::new(None),
            pause_on_start: Cell::new(false),
            buffering: Cell::new(false),
            reload_on_resume: Cell::new(false),
            retry_streamed: Cell::new(false),
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

    pub fn has_queued_stop_or_start(&self) -> bool {
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

    pub fn preparing(&self) -> bool {
        self.state.get() == State::Preparing
    }

    /// Whether the track preparing now starts paused: paused while it prepared, or a start that seeks first.
    pub fn preparing_paused(&self) -> bool {
        self.pause_on_start.get()
    }

    pub fn retrying(&self) -> bool {
        self.state.get() == State::PlayingRetrying
    }

    /// The next `TokenExpired` of this retry reloads it once more instead of stopping.
    pub fn retry_once_more(&self) {
        self.retry_streamed.set(true);
    }

    /// Whether the machine is stopped and nothing is about to start playback.
    pub fn stopped_without_pending_start(&self) -> bool {
        self.state.get() == State::Stopped && !self.start_pending()
    }

    /// Queues a trigger; transitions run in order on the main loop.
    pub fn fire(self: &Rc<Self>, t: Trigger) {
        self.queue.borrow_mut().push_back(t);
        self.sync_gate_hold();
        if self.running.replace(true) {
            return;
        }
        let this = Rc::clone(self);
        glib::spawn_future_local(async move {
            loop {
                let next = this.queue.borrow_mut().pop_front();
                let Some(t) = next else { break };
                this.handling_start.set(matches!(t, Trigger::PlayTrack(..) | Trigger::PlayTrackId(..)));
                this.handling_cancel.set(cancels_gapless(&t));
                this.handle(t).await;
                this.handling_cancel.set(false);
                this.sync_gate_hold();
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
            (PlayingRetrying, Trigger::TokenExpired) if self.retry_streamed.replace(false) => Some(PlayingRetrying),
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
        if let Trigger::AfterStop(action) = t {
            action();
            return;
        }
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
                Trigger::Stop if from == State::Stopped => {
                    if track_changed().with(Option::is_some) {
                        on_stopped_stop();
                    }
                }
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
            self.retry_streamed.set(false);
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
                // Without a next URI the track ends in EOS, and `on_eos_stopped` starts the next queued track.
                // PlayingGapless -> Playing runs no exit or entry action (the worker never stopped), so the
                // state is set here rather than through a queued trigger that a queued Pause would overtake.
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
        self.sync_gate_hold();
    }

    /// Holds the gate while a stop or start is queued or being handled; the
    /// drain loop recomputes it after every trigger, so nothing can leak it.
    fn sync_gate_hold(&self) {
        let queued = self.queue.borrow().iter().any(cancels_gapless);
        self.gate.set_held(self.handling_cancel.get() || queued);
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
#[path = "statemachine_tests.rs"]
pub mod tests;

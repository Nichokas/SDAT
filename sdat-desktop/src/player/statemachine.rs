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
use std::sync::mpsc;
use std::time::Duration;

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
global_signal!(pub fn enqueued_track() -> Sig<Option<TrackManifest>> = Sig::new(None));

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
    playbin.seek(position, gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE);
}

async fn fetch_and_enqueue_track(playbin: &dyn Playbin, track_id: &str) -> Result<TrackManifest> {
    let id = track_id.to_string();
    let manifest = api::run(move || service::get_track_manifest(&id))
        .await
        .map_err(|e| anyhow!("failed to fetch and enqueue track: {e}"))?;
    playbin.set_uri(&manifest.url);
    playbin.set_volume(settings::volume())?;
    enqueued_track().set(Some(manifest.clone()));
    Ok(manifest)
}

pub async fn on_play_track(playbin: &dyn Playbin, track: Track, skip_history: bool) -> Result<()> {
    let manifest = fetch_and_enqueue_track(playbin, track.id()).await?;
    playbin.set_state(gst::State::Playing)?;
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
    Pause,
    Resume,
    Stop,
    /// Sent from playbin's streaming thread; `done` is signalled once the
    /// next URI has been enqueued (or enqueuing failed).
    GaplessHint(Option<mpsc::Sender<()>>),
    TokenExpired,
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

/// Hooks into the player module (queues, autoplay) that the machine needs.
pub struct Hooks {
    pub next_track: fn(peek: bool) -> (Option<Track>, bool),
    pub on_eos_stopped: fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>>,
    pub clear_source: fn(),
    /// Tells the user that playback stopped on an error.
    pub on_error: fn(&str),
}

pub struct Machine {
    state: Cell<State>,
    queue: RefCell<VecDeque<Trigger>>,
    running: Cell<bool>,
    /// Whether the trigger being handled right now starts a track.
    handling_start: Cell<bool>,
    playbin: Rc<dyn Playbin>,
    gapless_manifest: RefCell<Option<TrackManifest>>,
    retry_position: Cell<Duration>,
    /// A pause arrived while the track was still preparing: the stream
    /// starts paused instead of playing (Listen Together aligned start).
    pause_on_start: Cell<bool>,
    /// The stream ran dry and holds the pipeline paused until its buffer is
    /// full again.
    buffering: Cell<bool>,
    hooks: Hooks,
}

impl Machine {
    pub fn new(playbin: Rc<dyn Playbin>, hooks: Hooks) -> Rc<Self> {
        Rc::new(Machine {
            state: Cell::new(State::Stopped),
            queue: RefCell::new(VecDeque::new()),
            running: Cell::new(false),
            handling_start: Cell::new(false),
            playbin,
            gapless_manifest: RefCell::new(None),
            retry_position: Cell::new(Duration::ZERO),
            pause_on_start: Cell::new(false),
            buffering: Cell::new(false),
            hooks,
        })
    }


    /// Whether the next track was already chosen (and is being fetched or
    /// waiting for its stream to start) for gapless playback.
    pub fn gapless_pending(&self) -> bool {
        self.state.get() == State::PlayingGapless
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
                this.handle(t).await;
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
                Trigger::GaplessHint(_) if s == Playing => Some(PlayingGapless),
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
                // Ignore(): the hint is a byproduct of retrying / already handled.
                Trigger::GaplessHint(done) => {
                    if let Some(d) = done {
                        let _ = d.send(());
                    }
                }
                Trigger::Stop if from == State::Stopped => log::debug!("already stopped"),
                Trigger::Error(e) if from == State::Stopped => log::debug!("playback error while stopped: {e}"),
                Trigger::StreamStart if from == State::Playing => {}
                _ => log::error!("Failed Transition state={from:?} trigger={t:?}"),
            }
            return;
        };
        // An accepted new start or stop drops a pause meant for the old one.
        if matches!(to, State::Preparing | State::Stopped) {
            self.pause_on_start.set(false);
            self.buffering.set(false);
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
            let (track, did_repeat) = (self.hooks.next_track)(false);
            let manifest = self.gapless_manifest.borrow_mut().take();
            if let (Some(track), Some(manifest)) = (track, manifest) {
                on_playing_gapless_exit(track, &manifest, did_repeat);
            }
        }
        if from == State::PlayingRetrying && to != State::PlayingRetrying {
            on_playing_retry_exit(&*self.playbin, self.retry_position.get());
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
                        on_stopped_stop();
                        // A queued stop or start has already replaced the
                        // source and moved on from the failed track.
                        let superseded = self.queue.borrow().iter().any(|t| matches!(t, Trigger::Stop | Trigger::PlayTrack(..) | Trigger::PlayTrackId(..) | Trigger::Deferred(_)));
                        if !superseded {
                            (self.hooks.clear_source)();
                            (self.hooks.on_error)(&e);
                        }
                    }
                    _ => {}
                }
                r
            }
            (State::Paused, _) if started_paused => on_paused_started(&*self.playbin),
            (State::Paused, _) => on_paused(&*self.playbin),
            (State::Preparing, t) => {
                on_preparing();
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
                }
                r
            }
            (State::Playing, _) if sub_to_parent => Ok(()),
            (State::Playing, _) => on_playing(Rc::clone(&self.playbin), self.buffering.get()),
            (State::PlayingGapless, Trigger::GaplessHint(done)) => {
                let _ = into_sub;
                let r = async {
                    let (track, _) = (self.hooks.next_track)(true);
                    let track = track.ok_or_else(|| anyhow!("no track available for gapless playback"))?;
                    let m = fetch_and_enqueue_track(&*self.playbin, track.id()).await?;
                    *self.gapless_manifest.borrow_mut() = Some(m);
                    Ok::<_, anyhow::Error>(())
                }
                .await;
                if let Some(d) = done {
                    let _ = d.send(());
                }
                if let Err(e) = r {
                    log::error!("failed to enqueue next track: {e:#}");
                    self.fire(Trigger::Failed);
                }
                Ok(())
            }
            (State::PlayingRetrying, _) => self.retry().await,
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
        // The gapless next track buffers while the current one still plays.
        if state == State::PlayingGapless && buffering {
            return;
        }
        if self.buffering.replace(buffering) == buffering {
            return;
        }
        log::debug!("buffering={buffering} state={state:?}");
        // Paused, or held paused for a paused start: the pipeline stays paused.
        let held = state.is_playing() || (state == State::Preparing && !self.pause_on_start.get());
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

    /// Called once the old stream is torn down: what it posted (a follow-up
    /// error after an expired token, a late stream start or buffering) must
    /// not act on the next stream.
    fn drop_stream_messages(&self) {
        self.playbin.flush_bus();
        self.queue.borrow_mut().retain(|t| {
            !matches!(t, Trigger::Error(_) | Trigger::Buffering(_) | Trigger::TokenExpired | Trigger::StreamStart | Trigger::Eos)
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
        pb.set_state(gst::State::Ready)?;
        self.drop_stream_messages();
        let id = track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string())).unwrap_or_default();
        fetch_and_enqueue_track(pb, &id).await?;
        let pos = pb.query_position().unwrap_or_else(|| playback_state_changed().get().position);
        self.retry_position.set(pos);
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
        pub volume: Cell<f64>,
        pub volume_should_error: bool,
        pub state: Cell<Option<gst::State>>,
        pub state_should_error: bool,
        pub uri: RefCell<String>,
    }

    impl Playbin for Testbin {
        fn set_state(&self, state: gst::State) -> Result<()> {
            if self.state_should_error {
                anyhow::bail!("set state failed");
            }
            self.state.set(Some(state));
            Ok(())
        }
        fn query_duration(&self) -> Option<Duration> {
            (!self.duration_should_error).then(|| self.duration.get())
        }
        fn query_position(&self) -> Option<Duration> {
            (!self.position_should_error).then(|| self.position.get())
        }
        fn seek(&self, position: Duration, _: gst::SeekFlags) -> bool {
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
        let m = Machine::new(
            Rc::new(Testbin::default()),
            Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || {}, on_error: |_| {} },
        );
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
        let m = Machine::new(
            Rc::new(Testbin::default()),
            Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || {}, on_error: |_| {} },
        );
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
                    Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || CLEARED.with(|c| c.set(c.get() + 1)), on_error: |_| {} },
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
            let m = Machine::new(
                Rc::new(Testbin::default()),
                Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || {}, on_error: |_| {} },
            );
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
            let m = Machine::new(
                Rc::new(Testbin::default()),
                Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || {}, on_error: |_| {} },
            );
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
            let m = Machine::new(
                Rc::clone(&pb) as Rc<dyn Playbin>,
                Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || {}, on_error: |_| {} },
            );
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
                next_track: |_| (None, false),
                on_eos_stopped: || Box::pin(async {}),
                clear_source: || {},
                on_error: |e| ERRORS.with(|v| v.borrow_mut().push(e.to_string())),
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

    /// #99: the old track's position is not published while the next loads.
    #[test]
    fn preparing_resets_position() {
        playback_state_changed().set(PlaybackState { position: secs(150), status: PlaybackStatus::Playing, ..Default::default() });
        on_preparing();
        assert_eq!(playback_state_changed().get().position, Duration::ZERO);
    }
}

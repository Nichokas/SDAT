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
use crate::signals::State as Sig;
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
    pub loading: bool,
}

#[derive(Debug, Clone)]
pub struct TrackChangedEvent {
    pub track: Track,
    pub skip_history: bool,
}

global_signal!(pub fn playback_state_changed() -> Sig<PlaybackState> = Sig::new(PlaybackState::default()));
global_signal!(pub fn playback_quality_changed() -> Sig<TrackQuality> = Sig::new(TrackQuality::HighResLossless));
global_signal!(pub fn track_changed() -> Sig<Option<TrackChangedEvent>> = Sig::new(None));
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
        s
    });
    Ok(())
}

pub fn on_stopped_stop() -> Result<()> {
    track_changed().set(None);
    playback_state_changed().notify(|mut s| {
        s.duration = Duration::ZERO;
        s.position = Duration::ZERO;
        s
    });
    Ok(())
}

pub fn on_paused(playbin: &dyn Playbin) -> Result<()> {
    playbin.set_state(gst::State::Paused)?;
    playback_state_changed().notify(|mut s| {
        s.status = PlaybackStatus::Paused;
        s
    });
    Ok(())
}

pub fn on_preparing() {
    playback_state_changed().notify(|mut s| {
        s.loading = true;
        s
    });
}

pub fn on_playing(playbin: Rc<dyn Playbin>) -> Result<()> {
    playbin.set_state(gst::State::Playing)?;
    playback_state_changed().notify(|mut s| {
        if let Some(d) = playbin.query_duration() {
            s.duration = d;
        }
        s.loading = false;
        s.status = PlaybackStatus::Playing;
        s
    });
    start_worker(playbin);
    Ok(())
}

pub fn on_playing_exit() -> Result<()> {
    stop_worker();
    Ok(())
}

pub fn on_playing_gapless_exit(track: Track, manifest: &TrackManifest, did_repeat: bool) {
    playback_state_changed().notify(|mut s| {
        s.duration = track.duration();
        s.position = Duration::ZERO;
        s
    });
    if did_repeat {
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
    playbin.set_state(gst::State::Ready)?;
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

pub enum Trigger {
    PlayTrackId(String, bool),
    PlayTrack(Track, bool),
    Pause,
    Resume,
    Stop,
    /// Sent from playbin's streaming thread; `done` is signalled once the
    /// next URI has been enqueued (or enqueuing failed).
    GaplessHint(Option<mpsc::Sender<()>>),
    TokenExpired,
    Failed,
    StreamStart,
    Eos,
}

impl std::fmt::Debug for Trigger {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Trigger::PlayTrackId(..) => "TRIGGER_PLAY_TRACK_ID",
            Trigger::PlayTrack(..) => "TRIGGER_PLAY_TRACK",
            Trigger::Pause => "TRIGGER_PAUSE",
            Trigger::Resume => "TRIGGER_RESUME",
            Trigger::Stop => "TRIGGER_STOP",
            Trigger::GaplessHint(_) => "TRIGGER_GAPLESS_HINT",
            Trigger::TokenExpired => "TRIGGER_TOKEN_EXPIRED",
            Trigger::Failed => "TRIGGER_FAILED",
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
}

pub struct Machine {
    state: Cell<State>,
    queue: RefCell<VecDeque<Trigger>>,
    running: Cell<bool>,
    playbin: Rc<dyn Playbin>,
    gapless_manifest: RefCell<Option<TrackManifest>>,
    retry_position: Cell<Duration>,
    hooks: Hooks,
}

impl Machine {
    pub fn new(playbin: Rc<dyn Playbin>, hooks: Hooks) -> Rc<Self> {
        Rc::new(Machine {
            state: Cell::new(State::Stopped),
            queue: RefCell::new(VecDeque::new()),
            running: Cell::new(false),
            playbin,
            gapless_manifest: RefCell::new(None),
            retry_position: Cell::new(Duration::ZERO),
            hooks,
        })
    }

    pub fn state(&self) -> State {
        self.state.get()
    }

    /// Queues a trigger; transitions run in order on the main loop.
    pub fn fire(self: &Rc<Self>, t: Trigger) {
        self.queue.borrow_mut().push_back(t);
        if self.running.replace(true) {
            return;
        }
        let this = self.clone();
        glib::spawn_future_local(async move {
            loop {
                let next = this.queue.borrow_mut().pop_front();
                let Some(t) = next else { break };
                this.handle(t).await;
            }
            this.running.set(false);
        });
    }

    fn target(&self, t: &Trigger) -> Option<State> {
        use State::*;
        let s = self.state.get();
        match (s, t) {
            (Stopped | Paused | Playing | PlayingGapless | PlayingRetrying, Trigger::PlayTrackId(..) | Trigger::PlayTrack(..)) => {
                Some(Preparing)
            }
            (Paused, Trigger::Resume) => Some(Playing),
            (Paused | Preparing, Trigger::Stop) => Some(Stopped),
            (Preparing, Trigger::TokenExpired) => Some(Stopped),
            (Preparing, Trigger::StreamStart) => Some(Playing),
            (PlayingGapless | PlayingRetrying, Trigger::StreamStart) => Some(Playing),
            (PlayingGapless, Trigger::Failed) => Some(Playing),
            (PlayingRetrying, Trigger::TokenExpired) => Some(Stopped),
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
        let from = self.state.get();
        let Some(to) = self.target(&t) else {
            match t {
                // Ignore(): the hint is a byproduct of retrying / already handled.
                Trigger::GaplessHint(done) => {
                    if let Some(d) = done {
                        let _ = d.send(());
                    }
                }
                Trigger::Stop if from == State::Stopped => log::debug!("already stopped"),
                Trigger::StreamStart if from == State::Playing => {}
                _ => log::error!("Failed Transition state={from:?} trigger={t:?}"),
            }
            return;
        };
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
            let _ = on_playing_exit();
        }
        self.state.set(to);

        // Entry actions.
        let res: Result<()> = match (to, t) {
            (State::Stopped, t) => {
                let mut r = on_stopped(&*self.playbin);
                match t {
                    Trigger::Stop => {
                        (self.hooks.clear_source)();
                        r = r.and(on_stopped_stop());
                    }
                    Trigger::TokenExpired => {
                        log::error!(
                            "track was enqueued successfully, but playback was rejected by remote server. this may be due to a network issue, account restrictions or a temporary outage."
                        );
                        (self.hooks.clear_source)();
                        r = r.and(on_stopped_stop());
                    }
                    Trigger::Eos => (self.hooks.on_eos_stopped)().await,
                    _ => {}
                }
                r
            }
            (State::Paused, _) => on_paused(&*self.playbin),
            (State::Preparing, t) => {
                on_preparing();
                let r = match t {
                    Trigger::PlayTrackId(id, skip) => {
                        let id2 = id.clone();
                        match api::run(move || service::get_track(&id2)).await {
                            Ok(track) => on_play_track(&*self.playbin, track, skip).await,
                            Err(e) => Err(e),
                        }
                    }
                    Trigger::PlayTrack(track, skip) => on_play_track(&*self.playbin, track, skip).await,
                    _ => Ok(()),
                };
                if let Err(e) = &r {
                    // Deviation from Go: don't stay stuck in PREPARING with a spinner.
                    log::error!("failed to start playback: {e:#}");
                    self.state.set(State::Stopped);
                    let _ = on_stopped(&*self.playbin);
                }
                r
            }
            (State::Playing, _) if sub_to_parent => Ok(()),
            (State::Playing, _) => on_playing(self.playbin.clone()),
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

    async fn retry(&self) -> Result<()> {
        let pb = &*self.playbin;
        pb.set_state(gst::State::Ready)?;
        let id = track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string())).unwrap_or_default();
        fetch_and_enqueue_track(pb, &id).await?;
        let mut pos = playback_state_changed().get().position;
        if let Some(p) = pb.query_position() {
            pos = p;
        }
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
        fn seek(&self, p: Duration, _: gst::SeekFlags) -> bool {
            self.position.set(p);
            !self.position_should_error
        }
        fn set_uri(&self, uri: &str) {
            *self.uri.borrow_mut() = uri.into();
        }
        fn set_volume(&self, v: f64) -> Result<()> {
            if self.volume_should_error {
                anyhow::bail!("set volume failed");
            }
            self.volume.set(v);
            Ok(())
        }
    }

    fn capture_next_state() -> Rc<RefCell<Option<PlaybackState>>> {
        let seen = Rc::new(RefCell::new(None));
        let s = seen.clone();
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
        on_playing(pb.clone()).unwrap();
        assert_eq!(pb.state.get(), Some(gst::State::Playing));
        on_playing_exit().unwrap();
    }

    #[test]
    fn on_playing_emits_playback_state() {
        let expected = secs(192);
        let pb = Rc::new(Testbin { duration: Cell::new(expected), ..Default::default() });
        let initial = PlaybackState { duration: secs(30), position: secs(12), status: PlaybackStatus::Paused, loading: true };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        on_playing(pb).unwrap();
        on_playing_exit().unwrap();
        let want = PlaybackState { duration: expected, status: PlaybackStatus::Playing, loading: false, ..initial };
        assert_eq!(seen.borrow().expect("not notified"), want);
    }

    #[test]
    fn on_playing_returns_state_error_without_updating_playback_state() {
        let initial = PlaybackState { duration: secs(60), position: secs(15), status: PlaybackStatus::Paused, loading: true };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        assert!(on_playing(Rc::new(Testbin { state_should_error: true, ..Default::default() })).is_err());
        assert!(seen.borrow().is_none());
        assert_eq!(playback_state_changed().get(), initial);
        stop_worker();
    }

    #[test]
    fn on_playing_preserves_duration_when_query_fails() {
        let initial = PlaybackState { duration: secs(120), ..Default::default() };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        on_playing(Rc::new(Testbin { duration: Cell::new(secs(300)), duration_should_error: true, ..Default::default() })).unwrap();
        on_playing_exit().unwrap();
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
        let initial = PlaybackState { duration: secs(180), position: secs(45), status: PlaybackStatus::Playing, loading: true };
        playback_state_changed().set(initial);
        let seen = capture_next_state();
        on_stopped(&Testbin::default()).unwrap();
        let want = PlaybackState { status: PlaybackStatus::Stopped, loading: false, ..initial };
        assert_eq!(seen.borrow().expect("not notified"), want);
    }

    #[test]
    fn on_stopped_returns_state_error_without_updating_playback_state() {
        let initial = PlaybackState { duration: secs(60), position: secs(15), status: PlaybackStatus::Playing, loading: true };
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
        let s = seen.clone();
        track_changed().on_lazy(move |t| {
            s.set(Some(t.is_none()));
            UNSUBSCRIBE
        });
        on_stopped_stop().unwrap();
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
            Hooks { next_track: |_| (None, false), on_eos_stopped: || Box::pin(async {}), clear_source: || {} },
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
            (Preparing, play(), None),
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
        ];
        for (from, t, want) in cases {
            m.state.set(from);
            assert_eq!(m.target(&t), want, "{from:?} + {t:?}");
        }
    }
}

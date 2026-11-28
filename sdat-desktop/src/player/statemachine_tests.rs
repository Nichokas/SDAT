use super::*;
use std::time::Instant;
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

/// Cleanup queued behind a Stop runs even when another Stop follows it.
#[test]
fn after_stop_runs_despite_a_later_stop() {
    let ctx = glib::MainContext::new();
    ctx.with_thread_default(|| {
        let m = Machine::new(Rc::new(Testbin::default()), Hooks::default());
        let ran = Rc::new(Cell::new(false));
        let r = Rc::clone(&ran);
        m.queue.borrow_mut().push_back(Trigger::Stop);
        m.queue.borrow_mut().push_back(Trigger::AfterStop(Box::new(move || r.set(true))));
        m.fire(Trigger::Stop);
        while ctx.iteration(false) {}
        assert!(ran.get());
        assert_eq!(m.state.get(), State::Stopped);
    })
    .unwrap();
}

#[test]
fn play_after_a_start_error_resumes_the_queue() {
    let ctx = glib::MainContext::new();
    ctx.with_thread_default(|| {
        ORDER.with(|o| o.borrow_mut().clear());
        let m = Machine::new(
            Rc::new(Testbin::default()),
            Hooks {
                on_stopped_resume: || ORDER.with(|o| o.borrow_mut().push("resume queue")),
                ..Hooks::default()
            },
        );
        m.state.set(State::Preparing);
        m.fire(Trigger::Error("start failed".into()));
        m.fire(Trigger::Resume);
        while ctx.iteration(false) {}
        assert_eq!(ORDER.with(|o| o.borrow().clone()), ["resume queue"]);

        m.fire(Trigger::Resume);
        m.fire(Trigger::Stop);
        while ctx.iteration(false) {}
        assert_eq!(ORDER.with(|o| o.borrow().clone()), ["resume queue"]);
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

#[test]
fn expired_token_on_fresh_start_reports_the_refused_track() {
    let ctx = glib::MainContext::new();
    ctx.with_thread_default(|| {
        ORDER.with(|o| o.borrow_mut().clear());
        let m = Machine::new(
            Rc::new(Testbin::default()),
            Hooks {
                on_error: |id, message| {
                    assert_eq!(id, Some("refused"));
                    assert!(message.contains("refused playback"));
                    ORDER.with(|o| o.borrow_mut().push("error"));
                },
                clear_source: || ORDER.with(|o| o.borrow_mut().push("clear")),
                ..Hooks::default()
            },
        );
        m.state.set(State::Preparing);
        let mut track = Track::default();
        track.info.id = "refused".into();
        track_changed().set(Some(TrackChangedEvent { track, skip_history: false }));
        m.fire(Trigger::TokenExpired);
        m.fire(Trigger::Error("stream error".into()));
        while ctx.iteration(false) {}
        assert_eq!(m.state.get(), State::Stopped);
        assert!(track_changed().get().is_none());
        assert_eq!(ORDER.with(|o| o.borrow().clone()), ["clear", "error"]);
    })
    .unwrap();
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

        // Play Next moving the armed track to the user queue repoints the entry, before and after arming.
        let m = Machine::new(Rc::new(Testbin::default()) as Rc<dyn Playbin>, hooks());
        m.begin_gapless(entry("a"));
        m.move_gapless_to_user_queue();
        assert_eq!(m.gapless_entry().map(|e| e.source), Some(GaplessSource::UserQueue));
        m.begin_gapless(entry("a"));
        assert!(m.arm_gapless(TrackManifest::default(), 1.0).unwrap());
        m.move_gapless_to_user_queue();
        assert_eq!(m.gapless_entry().map(|e| e.source), Some(GaplessSource::UserQueue));
        assert!(m.has_armed_gapless());

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
        // No next URI is left behind to play without its queue commit.
        assert!(failing_pb.uri.borrow().is_empty());
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

/// A retry whose cached copy failed reloads once more, from the stream; a retry that fails again stops.
#[test]
fn a_retry_whose_cached_copy_failed_retries_once_more() {
    let ctx = glib::MainContext::new();
    ctx.with_thread_default(|| {
        let m = Machine::new(Rc::new(Testbin::default()), Hooks::default());
        m.state.set(State::PlayingRetrying);
        assert_eq!(m.target(&Trigger::TokenExpired), Some(State::Stopped));
        m.retry_once_more();
        assert_eq!(m.target(&Trigger::TokenExpired), Some(State::PlayingRetrying));
        assert_eq!(m.target(&Trigger::TokenExpired), Some(State::Stopped));
    })
    .unwrap();
}

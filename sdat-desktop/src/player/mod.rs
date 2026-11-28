//! Port of `internal/player`: playbin3 wrapper, queues, controls, gapless
//! playback, `ReplayGain`, stream quality and MPRIS.


mod audio_cache;
pub mod mpris;
pub mod play_next;
pub use tonearm_core::playqueue;
pub mod quality;
pub mod replaygain;
mod gapless_gate;
mod next;
pub mod statemachine;

use anyhow::{anyhow, Result};
use gst::prelude::*;
use std::cell::{Cell, OnceCell};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::model::{PlaybackSource, PlaylistInfo, Track};
use crate::signals::{Signal, State, CONTINUE};
use crate::{api, global_signal, i18n::tr, service, settings};
use playqueue::{DurableQueue, HistoryEntry, HistoryQueue, UserQueue};
pub use audio_cache::{drop_plan as drop_audio_cache_plan, on_limit_changed as on_audio_cache_limit_changed};
pub use quality::audio_stream_quality;
use gapless_gate::GaplessGate;
use next::{commit_gapless, next_track, peek_gapless, pop_next_streamable_from_queue};
use statemachine::{GaplessEntry, GaplessSource, Hooks, Machine, Trigger};
pub use statemachine::{
    playback_quality_changed, playback_state_changed, track_changed, track_repeated, PlaybackState, PlaybackStatus,
};

// ---------------------------------------------------------------- signals

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepeatMode {
    #[default]
    None,
    Queue,
    Track,
}

global_signal!(pub fn repeat_mode_changed() -> State<RepeatMode> = State::new(RepeatMode::None));
global_signal!(pub fn shuffle_state_changed() -> State<bool> = State::new(false));
global_signal!(pub fn seeked() -> Signal<Duration> = Signal::default());
global_signal!(pub fn source_changed() -> State<Option<PlaybackSource>> = State::new(None));
global_signal!(pub fn volume_changed() -> State<f64> = State::new(settings::volume()));
// Toast requests from the player (notifications.OnToast).
global_signal!(pub fn toasts() -> Signal<String> = Signal::default());

global_signal!(pub fn base_queue() -> DurableQueue = DurableQueue::default());
global_signal!(pub fn user_queue() -> UserQueue = UserQueue::default());
global_signal!(pub fn history() -> HistoryQueue = HistoryQueue::default());

thread_local! {
    /// Entries at the front of the user queue that were added with "play next".
    static PLAY_NEXT_PINNED: Cell<usize> = const { Cell::new(0) };
    static PLAY_NEXT_ORDER: std::cell::RefCell<play_next::Sequencer<(Option<u64>, Vec<Track>)>> = std::cell::RefCell::default();
    /// Bumped when the queues are replaced, so pending "play next" fetches are discarded.
    static PLAY_NEXT_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Id of the gapless-prepared track moved from the base queue to the head
    /// of the user queue by "play next", until that entry is consumed.
    /// Bumped when the queues are replaced, so a deferred next / previous
    /// meant for the old queue is dropped.
    static QUEUE_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Set while end-of-queue autoplay is fetching a radio mix.
    static AUTOPLAY_PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Drops pending "play next" requests (e.g. when leaving a Listen Together
/// session, so a guest's request isn't applied to the local queue).
pub fn invalidate_play_next() {
    PLAY_NEXT_GENERATION.with(|g| g.set(g.get() + 1));
    PLAY_NEXT_ORDER.with(|s| *s.borrow_mut() = play_next::Sequencer::default());
}

/// A reserved place in the "play next" order, tied to the queue it was taken for.
#[derive(Debug, Clone, Copy)]
pub struct PlayNextTicket {
    generation: u64,
    seq: u64,
}

/// Reserves the order of a "play next" request when the user makes it.
pub fn play_next_ticket() -> PlayNextTicket {
    PlayNextTicket { generation: PLAY_NEXT_GENERATION.with(Cell::get), seq: PLAY_NEXT_ORDER.with(|s| s.borrow_mut().ticket()) }
}

/// Delivers the tracks for a "play next" request (empty if fetching failed);
/// requests are applied in the order their tickets were taken. Requests taken
/// before the queues were replaced are dropped. A Listen Together guest sends
/// each batch to the host as a Play Next, in the same order. `guest` is the
/// Listen Together session of a guest's request, rechecked on release since
/// control may be revoked while it waits behind an earlier request.
pub fn complete_play_next(ticket: PlayNextTicket, tracks: Vec<Track>, guest: Option<u64>) {
    if ticket.generation != PLAY_NEXT_GENERATION.with(Cell::get) {
        return;
    }
    let batches = PLAY_NEXT_ORDER.with(|s| s.borrow_mut().complete(ticket.seq, vec![(guest, tracks)]));
    let mut queued = false;
    for (guest, b) in batches.into_iter().flatten() {
        if guest.is_some_and(|g| !crate::live::guest_request_still_valid(g)) {
            continue;
        }
        if b.is_empty() || crate::live::user_play_next(&b.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()) {
            continue;
        }
        insert_tracklist_next(b);
        queued = true;
    }
    // Start playback once, and only if no start is already queued or in
    // flight: `next()` only queues the start, so a second call before the
    // first track plays would pop (and lose) another track.
    if queued {
        start_if_idle();
    }
}

/// Starts the next queued track if the player is stopped and nothing is about
/// to start it (a queued/in-flight start or a loading autoplay mix). `next()`
/// only queues the start, so a second call would skip a track.
fn start_if_idle() {
    if !AUTOPLAY_PENDING.with(Cell::get) && with(|p| p.machine.stopped_without_pending_start()).unwrap_or(false) {
        next();
    }
}

/// Fetches the radio mix for a track: its playlist info and all its tracks.
async fn fetch_radio(track_id: String) -> Result<(tonearm_core::model::PlaylistInfo, Vec<Track>)> {
    let radio = api::run(move || service::get_track_radio(&track_id))
        .await
        .inspect_err(|e| log::error!("failed to retrieve mix: {e}"))?;
    let Some(mix) = radio else {
        toasts().notify(&tr("No mix found for the current track"));
        anyhow::bail!("no mix found for the current track");
    };
    let id = mix.id.clone();
    let info = api::run(move || service::get_playlist_info(&id)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Playlist(mix.id)).await?;
    Ok((info, tracks))
}

/// Keeps the "play next" region in step with a drag in the user queue.
pub fn user_queue_moved(src: usize, dst: usize) {
    user_queue().edit(|rows| {
        if src < rows.len() {
            let row = rows.remove(src);
            let dst = dst.min(rows.len());
            rows.insert(dst, row);
            update_pinned(|p| play_next::after_move(p, src, dst));
        }
    });
}

fn update_pinned(f: impl FnOnce(usize) -> usize) {
    PLAY_NEXT_PINNED.with(|c| c.set(f(c.get())));
}

/// Removes the user-queue entry at `index`, keeping "play next" order intact.
pub fn remove_from_user_queue(index: usize) -> tonearm_core::error::CoreResult<()> {
    user_queue().remove_at(index)?;
    update_pinned(|p| play_next::after_remove(p, index));
    Ok(())
}

// ---------------------------------------------------------------- player

struct GuardedPlaybin {
    element: gst::Element,
    gate: Arc<GaplessGate>,
}

impl statemachine::Playbin for GuardedPlaybin {
    fn set_state(&self, state: gst::State) -> Result<()> {
        let _guard = matches!(state, gst::State::Ready | gst::State::Null).then(|| self.gate.enter_blocking_op());
        statemachine::Playbin::set_state(&self.element, state)
    }

    fn query_duration(&self) -> Option<Duration> {
        statemachine::Playbin::query_duration(&self.element)
    }

    fn query_position(&self) -> Option<Duration> {
        statemachine::Playbin::query_position(&self.element)
    }

    fn seek(&self, position: Duration, flags: gst::SeekFlags) -> bool {
        let _guard = flags.contains(gst::SeekFlags::FLUSH).then(|| self.gate.enter_blocking_op());
        statemachine::Playbin::seek(&self.element, position, flags)
    }

    fn set_uri(&self, uri: &str) {
        statemachine::Playbin::set_uri(&self.element, uri);
    }

    fn set_volume(&self, volume: f64) -> Result<()> {
        statemachine::Playbin::set_volume(&self.element, volume)
    }

    fn flush_bus(&self) {
        statemachine::Playbin::flush_bus(&self.element);
    }
}

struct Player {
    playbin: Rc<GuardedPlaybin>,
    machine: Rc<Machine>,
    _bus_watch: gst::bus::BusWatchGuard,
}

thread_local! {
    static PLAYER: OnceCell<Player> = const { OnceCell::new() };
}

fn with<R>(f: impl FnOnce(&Player) -> R) -> Option<R> {
    PLAYER.with(|p| p.get().map(f))
}

fn fire(t: Trigger) {
    if matches!(t, Trigger::PlayTrack(..) | Trigger::Deferred(_)) {
        PLAY_REQUESTS.with(|r| r.set(r.get() + 1));
    }
    fire_uncounted(t);
}

/// For a restart nobody asked for, such as streaming a failed cached copy: what waits on the user's start (a
/// handoff's seek) keeps waiting for it.
fn fire_uncounted(t: Trigger) {
    if matches!(t, Trigger::Stop | Trigger::PlayTrack(..) | Trigger::PlayTrackId(..)) {
        WAITING.with(|w| w.set(false));
    }
    if with(|p| p.machine.fire(t)).is_none() {
        log::warn!("player not initialised, dropping trigger");
    }
}

fn playbin() -> Option<Rc<GuardedPlaybin>> {
    with(|p| Rc::clone(&p.playbin))
}

/// Initialises `GStreamer`, playbin3, the state machine and MPRIS.
/// Must be called once on the GTK main thread.
pub fn init() -> Result<()> {
    if with(|_| ()).is_some() {
        return Ok(());
    }
    gst::init()?;
    let playbin = gst::ElementFactory::make("playbin3").build().map_err(|e| anyhow!("playbin3 unavailable: {e}"))?;
    let gate = Arc::new(GaplessGate::new());
    let guarded = Rc::new(GuardedPlaybin { element: playbin.clone(), gate: Arc::clone(&gate) });

    let pb: Rc<dyn statemachine::Playbin> = Rc::clone(&guarded) as Rc<dyn statemachine::Playbin>;
    let machine = Machine::with_gate(
        pb,
        Arc::clone(&gate),
        Hooks {
            peek_gapless,
            on_gapless_arm: |manifest| replaygain::set_pending_gapless(Some(manifest)),
            on_gapless_promote: |entry| {
                commit_gapless(entry);
                replaygain::promote_gapless();
            },
            on_gapless_cancel: replaygain::clear_pending_gapless,
            on_eos_stopped: || Box::pin(on_eos_stopped()),
            clear_source: || source_changed().set(None),
            on_error: |track, e| {
                if crate::live::playback_failed(track) {
                    toasts().notify(&crate::i18n::trf("Playback failed: %s", &[&e]));
                }
            },
        },
    );

    let bus = playbin.bus().ok_or_else(|| anyhow!("playbin has no bus"))?;
    let bus_watch = bus.add_watch_local(|_, msg| {
        on_bus_message(msg);
        glib::ControlFlow::Continue
    })?;

    playbin.connect_notify(Some("volume"), |pb, _| {
        let v: f64 = pb.property("volume");
        glib::MainContext::default().invoke(move || on_volume_change(v));
    });

    let about_to_finish_gate = Arc::clone(&gate);
    playbin.connect("about-to-finish", false, move |_| {
        let ctx = glib::MainContext::default();
        if ctx.is_owner() {
            if let Some(token) = about_to_finish_gate.begin_nonblocking() {
                fire(Trigger::GaplessHint(Some(token)));
            }
            return None;
        }
        // Block the streaming thread until the next URI is set (gapless) or canceled.
        if about_to_finish_gate.wait(Duration::from_secs(15), |token| {
            ctx.invoke(move || fire(Trigger::GaplessHint(Some(token))));
        }) {
            log::warn!("timed out waiting for the next gapless track");
        }
        None
    });

    if let Some(bin) = playbin.downcast_ref::<gst::Bin>() {
        for element in bin.iterate_recurse().into_iter().flatten() {
            quality::on_deep_element_added(&element);
            audio_cache::on_element_added(&element);
        }
        bin.connect_deep_element_added(|_, _, element| {
            quality::on_deep_element_added(element);
            audio_cache::on_element_added(element);
        });
    }

    replaygain::init(&playbin)?;
    playbin.set_property("volume", settings::volume());

    PLAYER.with(|p| {
        let _ = p.set(Player { playbin: guarded, machine, _bus_watch: bus_watch });
    });

    track_changed().on(|ev| {
        if let Some(ev) = ev
            && !ev.skip_history {
                history().prepend(HistoryEntry { track: ev.track.clone(), occurrence: base_queue().current_occurrence(&ev.track) });
            }
        audio_cache::on_track_changed();
        CONTINUE
    });
    playback_state_changed().on(|s| {
        audio_cache::on_playback_state(s);
        CONTINUE
    });
    audio_cache::init();
    source_changed().on(|_| {
        replaygain::update_album_mode();
        CONTINUE
    });
    Ok(())
}

fn on_bus_message(msg: &gst::Message) {
    use gst::MessageView;
    match msg.view() {
        MessageView::Error(err) => {
            let e = err.error();
            if let Some(id) = cached_copy_failed() {
                fire_uncounted(Trigger::Deferred(Box::new(move || stream_after_cached_copy_failed(&id))));
                return;
            }
            if e.matches(gst::ResourceError::NotAuthorized) || e.matches(gst::ResourceError::NotFound) {
                log::warn!("playback token expired, restarting playback session");
                fire(Trigger::TokenExpired);
                return;
            }
            log::error!("playback failed: {} (debug: {:?})", e, err.debug());
            fire(Trigger::Error(e.to_string()));
        }
        MessageView::Buffering(b) => fire(Trigger::Buffering(b.percent() < 100)),
        MessageView::Tag(t) => quality::on_tags_received(&t.tags()),
        MessageView::StreamStart(_) => fire(Trigger::StreamStart),
        MessageView::Eos(_) => fire(Trigger::Eos),
        _ => {}
    }
}

// A track that has not started yet goes to Stopped on TokenExpired, so it is started again instead.
fn stream_after_cached_copy_failed(id: &str) {
    // A start asked for after the error replaces the failed track; recovering it first would reload the old one. A
    // queued Next or Previous may find no track, so it does not count.
    if with(|p| p.machine.has_queued_stop_or_start()).unwrap_or(false) {
        return;
    }
    let current = track_changed().with(|t| t.as_ref().map(|e| e.track.clone()));
    match current {
        Some(t) if t.id() == id && with(|p| p.machine.preparing()).unwrap_or(false) => {
            // The new start drops the pause, so it loads paused and takes the pause again, as `play_at` starts.
            let paused = with(|p| p.machine.preparing_paused()).unwrap_or(false);
            if paused {
                statemachine::start_paused(id);
            }
            // Its start already went into the history.
            fire_uncounted(Trigger::PlayTrack(t, true));
            if paused {
                fire(Trigger::Pause);
            }
        }
        Some(t) if t.id() == id => {
            // A retry that failed again would stop, but this one streams now.
            if with(|p| p.machine.retrying()).unwrap_or(false) {
                with(|p| p.machine.retry_once_more());
            }
            fire(Trigger::TokenExpired);
        }
        // During a gapless handoff the failed copy is the next track's: the current one restarts and hands off to its
        // stream.
        _ if with(|p| p.machine.gapless_entry()).flatten().is_some_and(|e| e.track.id() == id) => fire(Trigger::TokenExpired),
        // The error of a stream that a newer start replaced.
        _ => {}
    }
}

fn cached_copy_failed() -> Option<String> {
    let uri = playbin().and_then(|p| p.element.property::<Option<String>>("current-uri"))?;
    audio_cache::drop_failed_copy(&uri)
}

fn on_volume_change(v: f64) {
    #[expect(clippy::float_cmp, reason = "exact echo of a value we published ourselves")]
    let unchanged = v == volume_changed().get();
    if unchanged {
        return;
    }
    settings::set_volume(v);
    volume_changed().set(v);
}

async fn on_eos_stopped() {
    let stop = || {
        source_changed().set(None);
        statemachine::on_stopped_stop();
    };
    if crate::live::is_guest() {
        stop();
        return;
    }
    if with(|p| p.machine.has_queued_stop_or_start()).unwrap_or(false) {
        return;
    }
    if list_pending() {
        // The loaded part ran out before the next page arrived: wait for it
        // instead of stopping, starting a radio or repeating.
        log::info!("queue ran dry while the list is still loading; waiting for the next page");
        WAITING.with(|w| w.set(true));
        feed(true);
        return;
    }
    // A failed gapless prefetch ends the current track with tracks still
    // queued: carry on with them instead of stopping or starting a radio,
    // unless a Stop or another start is already queued.
    if next_track(true).0.is_some() {
        // A pending start (maybe a new list behind its own Stop) runs as queued.
        if with(|p| p.machine.start_pending()).unwrap_or(true) {
            return;
        }
        if with(|p| p.machine.has_queued_stop()).unwrap_or(false) {
            stop();
        } else {
            next_now();
        }
        return;
    }
    if !settings::allow_autoplay() {
        stop();
        return;
    }
    let Some(id) = current_track_id() else {
        stop();
        return;
    };
    // Fetch the whole mix before touching the queues; "play next" requests
    // that land meanwhile are queued but not started (see `complete_play_next`).
    let list_generation = LIST_GENERATION.with(Cell::get);
    AUTOPLAY_PENDING.with(|a| a.set(true));
    let mix = fetch_radio(id).await;
    AUTOPLAY_PENDING.with(|a| a.set(false));
    // The user acted while the mix was loading: a new list or a queued track
    // start wins over autoplay as is.
    if LIST_GENERATION.with(Cell::get) != list_generation
        || !with(|p| p.machine.stopped_without_pending_start()).unwrap_or(true)
    {
        return;
    }
    // A Stop queued behind this handler is ignored once already stopped, so do
    // the stopped-track cleanup here instead of starting the mix.
    if with(|p| p.machine.has_queued_stop()).unwrap_or(false) {
        stop();
        return;
    }
    // Tracks queued while the mix was loading win over autoplay.
    if !user_queue().is_empty() {
        start_if_idle();
        // Fall back to the mix if nothing playable was queued (no start pending).
        if !with(|p| p.machine.stopped_without_pending_start()).unwrap_or(true) {
            return;
        }
    }
    let result = mix.and_then(|(info, tracks)| {
        // An automatic transition: keep "play next" requests still fetching so
        // they land in the radio queue instead of being discarded.
        let generation = PLAY_NEXT_GENERATION.with(Cell::get);
        let order = PLAY_NEXT_ORDER.with(|s| std::mem::take(&mut *s.borrow_mut()));
        clear_queues();
        stop();
        let started = play_tracks(tracks, ListStart::InOrder(1));
        PLAY_NEXT_GENERATION.with(|g| g.set(generation));
        PLAY_NEXT_ORDER.with(|s| *s.borrow_mut() = order);
        started?;
        source_changed().set(Some(PlaybackSource::Playlist(info)));
        Ok(())
    });
    if let Err(e) = result {
        log::info!("failed to play track radio: {e:#}");
        stop();
    }
}

pub fn current_track_id() -> Option<String> {
    track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string()))
}

// ---------------------------------------------------------------- queue.go

fn clear_queues() {
    QUEUE_GENERATION.with(|g| g.set(g.get() + 1));
    user_queue().clear();
    update_pinned(|_| 0);
    // A guest's pending "play next" requests belong to the session and go to
    // the host, so keep them (and their order) when the guest follows a host
    // track change; otherwise they target the replaced queue and are dropped.
    if !crate::live::is_guest() {
        PLAY_NEXT_GENERATION.with(|g| g.set(g.get() + 1));
        PLAY_NEXT_ORDER.with(|s| *s.borrow_mut() = play_next::Sequencer::default());
    }
    base_queue().clear();
}

fn pop_user_queue() -> Option<Track> {
    let entry = user_queue().pop()?;
    update_pinned(|p| play_next::after_remove(p, 0));
    base_queue().select_history_occurrence(entry.occurrence.as_ref());
    Some(entry.track)
}

fn track_matches(a: &Track, b: &Track) -> bool {
    a.id() == b.id() && a.playlist_item_id == b.playlist_item_id
}

pub async fn add_track_to_user_queue(track_id: &str) -> Result<()> {
    let id = track_id.to_string();
    let track = api::run(move || service::get_track(&id)).await?;
    user_queue().append(track);
    start_if_idle();
    Ok(())
}

/// Queues `tracks`, in order, right after the current one (and after any
/// earlier "play next" tracks). The caller starts playback if needed.
fn insert_tracklist_next(tracks: Vec<Track>) {
    if insert_tracklist_next_after(tracks, with(|p| p.machine.gapless_entry()).flatten()) {
        with(|p| p.machine.move_gapless_to_user_queue());
    }
}

/// Returns whether the armed base-queue track moved to the user queue, which
/// the machine's armed entry must then follow.
fn insert_tracklist_next_after(tracks: Vec<Track>, armed: Option<GaplessEntry>) -> bool {
    if tracks.is_empty() {
        return false;
    }
    let mut moved = false;
    // Keep an already-chosen gapless track ahead of Play Next entries when it
    // came from the queues so the queue view matches playback order.
    let floor = match armed {
        // The pick is the first streamable row, which can sit behind unstreamable ones.
        Some(GaplessEntry { source: GaplessSource::UserQueue, track: armed, .. }) => user_queue().entries().with(|rows| {
            next::find_streamable_index(rows, None).filter(|&i| track_matches(&rows[i], &armed)).map_or(0, |i| i + 1)
        }),
        Some(GaplessEntry { source: GaplessSource::BaseQueue { rewound: false }, track: armed, .. }) => {
            let next_in_base = user_queue().is_empty()
                && next::first_streamable(base_queue().entries()).is_some_and(|t| track_matches(&t, &armed));
            if next_in_base {
                // The unstreamable rows ahead of it would be skipped at the commit anyway.
                next::drain_unstreamable_base_prefix();
            }
            if next_in_base && let Some(head) = base_queue().take_for_gapless() {
                user_queue().prepend(head);
                moved = true;
                1
            } else {
                0
            }
        }
        _ => 0,
    };
    user_queue().edit(|entries| {
        let pinned = PLAY_NEXT_PINNED.with(Cell::get);
        let pinned = play_next::insert(entries, pinned, floor, tracks.into_iter().map(HistoryEntry::from).collect());
        update_pinned(|_| pinned);
    });
    moved
}

pub fn add_tracklist_to_user_queue(tracks: Vec<Track>) {
    let any = !tracks.is_empty();
    for t in tracks {
        user_queue().append(t);
    }
    if any {
        start_if_idle();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    User,
    Base,
}

/// Skips `to` entries of a queue (adding them to the history) and plays the next.
pub fn skip_through_queue(kind: QueueKind, to: usize) {
    let res = match kind {
        QueueKind::User => user_queue().skip(to).inspect(|_| update_pinned(|p| play_next::after_skip(p, to))),
        QueueKind::Base => base_queue().skip(to),
    };
    match res {
        Ok(skipped) => {
            for t in skipped {
                history().prepend(t);
            }
            let next = match kind {
                QueueKind::User => pop_user_queue(),
                QueueKind::Base => base_queue().pop(),
            };
            match next {
                Some(t) => fire(Trigger::PlayTrack(t, false)),
                None => log::error!("failed to start playback after skipping through queue"),
            }
        }
        Err(e) => log::error!("failed to skip through queue: {e}"),
    }
}

// ---------------------------------------------------------------- controls.go

pub fn cycle_repeat_mode() {
    set_repeat_mode(match repeat_mode_changed().get() {
        RepeatMode::None => RepeatMode::Queue,
        RepeatMode::Queue => RepeatMode::Track,
        RepeatMode::Track => RepeatMode::None,
    });
}

/// Plays the next track. While a start is still loading, waits for it, so a
/// quick second press skips that track instead of dropping the next one.
pub fn next() {
    defer_while_starting(next_now);
}

/// Runs `action` now, or after the track starts that are queued or loading.
fn defer_while_starting(action: fn()) {
    if with(|p| p.machine.start_pending()).unwrap_or(false) {
        let generation = QUEUE_GENERATION.with(Cell::get);
        fire(Trigger::Deferred(Box::new(move || {
            if QUEUE_GENERATION.with(Cell::get) == generation {
                action();
            }
        })));
    } else {
        action();
    }
}

fn next_now() {
    match next_track(false).0 {
        Some(t) => fire(Trigger::PlayTrack(t, false)),
        None => log::error!("failed to get next track: queue is empty"),
    }
}

/// Changes with every pause.
pub fn pauses() -> u64 {
    PAUSES.with(std::cell::Cell::get)
}

/// Changes as soon as a track start is asked for, unlike `track_changed`,
/// which fires once its stream has loaded.
pub fn play_requests() -> u64 {
    PLAY_REQUESTS.with(std::cell::Cell::get)
}

pub fn pause() {
    PAUSES.with(|p| p.set(p.get() + 1));
    fire(Trigger::Pause);
}

pub fn play() {
    PLAYS.with(|p| p.set(p.get() + 1));
    fire(Trigger::Resume);
}

/// Changes with every play command (a resume of the paused track too).
pub fn plays() -> u64 {
    PLAYS.with(std::cell::Cell::get)
}

pub fn play_pause() {
    match playback_state_changed().get().status {
        PlaybackStatus::Playing => pause(),
        PlaybackStatus::Paused | PlaybackStatus::Stopped => play(),
    }
}

/// Goes back a track (or to the start of this one); deferred like [`next`].
pub fn previous() {
    defer_while_starting(previous_now);
}

fn previous_now() {
    let current = (playback_state_changed().get().status != PlaybackStatus::Stopped)
        .then(|| track_changed().with(|t| t.as_ref().map(|e| e.track.clone())))
        .flatten();
    if current.is_some()
        && (repeat_mode_changed().get() == RepeatMode::Track
            || playbin().and_then(|p| statemachine::Playbin::query_position(&*p)).is_some_and(|p| p > Duration::from_secs(5)))
    {
        seek_to_position(Duration::ZERO, false);
        return;
    }
    let Some(prev) = history().pop_previous(current.as_ref()) else {
        if current.is_some() {
            seek_to_position(Duration::ZERO, false);
        }
        return;
    };
    if let Some(cur) = current {
        let occurrence = base_queue().current_occurrence(&cur);
        user_queue().prepend(HistoryEntry { track: cur, occurrence });
        // The track we came back from plays next, ahead of "play next" tracks.
        update_pinned(|p| p + 1);
    }
    base_queue().select_history_occurrence(prev.occurrence.as_ref());
    fire(Trigger::PlayTrack(prev.track, false));
}

pub fn seek_to_percent(percent: f64) {
    let percent = percent.clamp(0.0, 100.0);
    let d = playback_state_changed().get().duration;
    seek_to_position(d.mul_f64(percent / 100.0), false);
}

pub fn seek_to_position(position: Duration, precise: bool) -> bool {
    try_seek(position, precise)
}

/// `false` when the pipeline refused the seek.
fn try_seek(position: Duration, precise: bool) -> bool {
    playbin().is_some_and(|p| seek_on(&*p, position, precise))
}

fn seek_on(playbin: &dyn statemachine::Playbin, position: Duration, precise: bool) -> bool {
    let flags = if precise {
        gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE
    } else {
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT
    };
    // Subscribers (MPRIS `Seeked`, the Listen Together host anchor) take
    // `position` as reached, so a seek the pipeline refused must not notify.
    let ok = playbin.seek(position, flags);
    if ok {
        playback_state_changed().notify(|mut s| {
            s.position = position;
            s
        });
        seeked().notify(&position);
    }
    ok
}

pub fn set_volume(volume: f64) {
    let volume = if volume < 0.0 { 0.0 } else { volume };
    if volume > 1.0 {
        log::warn!("Volume is higher than 1. This will cause overdrive to the speakers.");
    }
    if let Some(p) = playbin() {
        p.element.set_property("volume", volume);
    }
}

pub fn stop() {
    source_changed().set(None);
    fire(Trigger::Stop);
}

/// Stops playback and runs `then` once the pipeline let go of its stream.
pub fn stop_then(then: impl FnOnce() + 'static) {
    stop();
    fire_uncounted(Trigger::AfterStop(Box::new(then)));
}

/// Call on quit. The player lives in a thread-local, dropped while the process
/// exits; disposing a playbin that is not in NULL there can hang forever.
pub fn shutdown() {
    if let Some(p) = playbin()
        && let Err(e) = statemachine::Playbin::set_state(&*p, gst::State::Null)
    {
        log::warn!("failed to stop the player on quit error={e}");
    }
}

pub fn set_repeat_mode(m: RepeatMode) {
    if m != repeat_mode_changed().get() {
        repeat_mode_changed().set(m);
    }
}

pub fn set_shuffle(shuffle: bool, current_track_id: &str) {
    if shuffle {
        base_queue().shuffle(current_track_id, &user_queue().source_occurrences());
    } else {
        base_queue().restore();
    }
    shuffle_state_changed().set(shuffle);
}

pub fn toggle_shuffle() {
    let cur = current_track_id().unwrap_or_default();
    set_shuffle(!base_queue().is_shuffled(), &cur);
}

// ---------------------------------------------------------------- playback.go

pub async fn play_track_id(track_id: &str) -> Result<()> {
    let id = track_id.to_string();
    let t = api::run(move || service::get_track(&id)).await?;
    play_track(t);
    Ok(())
}

pub fn play_track(track: Track) {
    bump_list_generation();
    clear_queues();
    stop();
    source_changed().set(Some(PlaybackSource::Track(Box::new(track.clone()))));
    fire(Trigger::PlayTrack(track, false));
}

#[expect(clippy::unused_async, reason = "callers await it like the other playback entry points; mirrors Go's PlayTracklist")]
pub async fn play_tracklist(source: PlaybackSource, tracks: Vec<Track>, start: ListStart) -> Result<()> {
    play_tracks(tracks, start)?;
    source_changed().set(Some(source));
    Ok(())
}

pub async fn play_album(album_id: &str, start: ListStart) -> Result<()> {
    let id = album_id.to_string();
    let i = id.clone();
    let info = api::run(move || service::get_album_info(&i)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Album(id)).await?;
    clear_queues();
    stop();
    play_tracks(tracks, start)?;
    source_changed().set(Some(PlaybackSource::Album(info)));
    Ok(())
}

pub async fn play_artist_top_songs(artist_id: &str, start: ListStart) -> Result<()> {
    let id = artist_id.to_string();
    let (info, tracks) = api::run(move || {
        let info = service::get_artist_info(&id)?;
        let page = api::artist_page(&id)?;
        let mut tracks = vec![];
        if let Some(module) = page.items.iter().find(|i| i.module_id == "ARTIST_TOP_TRACKS") {
            for item in &module.items {
                if let api::legacy::ItemData::Track(t) = &item.data {
                    match service::get_track(&t.id.to_string()) {
                        Ok(track) => tracks.push(track),
                        Err(e) => log::error!("error while resolving Top Track item {}: {e}", t.id),
                    }
                }
            }
        }
        Ok::<_, anyhow::Error>((info, tracks))
    })
    .await?;
    clear_queues();
    stop();
    play_tracks(tracks, start)?;
    source_changed().set(Some(PlaybackSource::Artist(info)));
    Ok(())
}

pub async fn play_playlist(playlist_id: &str, shuffle: bool) -> Result<()> {
    let id = playlist_id.to_string();
    let i = id.clone();
    let info = api::run(move || service::get_playlist_info(&i)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Playlist(id)).await?;
    let start = if shuffle { ListStart::Shuffle } else { ListStart::InOrder(0) };
    play_tracklist(PlaybackSource::Playlist(info), tracks, start).await
}

pub async fn play_playlist_track(playlist_id: &str, clicked: &Track, position: usize) -> Result<bool> {
    let id = playlist_id.to_string();
    let i = id.clone();
    let info = api::run(move || service::get_playlist_info(&i)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Playlist(id)).await?;
    play_playlist_selection(info, tracks, clicked, position).await
}

async fn play_playlist_selection(info: PlaylistInfo, tracks: Vec<Track>, clicked: &Track, position: usize) -> Result<bool> {
    let Some(start) = playlist_start(&tracks, clicked, position) else {
        return Ok(false);
    };
    play_tracklist(PlaybackSource::Playlist(info), tracks, ListStart::Selected(start)).await?;
    Ok(true)
}

fn playlist_start(tracks: &[Track], clicked: &Track, position: usize) -> Option<usize> {
    let matches = |track: &Track| {
        track.id() == clicked.id()
            && clicked.playlist_item_id.as_ref().is_none_or(|id| track.playlist_item_id.as_ref() == Some(id))
    };
    let start = if tracks.get(position).is_some_and(matches) {
        Some(position)
    } else {
        let mut candidates = tracks.iter().enumerate().filter(|(_, track)| matches(track));
        let (index, _) = candidates.next()?;
        if candidates.next().is_some() {
            return None;
        }
        Some(index)
    };
    start.filter(|&index| tracks[index].info.streamable)
}

#[cfg(test)]
mod playlist_tests {
    use super::*;

    fn track(id: &str, item: Option<&str>) -> Track {
        let mut track = Track::default();
        track.info.id = id.into();
        track.info.streamable = true;
        track.playlist_item_id = item.map(str::to_owned);
        track
    }

    #[test]
    fn clicking_prepared_user_queue_head_preserves_its_base_occurrence() {
        base_queue().set(vec![track("A", Some("prepared")), track("A", Some("later")), track("B", None)]);
        let prepared = base_queue().take_for_gapless().unwrap();
        user_queue().prepend(prepared);
        skip_through_queue(QueueKind::User, 0);
        set_shuffle(true, "A");
        base_queue().entries().with(|tracks| {
            assert_eq!(tracks.len(), 2);
            assert!(tracks.iter().any(|t| t.playlist_item_id.as_deref() == Some("later")));
            assert!(!tracks.iter().any(|t| t.playlist_item_id.as_deref() == Some("prepared")));
        });
    }

    #[test]
    fn consuming_another_user_track_preserves_the_moved_gapless_entry() {
        base_queue().set(vec![track("A", Some("prepared")), track("B", None)]);
        let prepared = base_queue().take_for_gapless().unwrap();
        user_queue().prepend(prepared);
        user_queue().append(track("A", None));
        user_queue_moved(0, 1);
        assert!(pop_user_queue().unwrap().playlist_item_id.is_none());
        set_shuffle(true, "A");
        base_queue().entries().with(|tracks| {
            assert_eq!(tracks.len(), 1);
            assert_eq!(tracks[0].id(), "B");
        });
    }

    #[test]
    fn previous_then_next_preserves_the_requeued_tracks_occurrence() {
        let q = base_queue();
        q.set(vec![track("A", None), track("B", Some("first")), track("C", None), track("B", Some("later"))]);
        let a = q.pop().unwrap();
        history().prepend(HistoryEntry { occurrence: q.current_occurrence(&a), track: a });
        let b = q.pop().unwrap();
        history().prepend(HistoryEntry { occurrence: q.current_occurrence(&b), track: b.clone() });
        track_changed().set(Some(statemachine::TrackChangedEvent { track: b, skip_history: false }));
        playback_state_changed().set(PlaybackState { status: PlaybackStatus::Playing, ..Default::default() });
        user_queue().prepend(q.take_for_gapless().unwrap());

        previous_now();
        set_shuffle(true, "A");
        assert!(q.entries().with(|v| v.len() == 1 && v[0].playlist_item_id.as_deref() == Some("later")));
        assert_eq!(pop_user_queue().unwrap().playlist_item_id.as_deref(), Some("first"));
        set_shuffle(true, "B");
        q.entries().with(|v| {
            assert_eq!(v.len(), 2);
            assert!(v.iter().any(|t| t.playlist_item_id.as_deref() == Some("later")));
            assert!(v.iter().any(|t| t.id() == "A"));
        });
        track_changed().set(None);
        playback_state_changed().set(PlaybackState::default());
    }

    #[test]
    fn repeat_queue_stops_after_wrapping_an_unplayable_list() {
        set_repeat_mode(RepeatMode::Queue);
        let mut unavailable = track("A", None);
        unavailable.info.streamable = false;
        base_queue().set(vec![unavailable]);
        assert!(next_track(false).0.is_none());
        set_repeat_mode(RepeatMode::None);
    }

    #[test]
    fn starting_a_list_under_repeat_one_plays_its_selected_track() -> Result<()> {
        set_repeat_mode(RepeatMode::Track);
        set_shuffle(true, "");
        let tracks = vec![track("A", None), track("B", None)];
        let started = play_tracks(tracks, ListStart::Selected(1))?;
        assert_eq!(started.id(), "B");
        assert!(shuffle_state_changed().get());
        assert_eq!(repeat_mode_changed().get(), RepeatMode::Track);
        set_repeat_mode(RepeatMode::None);
        Ok(())
    }

    #[test]
    fn reordered_playlist_starts_the_clicked_track() {
        let clicked = track("B", Some("b"));
        let fresh = [track("C", Some("c")), track("A", Some("a")), clicked.clone()];
        let started = playlist_start(&fresh, &clicked, 1).and_then(|index| fresh.get(index));
        assert_eq!(started.map(Track::id), Some("B"));
    }

    #[test]
    fn clicking_playlist_row_replaces_the_pending_queue() -> Result<()> {
        let info = PlaylistInfo::default();
        let clicked = track("B", Some("b"));
        let tracks = vec![track("A", Some("a")), clicked.clone(), track("C", Some("c")), track("B", Some("later"))];
        user_queue().append(track("queued", None));

        assert!(glib::MainContext::new().block_on(play_playlist_selection(info, tracks, &clicked, 0))?);
        assert!(user_queue().is_empty());
        base_queue().entries().with(|tracks| {
            let items: Vec<_> = tracks.iter().map(|track| track.playlist_item_id.as_deref()).collect();
            assert_eq!(items, [Some("c"), Some("later")]);
        });
        Ok(())
    }

    #[test]
    fn clicking_playlist_row_keeps_shuffle_and_selects_the_exact_occurrence() -> Result<()> {
        set_shuffle(true, "");
        let first = track("A", Some("first"));
        let clicked = track("A", Some("clicked"));
        let tracks = vec![first, track("B", None), clicked.clone(), track("C", None)];

        assert!(glib::MainContext::new().block_on(play_playlist_selection(PlaylistInfo::default(), tracks, &clicked, 0))?);
        assert!(shuffle_state_changed().get());
        assert!(base_queue().is_shuffled());
        set_shuffle(false, clicked.id());
        base_queue().entries().with(|tracks| {
            assert_eq!(tracks.iter().map(Track::id).collect::<Vec<_>>(), ["A", "B", "C"]);
            assert_eq!(tracks[0].playlist_item_id.as_deref(), Some("first"));
        });
        Ok(())
    }

    #[test]
    fn clicking_a_paginated_row_keeps_shuffle_for_late_pages() {
        set_shuffle(true, "");
        let clicked = track("C", None);
        let first_page = vec![track("A", None), track("B", None), clicked.clone()];
        let generation = play_list_from(PlaybackSource::MyTracks, first_page, &clicked, 2).unwrap();
        assert!(base_queue().is_shuffled());
        assert!(base_queue().current_occurrence(&clicked).is_some());
        let next_page = vec![track("D", None), track("E", None)];
        assert!(extend_list(generation, next_page, false));

        set_shuffle(false, clicked.id());
        base_queue().entries().with(|tracks| {
            assert_eq!(tracks.iter().map(Track::id).collect::<Vec<_>>(), ["A", "B", "D", "E"]);
        });
        base_queue().rewind();
        base_queue().entries().with(|tracks| {
            assert_eq!(tracks.iter().map(Track::id).collect::<Vec<_>>(), ["A", "B", "C", "D", "E"]);
        });
    }

    #[test]
    fn reordered_playlist_keeps_the_selected_occurrence() {
        let clicked = track("B", Some("b2"));
        let fresh = [clicked.clone(), track("B", Some("b1"))];
        let started = playlist_start(&fresh, &clicked, 1).and_then(|index| fresh.get(index));
        assert_eq!(started.and_then(|track| track.playlist_item_id.as_deref()), Some("b2"));
    }

    #[test]
    fn removed_occurrence_does_not_start_playback() {
        let clicked = track("B", Some("b2"));
        let fresh = [track("B", Some("b1")), track("C", Some("c"))];
        assert_eq!(playlist_start(&fresh, &clicked, 1), None);
    }

    #[test]
    fn newly_unavailable_track_does_not_start_playback() {
        let clicked = track("B", Some("b"));
        let mut unavailable = clicked.clone();
        unavailable.info.streamable = false;
        let fresh = [unavailable, track("C", Some("c"))];
        assert_eq!(playlist_start(&fresh, &clicked, 0), None);
    }

    #[test]
    fn rows_without_occurrence_ids_match_by_track() {
        let clicked = track("B", None);
        let fresh = [clicked.clone(), track("C", None), clicked.clone()];
        assert_eq!(playlist_start(&fresh, &clicked, 2), Some(2));
        assert_eq!(playlist_start(&fresh, &clicked, 1), None);
        assert_eq!(playlist_start(&fresh[..2], &clicked, 1), Some(0));
        assert_eq!(playlist_start(&fresh, &track("A", None), 1), None);
    }

    #[test]
    fn previous_now_under_repeat_track_does_not_duplicate_queue_or_history() {
        clear_queues();
        while history().pop().is_some() {}
        set_repeat_mode(RepeatMode::None);

        let a = track("A", None);
        let b = track("B", None);
        history().prepend(HistoryEntry::from(a));
        history().prepend(HistoryEntry::from(b.clone()));
        track_changed().set(Some(statemachine::TrackChangedEvent { track: b, skip_history: false }));
        playback_state_changed().set(PlaybackState { status: PlaybackStatus::Playing, ..Default::default() });

        set_repeat_mode(RepeatMode::Track);
        previous_now();

        assert!(user_queue().is_empty(), "previous_now must not enqueue duplicates under RepeatMode::Track");
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 0);
        // `HistoryQueue` holds the current track (`B`) as pending until another track starts;
        // prepending `C` flushes `B` ahead of `A` so popping verifies both remained intact.
        history().prepend(HistoryEntry::from(track("C", None)));
        assert_eq!(history().pop().map(|e| e.track.id().to_string()).as_deref(), Some("B"));
        assert_eq!(history().pop().map(|e| e.track.id().to_string()).as_deref(), Some("A"));
        assert!(history().is_empty());

        set_repeat_mode(RepeatMode::None);
        track_changed().set(None);
        playback_state_changed().set(PlaybackState::default());
    }

    #[test]
    fn seek_while_paused_updates_playback_state_position() {
        let pb = statemachine::tests::Testbin::default();
        playback_state_changed().set(PlaybackState {
            status: PlaybackStatus::Paused,
            position: Duration::from_secs(5),
            duration: Duration::from_secs(180),
            ..Default::default()
        });
        let seen = Rc::new(Cell::new(None));
        let s = Rc::clone(&seen);
        let sub = seeked().on(move |pos| {
            s.set(Some(*pos));
            CONTINUE
        });

        assert!(seek_on(&pb, Duration::from_secs(42), false));
        assert_eq!(playback_state_changed().get().position, Duration::from_secs(42));
        assert_eq!(seen.get(), Some(Duration::from_secs(42)));

        let failing = statemachine::tests::Testbin { position_should_error: true, ..Default::default() };
        seen.set(None);
        assert!(!seek_on(&failing, Duration::from_secs(90), true));
        assert_eq!(playback_state_changed().get().position, Duration::from_secs(42));
        assert_eq!(seen.get(), None);
        seeked().unsubscribe(sub);
    }

    #[test]
    fn guest_next_track_and_eos_do_not_advance_or_autoplay() {
        clear_queues();
        set_repeat_mode(RepeatMode::Track);
        let current = track("A", None);
        base_queue().set(vec![track("B", None)]);
        user_queue().append(track("U", None));
        track_changed().set(Some(statemachine::TrackChangedEvent { track: current.clone(), skip_history: false }));
        source_changed().set(Some(PlaybackSource::Track(Box::new(current))));

        crate::live::set_test_guest(Some(true));
        assert!(next_track(true).0.is_none());
        assert!(next_track(false).0.is_none());
        assert!(peek_gapless().is_none());

        glib::MainContext::new().block_on(on_eos_stopped());
        assert!(track_changed().get().is_none());
        assert!(source_changed().get().is_none());
        assert_eq!(user_queue().entries().with(Vec::len), 1);
        assert_eq!(base_queue().entries().with(Vec::len), 1);

        crate::live::set_test_guest(None);
        set_repeat_mode(RepeatMode::None);
        clear_queues();
    }
}

thread_local! {
    /// Bumped whenever the base queue is replaced, so background fetches for
    /// an older list can tell they are stale.
    static LIST_GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Bumped when a track start is asked for, before its stream loads.
    static PLAY_REQUESTS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Bumped by every play command.
    static PLAYS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Bumped by every pause, so a delayed start knows a later one came.
    static PAUSES: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// Plays `tracks` starting at the clicked row, honoring the shuffle setting.
/// In order, earlier rows go to history. Returns the queue generation for
/// [`extend_list`], or `None` if nothing could be played.
pub fn play_list_from(source: PlaybackSource, tracks: Vec<Track>, clicked: &Track, pos: usize) -> Option<u64> {
    let start = crate::model::queue_start(&tracks, clicked.id(), pos);
    match play_tracks(tracks, ListStart::Selected(start)) {
        Ok(_) => {
            source_changed().set(Some(source));
            Some(LIST_GENERATION.with(std::cell::Cell::get))
        }
        Err(e) => {
            toasts().notify(&tr("An error occurred while playing the track"));
            log::error!("playing the track list failed error={e:#}");
            None
        }
    }
}

/// Fetches the next page of the playing list; gets a callback to call with
/// `true` when more may follow, `false` at the end or on error.
pub type ListFeeder = Rc<dyn Fn(Rc<dyn Fn(bool)>)>;

/// Upcoming tracks left in the queue at which the next page is requested.
pub const FEED_THRESHOLD: usize = 10;

thread_local! {
    static FEEDER: std::cell::RefCell<Option<(u64, ListFeeder)>> = const { std::cell::RefCell::new(None) };
    static FEEDING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Playback reached the end of the loaded tracks and waits for a page.
    static WAITING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

// Tracks of the playing list that are not loaded yet (known from the list's
// total size); the queue view shows them as placeholder rows.
global_signal!(pub fn list_unloaded() -> State<usize> = State::new(0));

thread_local! {
    static LIST_TOTAL: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// How many of `total` list tracks are still to load once `loaded` are known.
pub fn unloaded_count(total: Option<usize>, loaded: usize, pending: bool) -> usize {
    match total {
        Some(t) if pending => t.saturating_sub(loaded),
        _ => 0,
    }
}

fn update_unloaded() {
    let n = unloaded_count(LIST_TOTAL.with(std::cell::Cell::get), base_queue().source_len(), list_pending());
    if list_unloaded().get() != n {
        list_unloaded().set(n);
    }
}

/// Records the full size of the list started at `generation` (e.g. the
/// number of favourite tracks) so the queue can show all of it.
pub fn set_list_total(generation: u64, total: usize) {
    if LIST_GENERATION.with(std::cell::Cell::get) == generation {
        LIST_TOTAL.with(|t| t.set(Some(total)));
        update_unloaded();
    }
}

/// Whether the playing list has pages that are not loaded yet.
fn list_pending() -> bool {
    let generation = LIST_GENERATION.with(std::cell::Cell::get);
    FEEDER.with(|f| f.borrow().as_ref().is_some_and(|(g, _)| *g == generation))
}

/// Whether to ask for another page: only when few tracks remain upcoming.
pub fn needs_more(upcoming: usize) -> bool {
    upcoming <= FEED_THRESHOLD
}

/// Lets the list started at `generation` grow lazily: `feeder` is called
/// (one request at a time) whenever the queue runs low.
pub fn set_list_feeder(generation: u64, feeder: ListFeeder) {
    static HOOKED: std::sync::Once = std::sync::Once::new();
    HOOKED.call_once(|| {
        base_queue().entries().on(|v| {
            if needs_more(v.len()) {
                glib::idle_add_local_once(maybe_feed);
            }
            CONTINUE
        });
    });
    if LIST_GENERATION.with(std::cell::Cell::get) != generation {
        return;
    }
    FEEDER.with(|f| *f.borrow_mut() = Some((generation, feeder)));
    maybe_feed();
}

fn maybe_feed() {
    feed(false);
}

/// Requests the next page (one at a time); `force` ignores the threshold
/// (playback is waiting, or the queue view shows unloaded rows).
pub fn feed(force: bool) {
    let generation = LIST_GENERATION.with(std::cell::Cell::get);
    let Some(feeder) = FEEDER.with(|f| f.borrow().as_ref().filter(|(g, _)| *g == generation).map(|(_, f)| Rc::clone(f))) else { return };
    if FEEDING.with(std::cell::Cell::get) || !(force || needs_more(base_queue().entries().with(Vec::len))) {
        return;
    }
    FEEDING.with(|f| f.set(true));
    feeder(Rc::new(move |more| {
        if LIST_GENERATION.with(std::cell::Cell::get) != generation {
            return;
        }
        FEEDING.with(|f| f.set(false));
        if !more {
            FEEDER.with(|f| {
                if f.borrow().as_ref().is_some_and(|(g, _)| *g == generation) {
                    *f.borrow_mut() = None;
                }
            });
        }
        update_unloaded();
        if !WAITING.with(std::cell::Cell::get) {
            return;
        }
        if !base_queue().is_empty() {
            WAITING.with(|w| w.set(false));
            next();
        } else if more {
            feed(true); // page was empty or all duplicates: keep going
        } else {
            // The list ended (or failed) while waiting: finish like a normal end.
            glib::spawn_future_local(async move {
                if list_generation() == generation && WAITING.with(|w| w.replace(false)) {
                    on_eos_stopped().await;
                }
            });
        }
    }));
}

/// Appends late-loaded tracks to the list started by [`play_list_from`].
/// An overlapping feed skips IDs already in the source.
/// Returns `false` (and does nothing) once another list replaced it.
pub fn extend_list(generation: u64, tracks: Vec<Track>, overlaps: bool) -> bool {
    if LIST_GENERATION.with(std::cell::Cell::get) != generation {
        return false;
    }
    if overlaps {
        base_queue().extend(&tracks);
    } else {
        base_queue().extend_occurrences(tracks);
    }
    update_unloaded();
    true
}

/// Makes `tracks` the rest of the list after the current track, as if the
/// list had started with them: queue repeat and shuffle cover all of it.
pub fn set_list_rest(tracks: Vec<Track>) {
    let Some(current) = track_changed().with(|t| t.as_ref().map(|e| e.track.clone())) else { return };
    let id = current.id().to_string();
    base_queue().set_rest(current, tracks);
    if base_queue().is_shuffled() {
        base_queue().shuffle(&id, &user_queue().source_occurrences());
    }
}

/// Changes with every new list started.
pub fn list_generation() -> u64 {
    LIST_GENERATION.with(std::cell::Cell::get)
}

fn bump_list_generation() {
    LIST_GENERATION.with(|g| g.set(g.get() + 1));
    FEEDER.with(|f| *f.borrow_mut() = None);
    FEEDING.with(|f| f.set(false));
    WAITING.with(|w| w.set(false));
    LIST_TOTAL.with(|t| t.set(None));
    update_unloaded();
}

/// The order and first track to use when starting a list.
#[derive(Debug, Clone, Copy)]
pub enum ListStart {
    InOrder(usize),
    Shuffle,
    /// Play this row first and honor the current shuffle setting.
    Selected(usize),
}

fn play_tracks(tracks: Vec<Track>, start: ListStart) -> Result<Track> {
    let shuffle_after_start = matches!(start, ListStart::Selected(_)) && shuffle_state_changed().get();
    bump_list_generation();
    clear_queues();
    stop();
    base_queue().set(tracks);
    match start {
        ListStart::Shuffle => set_shuffle(true, ""),
        ListStart::Selected(position) if shuffle_after_start => base_queue().move_entry(position, 0),
        ListStart::InOrder(position) | ListStart::Selected(position) => {
            set_shuffle(false, "");
            if position > 0 {
                match base_queue().skip(position) {
                    Ok(skipped) => skipped.into_iter().for_each(|t| history().prepend(t)),
                    Err(e) => log::warn!("could not skip to position {position}: {e:#}"),
                }
            }
        }
    }
    let started = pop_next_streamable_from_queue();
    if shuffle_after_start {
        set_shuffle(true, started.as_ref().map_or("", Track::id));
    }
    let Some(t) = started else {
        anyhow::bail!("tracklist produced no valid tracks to be played");
    };
    fire(Trigger::PlayTrack(t.clone(), false));
    Ok(t)
}

/// What a start at a position does on a player state change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartAt {
    Wait,
    /// Loaded on the track: seek, then play.
    Seek,
    /// The load ended elsewhere (failed, or another track).
    GiveUp,
}

/// Follows a track loaded paused until it can seek. `on_track`: the player's
/// current track is the one started.
#[derive(Debug, Default)]
pub struct StartGate {
    loading_seen: bool,
}

impl StartGate {
    pub fn step(&mut self, ps: &PlaybackState, on_track: bool) -> StartAt {
        if ps.loading {
            // A load still running for another track ends first; only the started one counts.
            self.loading_seen |= on_track;
            return StartAt::Wait;
        }
        // Until the load starts, the state is still the previous track's.
        if !self.loading_seen {
            return StartAt::Wait;
        }
        // Stopped after a load: an earlier load of the same track ended (the
        // stop before this start), or this one failed. Wait for the next load;
        // a new list ends the wait.
        if ps.status == PlaybackStatus::Stopped {
            self.loading_seen = false;
            return StartAt::Wait;
        }
        if on_track { StartAt::Seek } else { StartAt::GiveUp }
    }
}

/// Plays `tracks` from `position` into the first one. It loads paused and
/// seeks before playing, so the start of the track is never heard.
pub fn play_at(source: Option<PlaybackSource>, tracks: Vec<Track>, position: Duration) -> Result<()> {
    let first = tracks.first().map(|t| t.id().to_string());
    // Loads paused, so the start of the track is never heard before the seek.
    let token = first.as_deref().filter(|_| !position.is_zero()).map(statemachine::start_paused);
    let drop_request = move || {
        if let Some(t) = token {
            statemachine::drop_start_paused(t);
        }
    };
    let started = play_tracks(tracks, ListStart::InOrder(0)).inspect_err(|_| drop_request())?;
    source_changed().set(source);
    if position.is_zero() || first.as_deref() != Some(started.id()) {
        drop_request();
        return Ok(());
    }
    let generation = LIST_GENERATION.with(Cell::get);
    let paused = PAUSES.with(Cell::get);
    let requested = play_requests();
    let gate = std::cell::RefCell::new(StartGate::default());
    playback_state_changed().on_lazy(move |ps| {
        if LIST_GENERATION.with(Cell::get) != generation {
            drop_request();
            return crate::signals::UNSUBSCRIBE;
        }
        match gate.borrow_mut().step(ps, current_track_id().as_deref() == Some(started.id())) {
            StartAt::Wait => CONTINUE,
            StartAt::Seek => {
                // The pipeline may still be prerolling and refuse the seek.
                let mut tries = 0;
                let id = started.id().to_string();
                glib::timeout_add_local(Duration::from_millis(100), move || {
                    tries += 1;
                    // Another list, track or start (a repeat-one restart too), or a pause
                    // (a send elsewhere, say) came since.
                    if LIST_GENERATION.with(Cell::get) != generation
                        || current_track_id().as_deref() != Some(id.as_str())
                        || PAUSES.with(Cell::get) != paused
                        || play_requests() != requested
                    {
                        return glib::ControlFlow::Break;
                    }
                    let seeked = try_seek(position, true);
                    if seeked || tries >= 20 {
                        // Unseekable: playing from the start beats not playing.
                        if !seeked {
                            log::warn!("handoff: could not seek to the handed-off position");
                        }
                        play();
                        return glib::ControlFlow::Break;
                    }
                    glib::ControlFlow::Continue
                });
                crate::signals::UNSUBSCRIBE
            }
            StartAt::GiveUp => {
                // The handed-off start failed or was replaced: no later start of it loads paused.
                drop_request();
                crate::signals::UNSUBSCRIBE
            }
        }
    });
    // After the gate is installed, so it sees every state change the pause causes;
    // not `pause()`, which would count as a later pause and cancel this start.
    fire(Trigger::Pause);
    Ok(())
}

// ---------------------------------------------------------------- listen together

/// Current playback position as reported by `GStreamer`.
pub fn position() -> Option<Duration> {
    playbin().and_then(|p| statemachine::Playbin::query_position(&*p))
}

#[cfg(test)]
mod start_tests {
    use super::{PlaybackState, PlaybackStatus, StartAt, StartGate};

    fn ps(status: PlaybackStatus, loading: bool) -> PlaybackState {
        PlaybackState { status, loading, ..PlaybackState::default() }
    }

    #[test]
    fn seeks_once_the_track_has_loaded_paused() {
        let mut g = StartGate::default();
        // The previous track's state, before the new load starts.
        assert_eq!(g.step(&ps(PlaybackStatus::Paused, false), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, false), false), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, true), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Paused, false), true), StartAt::Seek);
    }

    #[test]
    fn an_earlier_load_of_another_track_is_not_the_start() {
        let mut g = StartGate::default();
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, true), false), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, false), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, true), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Paused, false), true), StartAt::Seek);
    }

    #[test]
    fn an_earlier_load_of_the_same_track_ending_waits_for_the_next() {
        let mut g = StartGate::default();
        g.step(&ps(PlaybackStatus::Stopped, true), true);
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, false), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Paused, false), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Stopped, true), true), StartAt::Wait);
        assert_eq!(g.step(&ps(PlaybackStatus::Paused, false), true), StartAt::Seek);
    }

    #[test]
    fn gives_up_when_another_track_plays() {
        let mut g = StartGate::default();
        g.step(&ps(PlaybackStatus::Stopped, true), true);
        assert_eq!(g.step(&ps(PlaybackStatus::Playing, false), false), StartAt::GiveUp);
    }
}

#[cfg(test)]
mod feed_tests {
    #[test]
    fn a_late_page_does_not_advance_after_a_start_or_stop() {
        use super::*;

        for trigger in [Trigger::PlayTrack(Track::default(), false), Trigger::Stop] {
            bump_list_generation();
            clear_queues();
            set_repeat_mode(RepeatMode::None);
            let generation = list_generation();
            let done = Rc::new(std::cell::RefCell::new(None));
            let callback = Rc::clone(&done);
            FEEDER.with(|f| *f.borrow_mut() = Some((generation, Rc::new(move |cb| *callback.borrow_mut() = Some(cb)))));
            WAITING.with(|w| w.set(true));
            feed(true);

            fire_uncounted(trigger);
            let mut late = Track::default();
            late.info.streamable = true;
            assert!(extend_list(generation, vec![late], false));
            done.borrow_mut().take().unwrap()(false);

            assert!(!WAITING.with(Cell::get));
            assert_eq!(base_queue().entries().with(Vec::len), 1);
        }
    }

    #[test]
    fn unloaded_rows_cover_the_rest_of_the_list() {
        assert_eq!(super::unloaded_count(Some(3000), 50, true), 2950);
        assert_eq!(super::unloaded_count(Some(10), 50, true), 0);
        // fully loaded or unknown size: no placeholders
        assert_eq!(super::unloaded_count(Some(3000), 50, false), 0);
        assert_eq!(super::unloaded_count(None, 50, true), 0);
    }

    #[test]
    fn asks_for_more_only_near_the_end_of_the_window() {
        assert!(super::needs_more(0));
        assert!(super::needs_more(super::FEED_THRESHOLD));
        assert!(!super::needs_more(super::FEED_THRESHOLD + 1));
    }

    #[test]
    fn stale_page_feeder_callback_does_not_clear_feeding_for_new_generation() {
        use std::cell::{Cell, RefCell};
        use std::rc::Rc;

        type DoneSlot = Rc<RefCell<Option<Rc<dyn Fn(bool)>>>>;

        super::bump_list_generation();
        let gen1 = super::list_generation();
        let stale_done: DoneSlot = Rc::new(RefCell::new(None));
        let sd = Rc::clone(&stale_done);
        super::FEEDER.with(|f| {
            *f.borrow_mut() = Some((
                gen1,
                Rc::new(move |done| {
                    *sd.borrow_mut() = Some(done);
                }),
            ));
        });
        super::feed(true);
        assert!(super::FEEDING.with(Cell::get));

        super::bump_list_generation();
        let gen2 = super::list_generation();
        let calls = Rc::new(Cell::new(0usize));
        let active_done: DoneSlot = Rc::new(RefCell::new(None));
        let c = Rc::clone(&calls);
        let ad = Rc::clone(&active_done);
        super::FEEDER.with(|f| {
            *f.borrow_mut() = Some((
                gen2,
                Rc::new(move |done| {
                    c.set(c.get() + 1);
                    *ad.borrow_mut() = Some(done);
                }),
            ));
        });
        super::feed(true);
        assert_eq!(calls.get(), 1);
        assert!(super::FEEDING.with(Cell::get));

        // Stale gen 1 callback must not clear gen 2's FEEDING or FEEDER.
        stale_done.borrow_mut().take().unwrap()(false);
        assert!(super::FEEDING.with(Cell::get));
        assert!(super::list_pending());
        super::feed(true);
        assert_eq!(calls.get(), 1);

        // Gen 2's own callback clears FEEDING so another request can run.
        active_done.borrow_mut().take().unwrap()(true);
        assert!(!super::FEEDING.with(Cell::get));
        super::feed(true);
        assert_eq!(calls.get(), 2);

        super::bump_list_generation();
    }
}


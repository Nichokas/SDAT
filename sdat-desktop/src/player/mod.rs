//! Port of `internal/player`: playbin3 wrapper, queues, controls, gapless
//! playback, `ReplayGain`, stream quality and MPRIS.


pub mod mpris;
pub mod play_next;
pub use tonearm_core::playqueue;
pub mod quality;
pub mod replaygain;
pub mod statemachine;

use anyhow::{anyhow, Result};
use gst::prelude::*;
use std::cell::{Cell, OnceCell};
use std::hash::{BuildHasher, RandomState};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use crate::model::{PlaybackSource, PlaylistInfo, Track};
use crate::signals::{Signal, State, CONTINUE};
use crate::{api, global_signal, i18n::tr, service, settings};
use playqueue::{DurableQueue, HistoryEntry, HistoryQueue, UserQueue};
pub use quality::audio_stream_quality;
use statemachine::{GaplessEntry, GaplessGate, GaplessSource, Hooks, Machine, Trigger};
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
            commit_gapless,
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
        }
        bin.connect_deep_element_added(|_, _, element| quality::on_deep_element_added(element));
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
        CONTINUE
    });
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
        let started = play_tracks(tracks, false, 1);
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

fn random_index(len: usize) -> usize {
    thread_local! {
        static COUNTER: Cell<u64> = const { Cell::new(0) };
    }
    let n = len as u64;
    if n <= 1 {
        return 0;
    }
    let threshold = n.wrapping_neg() % n;
    loop {
        let step = COUNTER.with(|c| {
            let next = c.get().wrapping_add(1);
            c.set(next);
            next
        });
        let sample = RandomState::new().hash_one(step);
        if sample >= threshold {
            return (sample % n) as usize;
        }
    }
}

fn peek_next_queue_entry() -> Option<(Track, GaplessSource)> {
    if let Some(t) = user_queue().entries().with(|v| v.iter().find(|t| t.info.streamable).cloned()) {
        return Some((t, GaplessSource::UserQueue));
    }
    if let Some(t) = base_queue().entries().with(|v| v.iter().find(|t| t.info.streamable).cloned()) {
        return Some((t, GaplessSource::BaseQueue { rewound: false }));
    }
    if list_pending() || repeat_mode_changed().get() != RepeatMode::Queue {
        return None;
    }
    // Sentinel id never matches a real track, so all source tracks are visible
    // without mutating `base_queue` during a peek.
    let mut streamable = base_queue()
        .played("\0", &user_queue().source_occurrences())
        .into_iter()
        .filter(|t| t.info.streamable);
    let next = if shuffle_state_changed().get() {
        let mut candidates: Vec<_> = streamable.collect();
        if candidates.is_empty() {
            return None;
        }
        if let Some(current_id) = current_track_id()
            && candidates.iter().any(|t| t.id() != current_id)
        {
            candidates.retain(|t| t.id() != current_id);
        }
        candidates.swap_remove(random_index(candidates.len()))
    } else {
        streamable.next()?
    };
    Some((next, GaplessSource::BaseQueue { rewound: true }))
}

fn pop_next_streamable_from_queue() -> Option<Track> {
    while let Some(t) = pop_user_queue() {
        if t.info.streamable {
            return Some(t);
        }
    }
    while let Some(t) = base_queue().pop() {
        if t.info.streamable {
            return Some(t);
        }
    }
    if list_pending() || repeat_mode_changed().get() != RepeatMode::Queue {
        return None;
    }
    log::debug!("queue repeat mode is enabled, replaying base queue");
    base_queue().rewind();
    if shuffle_state_changed().get() {
        base_queue().shuffle(&current_track_id().unwrap_or_default(), &user_queue().source_occurrences());
    }
    while let Some(t) = base_queue().pop() {
        if t.info.streamable {
            return Some(t);
        }
    }
    None
}

/// Returns the next streamable track and whether it is a track repeat.
fn next_track(peek: bool) -> (Option<Track>, bool) {
    if crate::live::is_guest() {
        return (None, false);
    }
    if repeat_mode_changed().get() == RepeatMode::Track {
        return (track_changed().with(|t| t.as_ref().map(|e| e.track.clone())), true);
    }
    (next_streamable_from_queue(peek), false)
}

fn next_streamable_from_queue(peek: bool) -> Option<Track> {
    if peek {
        peek_next_queue_entry().map(|(t, _)| t)
    } else {
        pop_next_streamable_from_queue()
    }
}

fn peek_gapless() -> Option<GaplessEntry> {
    if crate::live::is_guest() {
        return None;
    }
    if repeat_mode_changed().get() == RepeatMode::Track {
        return track_changed().with(|t| {
            t.as_ref().map(|e| GaplessEntry {
                track: e.track.clone(),
                did_repeat: true,
                source: GaplessSource::RepeatTrack,
            })
        });
    }
    peek_next_queue_entry().map(|(track, source)| GaplessEntry { track, did_repeat: false, source })
}

fn commit_gapless(entry: &GaplessEntry) {
    match entry.source {
        GaplessSource::RepeatTrack => {}
        GaplessSource::UserQueue => {
            let idx = user_queue()
                .entries()
                .with(|rows| rows.iter().position(|t| track_matches(t, &entry.track)));
            let removed = idx.and_then(|idx| {
                user_queue().edit(|rows| {
                    if idx >= rows.len() {
                        return None;
                    }
                    let row = rows.remove(idx);
                    update_pinned(|p| play_next::after_remove(p, idx));
                    Some(row)
                })
            });
            base_queue().select_history_occurrence(removed.as_ref().and_then(|r| r.occurrence.as_ref()));
        }
        GaplessSource::BaseQueue { rewound } => {
            let mut idx = base_queue()
                .entries()
                .with(|rows| rows.iter().position(|t| track_matches(t, &entry.track)));
            if idx.is_none() && rewound && repeat_mode_changed().get() == RepeatMode::Queue {
                log::debug!("queue repeat mode is enabled, replaying base queue");
                base_queue().rewind();
                if shuffle_state_changed().get() {
                    base_queue().shuffle(&current_track_id().unwrap_or_default(), &user_queue().source_occurrences());
                }
                idx = base_queue()
                    .entries()
                    .with(|rows| rows.iter().position(|t| track_matches(t, &entry.track)));
            }
            if let Some(i) = idx {
                let leading_unstreamable = i > 0
                    && base_queue().entries().with(|rows| rows[..i].iter().all(|t| !t.info.streamable));
                if leading_unstreamable {
                    let _ = base_queue().skip(i);
                } else if i > 0 {
                    base_queue().move_entry(i, 0);
                }
                base_queue().pop();
            } else {
                base_queue().select_history_occurrence(None);
            }
        }
    }
}

fn previous_track() -> Option<HistoryEntry> {
    history().pop()
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
    if tracks.is_empty() {
        return;
    }
    // Keep an already-chosen gapless track ahead of Play Next entries when it
    // came from the queues so the queue view matches playback order.
    let armed = with(|p| (p.machine.gapless_source(), p.machine.gapless_track()));
    let floor = match armed {
        Some((Some(GaplessSource::UserQueue), Some(armed))) => {
            usize::from(user_queue().peek().is_some_and(|head| track_matches(&head, &armed)))
        }
        Some((Some(GaplessSource::BaseQueue { rewound: false }), Some(armed))) => {
            if user_queue().is_empty()
                && base_queue().peek().is_some_and(|head| track_matches(&head, &armed))
                && let Some(head) = base_queue().take_for_gapless()
            {
                user_queue().prepend(head);
                with(|p| p.machine.move_gapless_to_user_queue());
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
    let pos = playbin().and_then(|p| statemachine::Playbin::query_position(&*p));
    if repeat_mode_changed().get() == RepeatMode::Track
        || pos.is_some_and(|p| p > Duration::from_secs(5))
        || history().is_empty()
    {
        seek_to_position(Duration::ZERO, false);
        return;
    }
    let Some(prev) = previous_track() else {
        log::error!("failed to get previous track");
        return;
    };
    if let Some(cur) = track_changed().with(|t| t.as_ref().map(|e| e.track.clone())) {
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
        // Continues after the last list track that played, which may not be
        // the current one (a "play next" track).
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
pub async fn play_tracklist(source: PlaybackSource, tracks: Vec<Track>, shuffle: bool, position: usize) -> Result<()> {
    play_tracks(tracks, shuffle, position)?;
    source_changed().set(Some(source));
    Ok(())
}

pub async fn play_album(album_id: &str, shuffle: bool, position: usize) -> Result<()> {
    let id = album_id.to_string();
    let i = id.clone();
    let info = api::run(move || service::get_album_info(&i)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Album(id)).await?;
    clear_queues();
    stop();
    play_tracks(tracks, shuffle, position)?;
    source_changed().set(Some(PlaybackSource::Album(info)));
    Ok(())
}

pub async fn play_artist_top_songs(artist_id: &str, shuffle: bool, position: usize) -> Result<()> {
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
    play_tracks(tracks, shuffle, position)?;
    source_changed().set(Some(PlaybackSource::Artist(info)));
    Ok(())
}

pub async fn play_playlist(playlist_id: &str, shuffle: bool) -> Result<()> {
    let id = playlist_id.to_string();
    let i = id.clone();
    let info = api::run(move || service::get_playlist_info(&i)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Playlist(id)).await?;
    play_tracklist(PlaybackSource::Playlist(info), tracks, shuffle, 0).await
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
    play_tracklist(PlaybackSource::Playlist(info), tracks, false, start).await?;
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
        let tracks = vec![track("A", None), track("B", None)];
        let started = play_tracks(tracks, false, 1)?;
        assert_eq!(started.id(), "B");
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
        let tracks = vec![track("A", Some("a")), clicked.clone(), track("C", Some("c"))];
        user_queue().append(track("queued", None));

        assert!(glib::MainContext::new().block_on(play_playlist_selection(info, tracks, &clicked, 0))?);
        assert!(user_queue().is_empty());
        assert_eq!(base_queue().peek().as_ref().map(Track::id), Some("C"));
        Ok(())
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
    fn gapless_commit_ignores_user_queue_addition_and_repeat_toggle() {
        clear_queues();
        set_repeat_mode(RepeatMode::None);
        base_queue().set(vec![track("A", None), track("B", None), track("C", None)]);
        let a = base_queue().pop().unwrap();
        track_changed().set(Some(statemachine::TrackChangedEvent { track: a, skip_history: false }));

        // Arm Track B from the base queue.
        let armed = peek_gapless().unwrap();
        assert_eq!(armed.track.id(), "B");
        assert!(!armed.did_repeat);
        assert_eq!(armed.source, GaplessSource::BaseQueue { rewound: false });

        // User adds Track X to an empty user_queue and toggles RepeatMode::Track on during PlayingGapless.
        user_queue().append(track("X", None));
        set_repeat_mode(RepeatMode::Track);

        // StreamStart commits the exact armed Track B from base_queue without popping X.
        commit_gapless(&armed);
        assert_eq!(user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["X"]);
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["C"]);

        // Now arm a RepeatMode::Track repeat, then turn repeat off before commit.
        let repeated = peek_gapless().unwrap();
        assert_eq!(repeated.track.id(), "A");
        assert!(repeated.did_repeat);
        assert_eq!(repeated.source, GaplessSource::RepeatTrack);
        set_repeat_mode(RepeatMode::None);
        commit_gapless(&repeated);
        assert_eq!(user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["X"]);
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["C"]);

        clear_queues();
        track_changed().set(None);
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

        set_repeat_mode(RepeatMode::Track);
        previous_now();

        assert!(user_queue().is_empty(), "previous_now must not enqueue duplicates under RepeatMode::Track");
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 0);
        assert_eq!(history().pop().map(|e| e.track.id().to_string()).as_deref(), Some("A"));
        assert!(history().is_empty());

        set_repeat_mode(RepeatMode::None);
        track_changed().set(None);
    }

    #[test]
    fn peek_next_track_does_not_mutate_base_queue_on_repeat_or_shuffle() {
        clear_queues();
        set_repeat_mode(RepeatMode::Queue);
        base_queue().set(vec![track("A", None), track("B", None)]);
        assert_eq!(base_queue().pop().unwrap().id(), "A");
        let b = base_queue().pop().unwrap();
        track_changed().set(Some(statemachine::TrackChangedEvent { track: b, skip_history: false }));
        assert!(base_queue().is_empty());

        // Peeking under RepeatMode::Queue must inspect without rewinding.
        let (peeked, did_repeat) = next_track(true);
        assert!(!did_repeat);
        assert_eq!(peeked.as_ref().map(Track::id), Some("A"));
        assert!(base_queue().is_empty(), "peek mutated base_queue by rewinding");

        // Same when shuffle is enabled: peeking must neither rewind nor reshuffle.
        shuffle_state_changed().set(true);
        let (peeked_shuf, _) = next_track(true);
        assert!(peeked_shuf.is_some());
        assert!(base_queue().is_empty(), "peek mutated base_queue while shuffled");

        // Popping actually rewinds, shuffles, and pops one entry (leaving the other).
        let (popped, did_repeat) = next_track(false);
        assert!(!did_repeat);
        assert!(popped.is_some());
        assert_eq!(base_queue().entries().with(Vec::len), 1);

        shuffle_state_changed().set(false);
        set_repeat_mode(RepeatMode::None);
        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn shuffle_repeat_queue_wraparound_peek_is_randomized_and_non_mutating() {
        use std::collections::HashSet;

        clear_queues();
        set_repeat_mode(RepeatMode::Queue);
        let mut unavailable = track("U", Some("u1"));
        unavailable.info.streamable = false;
        let tracks = vec![
            track("A", Some("a1")),
            track("B", Some("b1")),
            track("B", Some("b2")),
            track("C", Some("c1")),
            unavailable,
            track("D", Some("d1")),
        ];
        let all_streamable: HashSet<(String, Option<String>)> = tracks
            .iter()
            .filter(|t| t.info.streamable)
            .map(|t| (t.id().to_string(), t.playlist_item_id.clone()))
            .collect();
        base_queue().set(tracks);
        set_shuffle(true, "");

        let mut last_streamable = None;
        while let Some(t) = base_queue().pop() {
            if t.info.streamable {
                last_streamable = Some(t);
            }
        }
        let current = last_streamable.unwrap();
        let current_id = current.id().to_string();
        track_changed().set(Some(statemachine::TrackChangedEvent { track: current, skip_history: false }));
        assert!(base_queue().is_empty());

        let played_before: Vec<(String, Option<String>)> = base_queue()
            .played("\0", &[])
            .into_iter()
            .map(|t| (t.id().to_string(), t.playlist_item_id))
            .collect();
        let first_streamable_key = base_queue()
            .played("\0", &[])
            .into_iter()
            .find(|t| t.info.streamable)
            .map(|t| (t.id().to_string(), t.playlist_item_id));
        let expected: HashSet<(String, Option<String>)> = all_streamable
            .iter()
            .filter(|(id, _)| id != &current_id)
            .cloned()
            .collect();
        assert!(expected.len() >= 3);

        let mut seen = HashSet::new();
        let mut chosen_non_first = None;
        for _ in 0..256 {
            let entry = peek_gapless().unwrap();
            assert!(!entry.did_repeat);
            assert_eq!(entry.source, GaplessSource::BaseQueue { rewound: true });
            assert!(entry.track.info.streamable);
            assert_ne!(entry.track.id(), current_id, "wraparound peek must avoid immediately repeating the current track");
            assert!(base_queue().is_empty(), "wraparound peek must not mutate base_queue");

            let key = (entry.track.id().to_string(), entry.track.playlist_item_id.clone());
            if first_streamable_key.as_ref() != Some(&key) && chosen_non_first.is_none() {
                chosen_non_first = Some(entry);
            }
            seen.insert(key);
        }

        let played_after: Vec<(String, Option<String>)> = base_queue()
            .played("\0", &[])
            .into_iter()
            .map(|t| (t.id().to_string(), t.playlist_item_id))
            .collect();
        assert_eq!(played_after, played_before, "repeated peeks must leave base_queue history untouched");
        assert_eq!(seen, expected, "wraparound peek should randomize across all eligible non-current candidates");

        let chosen = chosen_non_first.unwrap();
        let chosen_key = (chosen.track.id().to_string(), chosen.track.playlist_item_id.clone());
        commit_gapless(&chosen);
        assert!(base_queue().is_shuffled());
        assert!(base_queue().current_occurrence(&chosen.track).is_some());
        let remaining_streamable: HashSet<(String, Option<String>)> = base_queue().entries().with(|rows| {
            assert!(!rows.iter().any(|t| track_matches(t, &chosen.track)));
            rows.iter()
                .filter(|t| t.info.streamable)
                .map(|t| (t.id().to_string(), t.playlist_item_id.clone()))
                .collect()
        });
        let mut expected_remaining = all_streamable;
        assert!(expected_remaining.remove(&chosen_key));
        assert_eq!(remaining_streamable, expected_remaining);

        // When all streamable tracks share the current track id, wraparound must still succeed.
        clear_queues();
        let mut only_unstreamable = track("U2", Some("u2"));
        only_unstreamable.info.streamable = false;
        base_queue().set(vec![track("S", Some("s1")), track("S", Some("s2")), only_unstreamable]);
        set_shuffle(true, "");
        let mut only = None;
        while let Some(t) = base_queue().pop() {
            if t.info.streamable {
                only = Some(t);
            }
        }
        track_changed().set(Some(statemachine::TrackChangedEvent { track: only.unwrap(), skip_history: false }));
        let wrapped = peek_gapless().unwrap();
        assert_eq!(wrapped.track.id(), "S");
        assert_eq!(wrapped.source, GaplessSource::BaseQueue { rewound: true });
        assert!(base_queue().is_empty());
        commit_gapless(&wrapped);
        assert!(base_queue().current_occurrence(&wrapped.track).is_some());
        assert!(base_queue().entries().with(|rows| !rows.iter().any(|t| track_matches(t, &wrapped.track))));

        shuffle_state_changed().set(false);
        set_repeat_mode(RepeatMode::None);
        clear_queues();
        track_changed().set(None);
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

/// Plays `tracks` as the queue starting at the clicked row; earlier rows go to
/// the history so "previous" walks back through the list. Returns the queue
/// generation for [`extend_list`], or `None` if nothing could be played.
pub fn play_list_from(source: PlaybackSource, tracks: Vec<Track>, clicked: &Track, pos: usize) -> Option<u64> {
    let start = crate::model::queue_start(&tracks, clicked.id(), pos);
    match play_tracks(tracks, false, start) {
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
            WAITING.with(|w| w.set(false));
            glib::spawn_future_local(on_eos_stopped());
        }
    }));
}

/// Appends late-loaded tracks to the list started by [`play_list_from`].
/// Returns `false` (and does nothing) once another list replaced it.
pub fn extend_list(generation: u64, tracks: &[Track]) -> bool {
    if LIST_GENERATION.with(std::cell::Cell::get) != generation {
        return false;
    }
    base_queue().extend(tracks);
    update_unloaded();
    true
}

/// Makes `tracks` the rest of the list after the current track, as if the
/// list had started with them: queue repeat and shuffle cover all of it.
/// Unlike [`extend_list`], keeps repeated tracks.
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

fn play_tracks(tracks: Vec<Track>, shuffle: bool, starting_position: usize) -> Result<Track> {
    bump_list_generation();
    clear_queues();
    stop();
    base_queue().set(tracks);
    set_shuffle(shuffle, "");
    if starting_position > 0 {
        match base_queue().skip(starting_position) {
            Ok(skipped) => skipped.into_iter().for_each(|t| history().prepend(t)),
            Err(e) => log::warn!("could not skip to position {starting_position}: {e:#}"),
        }
    }
    let Some(t) = next_streamable_from_queue(false) else {
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
    let started = play_tracks(tracks, false, 0).inspect_err(|_| drop_request())?;
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


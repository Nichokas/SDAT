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
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use crate::model::{PlaybackSource, Track};
use crate::signals::{Signal, State, CONTINUE};
use crate::{api, global_signal, i18n::tr, service, settings};
use playqueue::{DurableQueue, HistoryQueue, Queue};
pub use quality::audio_stream_quality;
use statemachine::{Hooks, Machine, Trigger};
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
global_signal!(pub fn user_queue() -> Queue = Queue::default());
global_signal!(pub fn history() -> HistoryQueue = HistoryQueue::default());

thread_local! {
    /// Entries at the front of the user queue that were added with "play next".
    static PLAY_NEXT_PINNED: Cell<usize> = const { Cell::new(0) };
    static PLAY_NEXT_ORDER: std::cell::RefCell<play_next::Sequencer<(Option<u64>, Vec<Track>)>> = std::cell::RefCell::default();
    /// Bumped when the queues are replaced, so pending "play next" fetches are discarded.
    static PLAY_NEXT_GENERATION: Cell<u64> = const { Cell::new(0) };
    /// Id of the gapless-prepared track moved from the base queue to the head
    /// of the user queue by "play next", until that entry is consumed.
    static GAPLESS_MOVED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
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
/// only queues the start, so a second call would pop (and lose) a track.
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
    update_pinned(|p| play_next::after_move(p, src, dst));
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

struct Player {
    playbin: gst::Element,
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
    if with(|p| p.machine.fire(t)).is_none() {
        log::warn!("player not initialised, dropping trigger");
    }
}

fn playbin() -> Option<gst::Element> {
    with(|p| p.playbin.clone())
}

/// Initialises `GStreamer`, playbin3, the state machine and MPRIS.
/// Must be called once on the GTK main thread.
pub fn init() -> Result<()> {
    if with(|_| ()).is_some() {
        return Ok(());
    }
    gst::init()?;
    let playbin = gst::ElementFactory::make("playbin3").build().map_err(|e| anyhow!("playbin3 unavailable: {e}"))?;

    let pb: Rc<dyn statemachine::Playbin> = Rc::new(playbin.clone());
    let machine = Machine::new(
        pb,
        Hooks {
            next_track,
            on_eos_stopped: || Box::pin(on_eos_stopped()),
            clear_source: || source_changed().set(None),
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

    playbin.connect("about-to-finish", false, |_| {
        let (tx, rx) = mpsc::channel();
        let ctx = glib::MainContext::default();
        if ctx.is_owner() {
            fire(Trigger::GaplessHint(None));
            return None;
        }
        ctx.invoke(move || fire(Trigger::GaplessHint(Some(tx))));
        // Block the streaming thread until the next URI is set (gapless).
        if rx.recv_timeout(Duration::from_secs(15)).is_err() {
            log::warn!("timed out waiting for the next gapless track");
        }
        None
    });

    if let Some(bin) = playbin.downcast_ref::<gst::Bin>() {
        bin.connect_deep_element_added(|_, _, element| quality::on_deep_element_added(element));
    }

    replaygain::init(&playbin)?;
    playbin.set_property("volume", settings::volume());

    PLAYER.with(|p| {
        let _ = p.set(Player { playbin, machine, _bus_watch: bus_watch });
    });

    track_changed().on(|ev| {
        if let Some(ev) = ev
            && !ev.skip_history {
                history().prepend(ev.track.clone());
            }
        CONTINUE
    });
    source_changed().on(|_| {
        replaygain::update_album_mode();
        CONTINUE
    });
    enqueued_track_hook();
    Ok(())
}

fn enqueued_track_hook() {
    statemachine::enqueued_track().on(|m| {
        replaygain::set_enqueued(m.as_ref());
        CONTINUE
    });
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
        }
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
    if list_pending() {
        // The loaded part ran out before the next page arrived: wait for it
        // instead of stopping, starting a radio or repeating.
        log::info!("queue ran dry while the list is still loading; waiting for the next page");
        WAITING.with(|w| w.set(true));
        feed(true);
        return;
    }
    let stop = || {
        source_changed().set(None);
        statemachine::on_stopped_stop();
    };
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
    user_queue().clear();
    update_pinned(|_| 0);
    // A guest's pending "play next" requests belong to the session and go to
    // the host, so keep them (and their order) when the guest follows a host
    // track change; otherwise they target the replaced queue and are dropped.
    if !crate::live::is_guest() {
        PLAY_NEXT_GENERATION.with(|g| g.set(g.get() + 1));
        PLAY_NEXT_ORDER.with(|s| *s.borrow_mut() = play_next::Sequencer::default());
    }
    GAPLESS_MOVED.with(|m| m.borrow_mut().take());
    base_queue().clear();
}

fn get_next_track_from_queue(peek: bool) -> Option<Track> {
    let t = if peek { user_queue().peek() } else { user_queue().pop() };
    if t.is_some() {
        if !peek {
            update_pinned(|p| play_next::after_remove(p, 0));
            // Forget the moved gapless entry only once that entry is consumed.
            GAPLESS_MOVED.with(|m| {
                let mut m = m.borrow_mut();
                if t.as_ref().is_some_and(|t| m.as_deref() == Some(t.id())) {
                    m.take();
                }
            });
        }
        return t;
    }
    let t = if peek { base_queue().peek() } else { base_queue().pop() };
    if t.is_some() {
        return t;
    }
    if list_pending() {
        // More of the list is coming: don't wrap around (repeat) yet.
        return None;
    }
    if repeat_mode_changed().get() == RepeatMode::Queue {
        log::debug!("queue repeat mode is enabled, replaying base queue");
        base_queue().restore("");
        if shuffle_state_changed().get() {
            base_queue().shuffle("");
        }
        return base_queue().peek();
    }
    None
}

/// Returns the next streamable track and whether it is a track repeat.
fn next_track(peek: bool) -> (Option<Track>, bool) {
    let repeat = repeat_mode_changed().get();
    if repeat == RepeatMode::Track {
        return (track_changed().with(|t| t.as_ref().map(|e| e.track.clone())), true);
    }
    let mut next = get_next_track_from_queue(true);
    while matches!(&next, Some(t) if !t.info.streamable) {
        get_next_track_from_queue(false);
        next = get_next_track_from_queue(true);
    }
    if !peek {
        get_next_track_from_queue(false);
    }
    (next, false)
}

fn previous_track(peek: bool) -> Option<Track> {
    if repeat_mode_changed().get() == RepeatMode::Track
        && let Some(t) = track_changed().with(|t| t.as_ref().map(|e| e.track.clone())) {
            return Some(t);
        }
    if peek { history().peek() } else { history().pop() }
}

pub async fn add_track_to_user_queue(track_id: &str) -> Result<()> {
    let id = track_id.to_string();
    let track = api::run(move || service::get_track(&id)).await?;
    user_queue().append(track);
    if playback_state_changed().get().status == PlaybackStatus::Stopped {
        log::info!("no track is currently playing, immediately playing track {track_id}");
        next();
    }
    Ok(())
}

/// Queues `tracks`, in order, right after the current one (and after any
/// earlier "play next" tracks). The caller starts playback if needed.
fn insert_tracklist_next(tracks: Vec<Track>) {
    if tracks.is_empty() {
        return;
    }
    // A gapless handoff already picked the head of the queues and pops it at
    // StreamStart; keep that entry first so the popped entry is the one playing.
    let gapless = with(|p| p.machine.gapless_pending()).unwrap_or(false)
        && repeat_mode_changed().get() != RepeatMode::Track;
    if gapless
        && user_queue().is_empty()
        && let Some(head) = base_queue().pop()
    {
        GAPLESS_MOVED.with(|m| *m.borrow_mut() = Some(head.id().to_string()));
        user_queue().prepend(head);
    }
    let floor = usize::from(gapless && !user_queue().is_empty());
    let mut entries = user_queue().entries().with(Clone::clone);
    let pinned = PLAY_NEXT_PINNED.with(Cell::get);
    let pinned = play_next::insert(&mut entries, pinned, floor, tracks);
    user_queue().set(entries);
    update_pinned(|_| pinned);
}

pub fn add_tracklist_to_user_queue(tracks: Vec<Track>) {
    let any = !tracks.is_empty();
    for t in tracks {
        user_queue().append(t);
    }
    if any && playback_state_changed().get().status == PlaybackStatus::Stopped {
        next();
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
                QueueKind::User => {
                    update_pinned(|p| play_next::after_remove(p, 0));
                    user_queue().pop()
                }
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

pub fn next() {
    match next_track(false).0 {
        Some(t) => fire(Trigger::PlayTrack(t, false)),
        None => log::error!("failed to get next track: queue is empty"),
    }
}

pub fn pause() {
    fire(Trigger::Pause);
}

pub fn play() {
    fire(Trigger::Resume);
}

pub fn play_pause() {
    match playback_state_changed().get().status {
        PlaybackStatus::Playing => pause(),
        PlaybackStatus::Paused | PlaybackStatus::Stopped => play(),
    }
}

pub fn previous() {
    let pos = playbin().and_then(|p| statemachine::Playbin::query_position(&p));
    if pos.is_some_and(|p| p > Duration::from_secs(5)) || history().peek().is_none() {
        seek_to_position(Duration::ZERO, false);
        return;
    }
    let Some(prev) = previous_track(false) else {
        log::error!("failed to get previous track");
        return;
    };
    if let Some(cur) = track_changed().with(|t| t.as_ref().map(|e| e.track.clone())) {
        user_queue().prepend(cur);
        // The track we came back from plays next, ahead of "play next" tracks.
        update_pinned(|p| p + 1);
    }
    fire(Trigger::PlayTrack(prev, false));
}

pub fn seek_to_percent(percent: f64) {
    let percent = percent.clamp(0.0, 100.0);
    let d = playback_state_changed().get().duration;
    seek_to_position(d.mul_f64(percent / 100.0), false);
}

pub fn seek_to_position(position: Duration, precise: bool) {
    let flags = if precise {
        gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE
    } else {
        gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT
    };
    // Subscribers (MPRIS `Seeked`, the Listen Together host anchor) take
    // `position` as reached, so a seek the pipeline refused must not notify.
    if playbin().is_some_and(|p| statemachine::Playbin::seek(&p, position, flags)) {
        seeked().notify(&position);
    }
}

pub fn set_volume(volume: f64) {
    let volume = if volume < 0.0 { 0.0 } else { volume };
    if volume > 1.0 {
        log::warn!("Volume is higher than 1. This will cause overdrive to the speakers.");
    }
    if let Some(p) = playbin() {
        p.set_property("volume", volume);
    }
}

pub fn stop() {
    source_changed().set(None);
    fire(Trigger::Stop);
}

pub fn set_repeat_mode(m: RepeatMode) {
    if m != repeat_mode_changed().get() {
        repeat_mode_changed().set(m);
    }
}

pub fn set_shuffle(shuffle: bool, current_track_id: &str) {
    if shuffle {
        base_queue().shuffle(current_track_id);
    } else {
        base_queue().restore(current_track_id);
    }
    // Rebuilding from the source brings back a gapless-prepared track that
    // "play next" moved to the user queue; drop the base copy so it plays once.
    // The marker is only valid while its entry is still in the user queue
    // (a skip may have drained it); otherwise forget it.
    let moved = GAPLESS_MOVED.with(|m| m.borrow_mut().take())
        .filter(|id| user_queue().entries().with(|e| e.iter().any(|t| t.id() == id.as_str())));
    GAPLESS_MOVED.with(|m| m.borrow_mut().clone_from(&moved));
    if let Some(id) = moved
        && let Some(i) = base_queue().entries().with(|e| e.iter().position(|t| t.id() == id))
    {
        // Only the upcoming copy: `remove_at` would also drop it from the
        // durable source, losing it for later shuffles and queue repeat.
        base_queue().entries().notify(|mut v| {
            v.remove(i);
            v
        });
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

pub async fn play_playlist(playlist_id: &str, shuffle: bool, position: usize) -> Result<()> {
    let id = playlist_id.to_string();
    let i = id.clone();
    let info = api::run(move || service::get_playlist_info(&i)).await?;
    let tracks = crate::cache::all_tracks(crate::cache::Job::Playlist(id)).await?;
    clear_queues();
    stop();
    play_tracks(tracks, shuffle, position)?;
    source_changed().set(Some(PlaybackSource::Playlist(info)));
    Ok(())
}

thread_local! {
    /// Bumped whenever the base queue is replaced, so background fetches for
    /// an older list can tell they are stale.
    static LIST_GENERATION: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
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
        FEEDING.with(|f| f.set(false));
        if !more {
            FEEDER.with(|f| {
                if f.borrow().as_ref().is_some_and(|(g, _)| *g == generation) {
                    *f.borrow_mut() = None;
                }
            });
        }
        update_unloaded();
        if LIST_GENERATION.with(std::cell::Cell::get) != generation || !WAITING.with(std::cell::Cell::get) {
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
    let Some(t) = next_track(false).0 else {
        anyhow::bail!("tracklist produced no valid tracks to be played");
    };
    fire(Trigger::PlayTrack(t.clone(), false));
    Ok(t)
}

// ---------------------------------------------------------------- listen together

/// Current playback position as reported by `GStreamer`.
pub fn position() -> Option<Duration> {
    playbin().and_then(|p| statemachine::Playbin::query_position(&p))
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
}


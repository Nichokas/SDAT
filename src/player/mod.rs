//! Port of `internal/player`: playbin3 wrapper, queues, controls, gapless
//! playback, ReplayGain, stream quality and MPRIS.
#![allow(dead_code, unused_imports)]

pub mod mpris;
pub mod playqueue;
pub mod quality;
pub mod replaygain;
pub mod statemachine;

use anyhow::{anyhow, Result};
use gst::prelude::*;
use std::cell::OnceCell;
use std::rc::Rc;
use std::sync::mpsc;
use std::time::Duration;

use crate::model::{PlaybackSource, Track};
use crate::signals::{Signal, State, CONTINUE};
use crate::{api, global_signal, i18n::tr, service, settings};
use playqueue::{DurableQueue, HistoryQueue, Queue};
pub use quality::{audio_stream_quality, Codec, StreamQuality};
use statemachine::{Hooks, Machine, Trigger};
pub use statemachine::{
    playback_quality_changed, playback_state_changed, track_changed, PlaybackState, PlaybackStatus, TrackChangedEvent,
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

/// Initialises GStreamer, playbin3, the state machine and MPRIS.
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
        if let Some(ev) = ev {
            if !ev.skip_history {
                history().prepend(ev.track.clone());
            }
        }
        CONTINUE
    });
    source_changed().on(|_| {
        replaygain::update_album_mode();
        CONTINUE
    });
    enqueued_track_hook();
    mpris::init();
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
    if v == volume_changed().get() {
        return;
    }
    settings::set_volume(v);
    volume_changed().set(v);
}

async fn on_eos_stopped() {
    let stop = || {
        source_changed().set(None);
        let _ = statemachine::on_stopped_stop();
    };
    if !settings::allow_autoplay() {
        stop();
        return;
    }
    let Some(id) = current_track_id() else {
        stop();
        return;
    };
    if let Err(e) = play_track_radio(&id, true).await {
        log::info!("failed to play track radio: {e:#}");
        stop();
    }
}

fn current_track_id() -> Option<String> {
    track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_string()))
}

// ---------------------------------------------------------------- queue.go

fn clear_queues() {
    user_queue().clear();
    base_queue().clear();
}

fn get_next_track_from_queue(peek: bool) -> Option<Track> {
    let t = if peek { user_queue().peek() } else { user_queue().pop() };
    if t.is_some() {
        return t;
    }
    let t = if peek { base_queue().peek() } else { base_queue().pop() };
    if t.is_some() {
        return t;
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
    if repeat_mode_changed().get() == RepeatMode::Track {
        if let Some(t) = track_changed().with(|t| t.as_ref().map(|e| e.track.clone())) {
            return Some(t);
        }
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
        QueueKind::User => user_queue().skip(to),
        QueueKind::Base => base_queue().skip(to),
    };
    match res {
        Ok(skipped) => {
            for t in skipped {
                history().prepend(t);
            }
            let next = match kind {
                QueueKind::User => user_queue().pop(),
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
    }
    fire(Trigger::PlayTrack(prev, false));
}

pub fn seek_to_percent(percent: f64) {
    let percent = percent.clamp(0.0, 100.0);
    let d = playback_state_changed().get().duration;
    seek_to_position(d.mul_f64(percent / 100.0), false);
}

pub fn seek_to_position(position: Duration, precise: bool) {
    if let Some(p) = playbin() {
        let flags = if precise {
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE
        } else {
            gst::SeekFlags::FLUSH | gst::SeekFlags::KEY_UNIT
        };
        statemachine::Playbin::seek(&p, position, flags);
    }
    seeked().notify(&position);
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
    clear_queues();
    stop();
    source_changed().set(Some(PlaybackSource::Track(Box::new(track.clone()))));
    fire(Trigger::PlayTrack(track, false));
}

pub async fn play_tracklist(source: PlaybackSource, tracks: Vec<Track>, shuffle: bool, position: usize) -> Result<()> {
    play_tracks(tracks, shuffle, position)?;
    source_changed().set(Some(source));
    Ok(())
}

pub async fn play_album(album_id: &str, shuffle: bool, position: usize) -> Result<()> {
    let id = album_id.to_string();
    let (info, tracks) =
        api::run(move || Ok((service::get_album_info(&id)?, service::get_album_tracks(&id).get_all()?))).await?;
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
        Ok((info, tracks))
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
    let (info, tracks) =
        api::run(move || Ok((service::get_playlist_info(&id)?, service::get_playlist_tracks(&id).get_all()?))).await?;
    clear_queues();
    stop();
    play_tracks(tracks, shuffle, position)?;
    source_changed().set(Some(PlaybackSource::Playlist(info)));
    Ok(())
}

pub async fn play_track_radio(track_id: &str, skip_self: bool) -> Result<()> {
    let id = track_id.to_string();
    let radio = api::run(move || service::get_track_radio(&id)).await.inspect_err(|e| log::error!("failed to retrieve mix: {e}"))?;
    let Some(mix) = radio else {
        toasts().notify(&tr("No mix found for the current track"));
        anyhow::bail!("no mix found for the current track");
    };
    play_playlist(&mix.id, false, usize::from(skip_self)).await
}

/// playTracklist: the tracks are already fetched (Go fetched inside; we fetch
/// on a worker thread before calling this to keep the UI responsive).
fn play_tracks(tracks: Vec<Track>, shuffle: bool, starting_position: usize) -> Result<Track> {
    clear_queues();
    stop();
    base_queue().set(tracks);
    set_shuffle(shuffle, "");
    if starting_position > 0 {
        let _ = base_queue().skip(starting_position);
    }
    let Some(t) = next_track(false).0 else {
        anyhow::bail!("tracklist produced no valid tracks to be played");
    };
    fire(Trigger::PlayTrack(t.clone(), false));
    Ok(t)
}

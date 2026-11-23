//! Plays tracks from the core's audio cache ([`tonearm_core::audio_cache`]) and fills it in the background: the next
//! track first, so it starts from disk, then the current one, so a replay does.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

use gst::prelude::*;
use tonearm_core::audio_cache;
use tonearm_core::CoreError;

use super::next::peek_gapless;
use super::{track_changed, PlaybackState, PlaybackStatus};
use crate::model::{TrackManifest, TrackQuality};
use crate::settings;

/// The fills wait until this much of the streamed track is buffered (or all of its rest), so on a slow link they
/// never take bandwidth the stream needs.
const HEADROOM: Duration = Duration::from_secs(30);

pub fn dir() -> PathBuf {
    glib::user_cache_dir().join(settings::CONFIG_DIR_NAME).join("audio")
}

#[derive(Debug, Default)]
struct Plan {
    generation: u64,
    tracks: Vec<String>,
    quality: TrackQuality,
}

static PLAN: Mutex<Plan> = Mutex::new(Plan { generation: 0, tracks: Vec::new(), quality: TrackQuality::HighResLossless });
static PLAN_CHANGED: Condvar = Condvar::new();
static DEMUXER: Mutex<Option<glib::WeakRef<gst::Element>>> = Mutex::new(None);

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
    static PLANNED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// The next track when the fills were last planned: a queue edit that changes it plans again.
    static PLANNED_NEXT: RefCell<Option<String>> = const { RefCell::new(None) };
}

pub fn init() {
    let limit = settings::audio_cache_limit();
    let spawned = std::thread::Builder::new().name("audio-cache".into()).spawn(move || {
        if let Err(e) = audio_cache::install(dir(), limit) {
            log::error!("failed to open the audio cache error={e}");
            return;
        }
        fill_planned_tracks();
    });
    if let Err(e) = spawned {
        log::error!("failed to start the audio cache thread error={e}");
    }
}

fn fill_planned_tracks() -> ! {
    let mut done = 0;
    loop {
        let (generation, tracks, quality) = {
            let mut p = PLAN.lock().unwrap_or_else(PoisonError::into_inner);
            while p.generation == done {
                p = PLAN_CHANGED.wait(p).unwrap_or_else(PoisonError::into_inner);
            }
            (p.generation, p.tracks.clone(), p.quality)
        };
        done = generation;
        for id in tracks {
            // Begun under the plan's lock: a stall that replaces the plan then cancels this fill, or it never begins.
            let job = {
                let p = PLAN.lock().unwrap_or_else(PoisonError::into_inner);
                if p.generation != generation {
                    break;
                }
                audio_cache::begin(&id, quality, Vec::new())
            };
            match job.and_then(audio_cache::Job::finish) {
                Ok(_) => {}
                // A purge (Clear, sign-out) or a new plan: the rest of this one must not fill the cache again.
                Err(CoreError::Cancelled(_)) => break,
                Err(e) => log::warn!("failed to cache track id={id} error={e}"),
            }
        }
    }
}

/// The cached copy of `track_id`, as a manifest playbin plays from disk. Blocking.
pub fn manifest(track_id: &str, quality: TrackQuality) -> Option<TrackManifest> {
    let c = audio_cache::lookup(track_id, quality)?;
    match glib::filename_to_uri(&c.playlist, None) {
        Ok(url) => Some(TrackManifest {
            url: url.into(),
            quality: c.quality,
            formats: c.formats,
            track_normalization: c.track_norm,
            album_normalization: c.album_norm,
        }),
        Err(e) => {
            log::warn!("cached track has no file URI id={track_id} error={e}");
            None
        }
    }
}

/// Removes the cached copy `uri` plays from and returns its track, which the file names: during a gapless
/// handoff `uri` already belongs to the next track.
pub fn drop_failed_copy(uri: &str) -> Option<String> {
    let (path, _) = glib::filename_from_uri(uri).ok()?;
    if !path.starts_with(dir()) {
        return None;
    }
    let id = path.file_stem()?.to_str()?.split('-').next()?.to_owned();
    log::warn!("cached track failed to play, streaming it instead id={id}");
    audio_cache::remove(&id);
    Some(id)
}

/// Called (from a streaming thread) for every element added to playbin.
pub fn on_element_added(element: &gst::Element) {
    let Some(factory) = element.factory() else { return };
    if !matches!(factory.name().as_str(), "dashdemux2" | "hlsdemux2") {
        return;
    }
    // Downloads run up to a minute ahead, and playback starts (or resumes after a stall) at 4 s buffered instead of 10.
    element.set_property("max-buffering-time", 60 * gst::ClockTime::SECOND.nseconds());
    element.set_property("high-watermark-time", 60 * gst::ClockTime::SECOND.nseconds());
    element.set_property("low-watermark-time", 4 * gst::ClockTime::SECOND.nseconds());
    *DEMUXER.lock().unwrap_or_else(PoisonError::into_inner) = Some(element.downgrade());
}

/// `None` without a streaming demuxer, and once it has downloaded the whole track (it then reports no level).
fn buffered_ahead() -> Option<Duration> {
    let demuxer = DEMUXER.lock().unwrap_or_else(PoisonError::into_inner).as_ref()?.upgrade()?;
    let level = demuxer.property::<u64>("current-level-time-audio");
    (level != u64::MAX).then(|| Duration::from_nanos(level))
}

fn has_headroom(s: &PlaybackState) -> bool {
    if s.status != PlaybackStatus::Playing || s.loading || s.buffering {
        return false;
    }
    let Some(ahead) = buffered_ahead() else { return true };
    let rest = s.duration.saturating_sub(s.position).saturating_sub(Duration::from_secs(2));
    ahead >= HEADROOM.min(rest)
}

/// Call before a purge: what is left of the plan must not fill the cache again.
pub fn drop_plan() {
    PLANNED.with(RefCell::take);
    let mut p = PLAN.lock().unwrap_or_else(PoisonError::into_inner);
    p.generation += 1;
    p.tracks.clear();
}

pub fn on_track_changed() {
    PENDING.set(true);
}

pub fn on_playback_state(s: &PlaybackState) {
    if s.buffering {
        // A stalled stream needs the bandwidth more than the fills; they plan again once it has headroom.
        let planned = PLANNED.with(RefCell::take);
        if !planned.is_empty() {
            let mut p = PLAN.lock().unwrap_or_else(PoisonError::into_inner);
            p.generation += 1;
            p.tracks.clear();
            drop(p);
            for id in &planned {
                audio_cache::cancel(id);
            }
            PENDING.set(true);
        }
    }
    if settings::audio_cache_limit() == 0 {
        // Turning the cache off stopped the fills; turning it on fills what plays.
        PENDING.set(true);
        return;
    }
    if !has_headroom(s) {
        return;
    }
    let next = peek_gapless().map(|e| e.track.id().to_owned());
    if !PENDING.get() && PLANNED_NEXT.with(|n| *n.borrow() == next) {
        return;
    }
    PENDING.set(false);
    PLANNED_NEXT.set(next.clone());
    let current = track_changed().with(|t| t.as_ref().map(|e| e.track.id().to_owned()));
    let mut tracks: Vec<String> = next.into_iter().chain(current).collect();
    tracks.dedup();
    // The new plan first, so the worker begins no fill of the old one after the cancels below.
    let mut p = PLAN.lock().unwrap_or_else(PoisonError::into_inner);
    p.generation += 1;
    p.tracks.clone_from(&tracks);
    p.quality = settings::audio_quality();
    PLAN_CHANGED.notify_one();
    drop(p);
    let old = PLANNED.with(|p| p.replace(tracks.clone()));
    for id in old.iter().filter(|id| !tracks.contains(id)) {
        audio_cache::cancel(id);
    }
}

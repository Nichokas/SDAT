//! Background prefetcher on top of [`tonearm_core::cache`].
//!
//! * Pages already cached render instantly and are revalidated in the background.
//! * Background prefetch fetches one page at a time with pauses, and yields
//!   whenever a page the user is looking at is loading (foreground priority).
//! * The user's own changes (add to playlist, favourites) invalidate entries.

use std::collections::{HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::PoisonError;
use std::time::Duration;

pub use tonearm_core::cache::*;

use crate::model::Track;
use crate::service::{self, Paginator};
use crate::api;

/// Full track list for `job`: from the cache, or fetched with foreground priority
/// (and then cached).
pub async fn all_tracks(job: Job) -> anyhow::Result<Vec<Track>> {
    let key = job.key();
    if let Some(t) = get(&key) {
        return Ok(t);
    }
    let _fg = Foreground::begin();
    let generation_at_start = generation(&key);
    let mut p = job.paginator();
    let tracks = api::run(move || p.get_all()).await?;
    put(&key, generation_at_start, tracks.clone());
    Ok(tracks)
}

// ------------------------------------------------------------------ background queue

/// One list to prefetch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Job {
    Playlist(String),
    Album(String),
    FavouriteTracks,
}

impl Job {
    pub fn key(&self) -> String {
        match self {
            Job::Playlist(id) => playlist_key(id),
            Job::Album(id) => album_key(id),
            Job::FavouriteTracks => FAVOURITE_TRACKS.to_owned(),
        }
    }
    pub fn paginator(&self) -> Paginator<Track> {
        match self {
            Job::Playlist(id) => service::get_playlist_tracks(id),
            Job::Album(id) => service::get_album_tracks(id),
            Job::FavouriteTracks => service::user_collection_tracks("me"),
        }
    }
}

/// FIFO of unique jobs, skipping ones already cached.
#[derive(Default)]
pub struct Queue {
    jobs: VecDeque<Job>,
    seen: HashSet<String>,
}

impl Queue {
    pub fn push(&mut self, job: Job) {
        if self.seen.insert(job.key()) {
            self.jobs.push_back(job);
        }
    }
    /// Next job whose key `is_cached` says is missing.
    pub fn pop(&mut self, is_cached: impl Fn(&str) -> bool) -> Option<Job> {
        while let Some(j) = self.jobs.pop_front() {
            if !is_cached(&j.key()) {
                return Some(j);
            }
        }
        None
    }
    pub fn len(&self) -> usize {
        self.jobs.len()
    }
}

/// Pause between background page requests.
const PAGE_PAUSE: Duration = Duration::from_millis(400);
/// How long to wait while the foreground is busy before checking again.
const YIELD_PAUSE: Duration = Duration::from_millis(300);
/// Delay after start before prefetching begins.
const START_DELAY: Duration = Duration::from_secs(5);

static STARTED: AtomicUsize = AtomicUsize::new(0);
static PREFETCH_EPOCH: AtomicU64 = AtomicU64::new(0);

fn prefetch_current(epoch: u64) -> bool {
    PREFETCH_EPOCH.load(Ordering::SeqCst) == epoch
}

fn prefetch_still_valid(epoch: u64) -> bool {
    prefetch_current(epoch) && preload_enabled() && !crate::secrets::user_id().is_empty()
}

fn clear_started_if_current(epoch: u64) {
    if prefetch_current(epoch) {
        STARTED.store(0, Ordering::SeqCst);
    }
}

// ------------------------------------------------------------- preference
// Stored as a marker file (like the country choice) so an older installed
// GSettings schema without the key can't make reads abort.

static PRELOAD_LOADED: AtomicBool = AtomicBool::new(false);
static PRELOAD: AtomicBool = AtomicBool::new(true);

fn preload_marker() -> std::path::PathBuf {
    glib::user_config_dir().join(crate::settings::CONFIG_DIR_NAME).join("preload-disabled")
}

/// Whether background preloading is enabled (default on).
pub fn preload_enabled() -> bool {
    if !PRELOAD_LOADED.swap(true, Ordering::SeqCst) {
        PRELOAD.store(!preload_marker().exists(), Ordering::SeqCst);
    }
    PRELOAD.load(Ordering::SeqCst)
}

/// Saves the preference and applies it live: turning it off stops the running
/// prefetch before its next request; turning it on starts it again.
pub fn set_preload_enabled(on: bool) {
    PRELOAD_LOADED.store(true, Ordering::SeqCst);
    PRELOAD.store(on, Ordering::SeqCst);
    let m = preload_marker();
    let r = if on {
        match std::fs::remove_file(&m) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        }
    } else {
        m.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&m, b""))
    };
    if let Err(e) = r {
        log::warn!("saving preload preference failed error={e}");
    }
    if on && !crate::secrets::user_id().is_empty() {
        start_prefetch();
    }
}

/// Lets the next Home visit prefetch again (the account changed).
pub fn allow_prefetch_again() {
    PREFETCH_EPOCH.fetch_add(1, Ordering::SeqCst);
    STARTED.store(0, Ordering::SeqCst);
}

/// Starts the background prefetcher once (after login, when Home is shown).
pub fn start_prefetch() {
    if !preload_enabled() {
        log::debug!("prefetch: disabled in preferences");
        return;
    }
    if crate::secrets::user_id().is_empty() {
        return;
    }
    if STARTED.fetch_add(1, Ordering::SeqCst) > 0 {
        return;
    }
    let epoch = PREFETCH_EPOCH.load(Ordering::SeqCst);
    glib::spawn_future_local(async move {
        glib::timeout_future(START_DELAY).await;
        if !prefetch_still_valid(epoch) {
            clear_started_if_current(epoch);
            return;
        }
        let mut queue = Queue::default();
        queue.push(Job::FavouriteTracks);
        match api::run(|| service::user_collection_playlists("me").get_all()).await {
            Ok(ps) => ps.iter().for_each(|p| queue.push(Job::Playlist(p.info.id.clone()))),
            Err(e) => log::debug!("prefetch: playlists list failed error={e:#}"),
        }
        if !prefetch_still_valid(epoch) {
            clear_started_if_current(epoch);
            return;
        }
        match api::run(|| service::user_collection_albums("me").get_all()).await {
            Ok(al) => al.iter().for_each(|a| queue.push(Job::Album(a.info.id.clone()))),
            Err(e) => log::debug!("prefetch: albums list failed error={e:#}"),
        }
        if !prefetch_still_valid(epoch) {
            clear_started_if_current(epoch);
            return;
        }
        log::debug!("prefetch: {} lists queued", queue.len());
        while let Some(job) = queue.pop(|k| get(k).is_some()) {
            if !prefetch_one(&job, epoch).await {
                log::debug!("prefetch: stopped");
                clear_started_if_current(epoch);
                return;
            }
        }
        log::debug!("prefetch: done");
    });
}

/// Returns `false` if preloading was disabled or the session changed meanwhile.
async fn prefetch_one(job: &Job, epoch: u64) -> bool {
    let key = job.key();
    let generation_at_start = generation(&key);
    let p = service::shared(job.paginator());
    prefetch_paginator(&key, generation_at_start, p, epoch, PAGE_PAUSE, YIELD_PAUSE, || {
        preload_enabled() && !crate::secrets::user_id().is_empty()
    })
    .await
}

async fn prefetch_paginator(
    key: &str,
    generation_at_start: u64,
    p: service::SharedPaginator<Track>,
    epoch: u64,
    page_pause: Duration,
    yield_pause: Duration,
    can_continue: impl Fn() -> bool,
) -> bool {
    let still_valid = || prefetch_current(epoch) && can_continue();
    loop {
        while foreground_busy() {
            if !still_valid() {
                return false;
            }
            glib::timeout_future(yield_pause).await;
        }
        if !still_valid() {
            return false;
        }
        if p.lock().unwrap_or_else(PoisonError::into_inner).is_consumed() {
            break;
        }
        if let Err(e) = service::next_page(&p).await {
            if !still_valid() {
                return false;
            }
            log::debug!("prefetch: {key} failed error={e:#}");
            return true;
        }
        if !still_valid() {
            return false;
        }
        glib::timeout_future(page_pause).await;
    }
    if !still_valid() {
        return false;
    }
    let all = p.lock().unwrap_or_else(PoisonError::into_inner).items().to_vec();
    put(key, generation_at_start, all);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        let mut t = Track::default();
        t.info.id = id.into();
        t
    }

    #[test]
    fn queue_dedups_and_skips_cached() {
        let mut q = Queue::default();
        q.push(Job::Playlist("a".into()));
        q.push(Job::Playlist("a".into()));
        q.push(Job::Album("b".into()));
        q.push(Job::FavouriteTracks);
        assert_eq!(q.len(), 3);
        let cached = |k: &str| k == "album:b";
        assert_eq!(q.pop(cached), Some(Job::Playlist("a".into())));
        assert_eq!(q.pop(cached), Some(Job::FavouriteTracks));
        assert_eq!(q.pop(cached), None);
    }

    #[test]
    fn invalidation_drops_entry_and_rejects_stale_writes() {
        let key = "playlist:test-invalidate";
        let g = generation(key);
        assert!(put(key, g, vec![track("1")]));
        assert_eq!(get(key).map(|v| v.len()), Some(1));
        // A fetch starts, then the user changes the playlist.
        let started = generation(key);
        invalidate(key);
        assert!(get(key).is_none());
        assert!(!put(key, started, vec![track("1")]), "stale result must not be cached");
        assert!(put(key, generation(key), vec![track("1"), track("2")]));
        assert_eq!(get(key).map(|v| v.len()), Some(2));
        invalidate(key);
    }

    #[test]
    fn foreground_guard_counts() {
        let g = Foreground::begin();
        assert!(foreground_busy());
        drop(g);
    }

    #[test]
    fn same_ids_compares_order() {
        assert!(same_ids(&[track("1"), track("2")], &[track("1"), track("2")]));
        assert!(!same_ids(&[track("1"), track("2")], &[track("2"), track("1")]));
        assert!(!same_ids(&[track("1")], &[]));
    }

    #[test]
    fn bumping_prefetch_epoch_stops_active_loop() {
        use std::sync::Arc;
        use crate::api::openapi::{LinksMeta, Relationships};

        let fetched = Arc::new(AtomicUsize::new(0));
        let count = Arc::clone(&fetched);
        let paginator = service::shared(Paginator::new(
            Box::new(move |_| {
                let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                if n == 1 {
                    allow_prefetch_again();
                }
                let mut page = Relationships::default();
                page.links.meta = Some(LinksMeta { next_cursor: (n < 3).then(|| format!("page-{}", n + 1)) });
                Ok(page)
            }),
            Box::new(|_| vec![track("1")]),
        ));
        let key = "playlist:test-epoch-cancel";
        invalidate(key);
        let epoch = PREFETCH_EPOCH.load(Ordering::SeqCst);
        let completed = glib::MainContext::new().block_on(prefetch_paginator(
            key,
            generation(key),
            paginator,
            epoch,
            Duration::ZERO,
            Duration::from_millis(1),
            || true,
        ));
        assert!(!completed, "loop must stop once allow_prefetch_again bumps PREFETCH_EPOCH");
        assert_eq!(fetched.load(Ordering::SeqCst), 1, "subsequent pages must not be fetched after epoch bump");
        assert!(get(key).is_none(), "cancelled prefetch must not populate cache");

        let signed_in = AtomicBool::new(true);
        let fetched2 = Arc::new(AtomicUsize::new(0));
        let count2 = Arc::clone(&fetched2);
        let paginator2 = service::shared(Paginator::new(
            Box::new(move |_| {
                let n = count2.fetch_add(1, Ordering::SeqCst) + 1;
                let mut page = Relationships::default();
                page.links.meta = Some(LinksMeta { next_cursor: (n < 3).then(|| format!("page-{}", n + 1)) });
                Ok(page)
            }),
            Box::new(|_| vec![track("2")]),
        ));
        let key2 = "playlist:test-signed-out-cancel";
        invalidate(key2);
        let epoch2 = PREFETCH_EPOCH.load(Ordering::SeqCst);
        let completed2 = glib::MainContext::new().block_on(prefetch_paginator(
            key2,
            generation(key2),
            paginator2,
            epoch2,
            Duration::ZERO,
            Duration::from_millis(1),
            || {
                let ok = signed_in.load(Ordering::SeqCst);
                if fetched2.load(Ordering::SeqCst) >= 1 {
                    signed_in.store(false, Ordering::SeqCst);
                    return false;
                }
                ok
            },
        ));
        assert!(!completed2, "loop must stop when signed out");
        assert_eq!(fetched2.load(Ordering::SeqCst), 1);
        assert!(get(key2).is_none());
    }
}

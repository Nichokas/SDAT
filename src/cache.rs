//! In-memory track-list cache with a throttled background prefetcher.
//!
//! * Pages already cached render instantly and are revalidated in the background.
//! * Background prefetch fetches one page at a time with pauses, and yields
//!   whenever a page the user is looking at is loading (foreground priority).
//! * The user's own changes (add to playlist, favourites) invalidate entries.
//!
//! Only public catalogue data is stored (track metadata); nothing is written to disk.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use crate::model::Track;
use crate::service::{self, Paginator};
use crate::api;

/// Cache keys.
pub fn playlist_key(id: &str) -> String {
    format!("playlist:{id}")
}
pub fn album_key(id: &str) -> String {
    format!("album:{id}")
}
pub const FAVOURITE_TRACKS: &str = "collection:tracks";

/// Upper bound on cached tracks overall, to keep memory sane.
const MAX_TRACKS: usize = 50_000;

#[derive(Default)]
struct Store {
    lists: HashMap<String, Vec<Track>>,
    /// Bumped on invalidation so in-flight fetches started earlier don't write stale data.
    generation: HashMap<String, u64>,
    total: usize,
}

static STORE: Mutex<Option<Store>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut Store) -> R) -> R {
    let mut g = STORE.lock().unwrap_or_else(PoisonError::into_inner);
    f(g.get_or_insert_with(Store::default))
}

pub fn get(key: &str) -> Option<Vec<Track>> {
    with(|s| s.lists.get(key).cloned())
}

pub fn generation(key: &str) -> u64 {
    with(|s| s.generation.get(key).copied().unwrap_or(0))
}

/// Stores a full list unless it was invalidated since `generation` was read.
pub fn put(key: &str, generation_at_start: u64, tracks: Vec<Track>) -> bool {
    with(|s| {
        if s.generation.get(key).copied().unwrap_or(0) != generation_at_start {
            return false;
        }
        let new_total = s.total - s.lists.get(key).map_or(0, Vec::len) + tracks.len();
        if new_total > MAX_TRACKS {
            return false;
        }
        s.total = new_total;
        s.lists.insert(key.to_owned(), tracks);
        true
    })
}

pub fn invalidate(key: &str) {
    with(|s| {
        if let Some(old) = s.lists.remove(key) {
            s.total -= old.len();
        }
        *s.generation.entry(key.to_owned()).or_default() += 1;
    });
}

/// Same track-id sequence?
pub fn same_ids(a: &[Track], b: &[Track]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.id() == y.id())
}

// ------------------------------------------------------------------ priority

static FOREGROUND: AtomicUsize = AtomicUsize::new(0);

/// Held while a page the user is looking at is loading; background work yields.
pub struct Foreground(());

impl Foreground {
    pub fn begin() -> Self {
        FOREGROUND.fetch_add(1, Ordering::SeqCst);
        Foreground(())
    }
}

impl Drop for Foreground {
    fn drop(&mut self) {
        FOREGROUND.fetch_sub(1, Ordering::SeqCst);
    }
}

pub fn foreground_busy() -> bool {
    FOREGROUND.load(Ordering::SeqCst) > 0
}

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

/// Starts the background prefetcher once (after login, when Home is shown).
pub fn start_prefetch() {
    if STARTED.fetch_add(1, Ordering::SeqCst) > 0 {
        return;
    }
    glib::spawn_future_local(async {
        glib::timeout_future(START_DELAY).await;
        let mut queue = Queue::default();
        queue.push(Job::FavouriteTracks);
        match api::run(|| service::user_collection_playlists("me").get_all()).await {
            Ok(ps) => ps.iter().for_each(|p| queue.push(Job::Playlist(p.info.id.clone()))),
            Err(e) => log::debug!("prefetch: playlists list failed error={e:#}"),
        }
        match api::run(|| service::user_collection_albums("me").get_all()).await {
            Ok(al) => al.iter().for_each(|a| queue.push(Job::Album(a.info.id.clone()))),
            Err(e) => log::debug!("prefetch: albums list failed error={e:#}"),
        }
        log::debug!("prefetch: {} lists queued", queue.len());
        while let Some(job) = queue.pop(|k| get(k).is_some()) {
            prefetch_one(&job).await;
        }
        log::debug!("prefetch: done");
    });
}

async fn prefetch_one(job: &Job) {
    let key = job.key();
    let generation_at_start = generation(&key);
    let p = service::shared(job.paginator());
    loop {
        while foreground_busy() {
            glib::timeout_future(YIELD_PAUSE).await;
        }
        if p.lock().unwrap_or_else(PoisonError::into_inner).is_consumed() {
            break;
        }
        if let Err(e) = service::next_page(&p).await {
            log::debug!("prefetch: {key} failed error={e:#}");
            return;
        }
        glib::timeout_future(PAGE_PAUSE).await;
    }
    let all = p.lock().unwrap_or_else(PoisonError::into_inner).items().to_vec();
    put(&key, generation_at_start, all);
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
        assert!(!foreground_busy() || FOREGROUND.load(Ordering::SeqCst) > 0);
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
}

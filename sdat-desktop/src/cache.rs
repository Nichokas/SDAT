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

fn prefetch_session_valid(epoch: u64, session: u64) -> bool {
    prefetch_current(epoch)
        && crate::secrets::session_generation() == session
        && crate::state::signed_in_changed().get()
}

fn prefetch_still_valid(epoch: u64, session: u64) -> bool {
    prefetch_session_valid(epoch, session) && preload_enabled()
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
    if on && crate::state::signed_in_changed().get() {
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
    if !crate::state::signed_in_changed().get() {
        return;
    }
    if STARTED.fetch_add(1, Ordering::SeqCst) > 0 {
        return;
    }
    let epoch = PREFETCH_EPOCH.load(Ordering::SeqCst);
    let session = crate::secrets::session_generation();
    glib::spawn_future_local(async move {
        glib::timeout_future(START_DELAY).await;
        if !prefetch_still_valid(epoch, session) {
            clear_started_if_current(epoch);
            return;
        }
        let mut queue = Queue::default();
        queue.push(Job::FavouriteTracks);
        match api::run(|| service::user_collection_playlists("me").get_all()).await {
            Ok(ps) => ps.iter().for_each(|p| queue.push(Job::Playlist(p.info.id.clone()))),
            Err(e) => log::debug!("prefetch: playlists list failed error={e:#}"),
        }
        if !prefetch_still_valid(epoch, session) {
            clear_started_if_current(epoch);
            return;
        }
        match api::run(|| service::user_collection_albums("me").get_all()).await {
            Ok(al) => al.iter().for_each(|a| queue.push(Job::Album(a.info.id.clone()))),
            Err(e) => log::debug!("prefetch: albums list failed error={e:#}"),
        }
        if !prefetch_still_valid(epoch, session) {
            clear_started_if_current(epoch);
            return;
        }
        log::debug!("prefetch: {} lists queued", queue.len());
        while let Some(job) = queue.pop(|k| get(k).is_some()) {
            if !prefetch_one(&job, epoch, session).await {
                log::debug!("prefetch: stopped");
                clear_started_if_current(epoch);
                return;
            }
        }
        log::debug!("prefetch: done");
    });
}

/// Returns `false` if preloading was disabled or the session changed meanwhile.
async fn prefetch_one(job: &Job, epoch: u64, session: u64) -> bool {
    let key = job.key();
    let generation_at_start = generation(&key);
    let p = service::shared(job.paginator());
    prefetch_paginator(&key, generation_at_start, p, PAGE_PAUSE, YIELD_PAUSE, || {
        prefetch_still_valid(epoch, session)
    })
    .await
}

async fn prefetch_paginator(
    key: &str,
    generation_at_start: u64,
    p: service::SharedPaginator<Track>,
    page_pause: Duration,
    yield_pause: Duration,
    still_valid: impl Fn() -> bool,
) -> bool {
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
    fn session_or_epoch_change_stops_active_prefetch() {
        use std::sync::Arc;
        use crate::api::openapi::{LinksMeta, Relationships};

        struct RestoreSignedIn(bool);
        impl Drop for RestoreSignedIn {
            fn drop(&mut self) {
                crate::state::signed_in_changed().set(self.0);
            }
        }
        let _restore = RestoreSignedIn(crate::state::signed_in_changed().get());

        let run_case = |key: &str, on_first_page: Box<dyn Fn(&glib::MainContext) + Send + Sync>| {
            crate::state::signed_in_changed().set(true);
            let ctx = glib::MainContext::new();
            let main = ctx.clone();
            let fetched = Arc::new(AtomicUsize::new(0));
            let count = Arc::clone(&fetched);
            let paginator = service::shared(Paginator::new(
                Box::new(move |_| {
                    let n = count.fetch_add(1, Ordering::SeqCst) + 1;
                    if n == 1 {
                        on_first_page(&main);
                    }
                    let mut page = Relationships::default();
                    page.links.meta = Some(LinksMeta { next_cursor: (n < 3).then(|| format!("page-{}", n + 1)) });
                    Ok(page)
                }),
                Box::new(|_| vec![track("1")]),
            ));
            invalidate(key);
            let epoch = PREFETCH_EPOCH.load(Ordering::SeqCst);
            let session = crate::secrets::session_generation();
            let completed = ctx.block_on(prefetch_paginator(
                key,
                generation(key),
                paginator,
                Duration::ZERO,
                Duration::from_millis(1),
                || prefetch_session_valid(epoch, session),
            ));
            (completed, fetched.load(Ordering::SeqCst))
        };

        let (completed, pages) = run_case("playlist:test-epoch-cancel", Box::new(|_| allow_prefetch_again()));
        assert!(!completed, "loop must stop once allow_prefetch_again bumps PREFETCH_EPOCH");
        assert_eq!(pages, 1, "subsequent pages must not be fetched after epoch bump");
        assert!(get("playlist:test-epoch-cancel").is_none(), "cancelled prefetch must not populate cache");

        let (completed, pages) = run_case(
            "playlist:test-session-cancel",
            Box::new(|_| crate::secrets::bump_session_for_test()),
        );
        assert!(!completed, "loop must stop once session_generation advances");
        assert_eq!(pages, 1, "subsequent pages must not be fetched after session change");
        assert!(get("playlist:test-session-cancel").is_none());

        let (completed, pages) = run_case(
            "playlist:test-signed-out-cancel",
            Box::new(|main| {
                let (tx, rx) = std::sync::mpsc::channel();
                main.invoke(move || {
                    crate::state::signed_in_changed().set(false);
                    let _ = tx.send(());
                });
                let _ = rx.recv();
            }),
        );
        assert!(!completed, "loop must stop when signed_in_changed becomes false");
        assert_eq!(pages, 1);
        assert!(get("playlist:test-signed-out-cancel").is_none());
    }
}

//! In-memory track-list cache (the store and the foreground-priority flag).
//!
//! The desktop app adds a throttled background prefetcher on top.
//!
//! * Pages already cached render instantly and are revalidated in the background.
//! * Background prefetch fetches one page at a time with pauses, and yields
//!   whenever a page the user is looking at is loading (foreground priority).
//! * The user's own changes (add to playlist, favourites) invalidate entries.
//!
//! Only public catalogue data is stored (track metadata); nothing is written to disk.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::model::Track;

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
    /// Bumped by [`clear`]; part of every key's generation, so it also
    /// covers fetches for keys that were never cached.
    epoch: u64,
    total: usize,
}

impl Store {
    fn generation(&self, key: &str) -> u64 {
        self.epoch + self.generation.get(key).copied().unwrap_or(0)
    }
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
    with(|s| s.generation(key))
}

/// Stores a full list unless it was invalidated since `generation` was read.
pub fn put(key: &str, generation_at_start: u64, tracks: Vec<Track>) -> bool {
    with(|s| {
        if s.generation(key) != generation_at_start {
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

/// Drops every cached list (e.g. when the country changes).
pub fn clear() {
    with(|s| {
        s.epoch += 1;
        s.lists.clear();
        s.total = 0;
    });
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
#[derive(Debug)]
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_rejects_fetches_for_keys_never_cached() {
        let key = "test:never-cached";
        let g = generation(key);
        clear();
        assert!(!put(key, g, Vec::new()), "a fetch started before clear() must not be stored");
        assert!(put(key, generation(key), Vec::new()));
    }
}

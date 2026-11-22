//! Port of `internal/state`: favourite ID caches and the signed-in state.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use crate::error::CoreResult as Result;

use crate::signals::State;
use crate::{api, global_signal};

global_signal!(pub fn signed_in_changed() -> State<bool> = State::new(crate::auth::tokens().has_refresh_token()));

/// Re-reads the keyring and notifies listeners if the signed-in state changed
/// (Go: triggerSignedInChanged). Main thread only.
pub fn notify_signed_in_changed() {
    let now = crate::auth::tokens().has_refresh_token();
    if signed_in_changed().get() != now {
        signed_in_changed().set(now);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Albums,
    Artists,
    Mixes,
    Playlists,
    Tracks,
}

#[derive(Debug, Default)]
struct Inner {
    uid: String,
    etag: String,
    items: Option<Vec<String>>,
    last_fetched: Option<Instant>,
    /// Bumped on reset/invalidation so an in-flight fetch can't store stale data.
    generation: u64,
}

impl Inner {
    fn fresh_items(&self, uid: &str) -> Option<Vec<String>> {
        if self.uid != uid {
            return None;
        }
        let (Some(items), Some(t)) = (&self.items, self.last_fetched) else {
            return None;
        };
        (t.elapsed() < TTL).then(|| items.clone())
    }
}

enum FetchOutcome {
    Items(Vec<String>, String),
    NotModified,
    Invalidated,
}

#[derive(Debug)]
pub struct FavouriteCache {
    kind: Kind,
    inner: Mutex<Inner>,
    /// Serializes network refreshes so concurrent misses coalesce without blocking `reset` or `bust`.
    fetch_lock: Mutex<()>,
}

pub static ALBUMS: FavouriteCache = FavouriteCache::new(Kind::Albums);
pub static ARTISTS: FavouriteCache = FavouriteCache::new(Kind::Artists);
pub static MIXES: FavouriteCache = FavouriteCache::new(Kind::Mixes);
pub static PLAYLISTS: FavouriteCache = FavouriteCache::new(Kind::Playlists);
pub static TRACKS: FavouriteCache = FavouriteCache::new(Kind::Tracks);

const TTL: Duration = Duration::from_mins(5);
const MAX_FETCH_ATTEMPTS: usize = 3;

impl FavouriteCache {
    const fn new(kind: Kind) -> Self {
        FavouriteCache {
            kind,
            inner: Mutex::new(Inner { uid: String::new(), etag: String::new(), items: None, last_fetched: None, generation: 0 }),
            fetch_lock: Mutex::new(()),
        }
    }

    fn v1_kind(&self) -> &'static str {
        match self.kind {
            Kind::Albums => "albums",
            Kind::Artists => "artists",
            Kind::Playlists => "playlists",
            Kind::Tracks => "tracks",
            Kind::Mixes => "mixes",
        }
    }

    /// Blocking. Call from a worker thread.
    pub fn add(&self, id: &str) -> Result<()> {
        let uid = crate::auth::tokens().user_id();
        if uid.is_empty() {
            log::debug!("not logged in, ignoring favourite addition");
            return Ok(());
        }
        let r = match self.kind {
            Kind::Mixes => api::favourite_mix_add(id),
            _ => api::favourite_add(&uid, self.v1_kind(), id),
        };
        self.bust();
        r
    }

    pub fn remove(&self, id: &str) -> Result<()> {
        let uid = crate::auth::tokens().user_id();
        if uid.is_empty() {
            log::debug!("not logged in, ignoring favourite removal");
            return Ok(());
        }
        let r = match self.kind {
            Kind::Mixes => api::favourite_mix_remove(id),
            _ => api::favourite_remove(&uid, self.v1_kind(), id),
        };
        self.bust();
        r
    }

    pub fn bust(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.last_fetched = None;
        inner.generation = inner.generation.wrapping_add(1);
    }

    /// Forgets the cached ids and `ETag` (another account's).
    pub fn reset(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        *inner = Inner { generation: inner.generation.wrapping_add(1), ..Inner::default() };
    }

    fn generation(&self) -> u64 {
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).generation
    }

    fn fetch_remote(&self, uid: &str, etag: &str, generation: u64) -> Result<FetchOutcome> {
        if self.kind == Kind::Mixes {
            let (mut list, mut etag) = api::favourite_mix_ids("", "")?;
            let mut items = std::mem::take(&mut list.content);
            while !list.cursor.is_empty() {
                if self.generation() != generation || crate::auth::tokens().user_id() != uid {
                    return Ok(FetchOutcome::Invalidated);
                }
                let (l, e) = api::favourite_mix_ids(&list.cursor, &etag)?;
                list = l;
                etag = e;
                items.append(&mut list.content);
            }
            Ok(FetchOutcome::Items(items, etag))
        } else {
            Ok(match api::favourite_ids(uid, etag)? {
                Some((lists, etag)) => {
                    let list = match self.kind {
                        Kind::Albums => lists.album,
                        Kind::Artists => lists.artist,
                        Kind::Playlists => lists.playlist,
                        _ => lists.track,
                    };
                    FetchOutcome::Items(list, etag)
                }
                None => FetchOutcome::NotModified,
            })
        }
    }

    pub fn get(&self) -> Result<Vec<String>> {
        let uid = crate::auth::tokens().user_id();
        if uid.is_empty() {
            log::debug!("not logged in, ignoring favourite fetch");
            return Ok(vec![]);
        }
        {
            let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(items) = inner.fresh_items(&uid) {
                return Ok(items);
            }
        }

        // Release `inner` before HTTP so `reset` and `bust` never wait on I/O,
        // while `fetch_lock` coalesces concurrent misses and preserves order.
        let _fetch = self.fetch_lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for _ in 0..MAX_FETCH_ATTEMPTS {
            let uid = crate::auth::tokens().user_id();
            if uid.is_empty() {
                log::debug!("not logged in, ignoring favourite fetch");
                return Ok(vec![]);
            }
            let (etag, prev_items, generation) = {
                let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(items) = inner.fresh_items(&uid) {
                    return Ok(items);
                }
                let (etag, prev_items) = if inner.uid == uid
                    && let Some(items) = &inner.items
                {
                    (inner.etag.clone(), Some(items.clone()))
                } else {
                    (String::new(), None)
                };
                (etag, prev_items, inner.generation)
            };

            let outcome = match self.fetch_remote(&uid, &etag, generation) {
                Ok(outcome) => outcome,
                Err(e) => {
                    if self.generation() != generation || crate::auth::tokens().user_id() != uid {
                        continue;
                    }
                    return Err(e);
                }
            };
            let fetched = match outcome {
                FetchOutcome::Invalidated => continue,
                FetchOutcome::Items(items, etag) => Some((items, etag)),
                FetchOutcome::NotModified => None,
            };
            if crate::auth::tokens().user_id() != uid {
                continue;
            }

            let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if inner.generation != generation {
                continue;
            }
            if let Some((items, etag)) = fetched {
                inner.uid = uid;
                inner.items = Some(items.clone());
                inner.etag = etag;
                inner.last_fetched = Some(Instant::now());
                return Ok(items);
            }
            let Some(items) = prev_items else {
                crate::error::bail!("favourite IDs returned not modified without cached items");
            };
            inner.last_fetched = Some(Instant::now());
            return Ok(items);
        }

        if crate::auth::tokens().user_id().is_empty() {
            log::debug!("not logged in, ignoring favourite fetch");
            return Ok(vec![]);
        }
        Err(crate::error::CoreError::Other("favourite cache invalidated during fetch".into()))
    }

    pub fn contains(&self, id: &str) -> Result<bool> {
        Ok(self.get()?.iter().any(|i| i == id))
    }
}

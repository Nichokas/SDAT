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

#[derive(Debug)]
struct Cached {
    uid: String,
    etag: String,
    items: Vec<String>,
    last_fetched: Instant,
}

#[derive(Debug, Default)]
struct Inner {
    cached: Option<Cached>,
    /// Bumped on reset/invalidation so an in-flight fetch can't store stale data.
    generation: u64,
}

impl Inner {
    fn fresh_items(&self, uid: &str, ttl: Duration) -> Option<Vec<String>> {
        let cached = self.cached.as_ref()?;
        (cached.uid == uid && cached.last_fetched.elapsed() < ttl).then(|| cached.items.clone())
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
            inner: Mutex::new(Inner { cached: None, generation: 0 }),
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

    /// Drops the cached ids and `ETag` so the next fetch requests a fresh list
    /// instead of revalidating stale items with `304 Not Modified`.
    pub fn bust(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        inner.cached = None;
        inner.generation = inner.generation.wrapping_add(1);
    }

    /// Forgets the cached ids and `ETag` (another account's).
    pub fn reset(&self) {
        self.bust();
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
        self.get_with(
            TTL,
            || crate::auth::tokens().user_id(),
            |uid, etag, generation| self.fetch_remote(uid, etag, generation),
        )
    }

    fn get_with(
        &self,
        ttl: Duration,
        user_id: impl Fn() -> String,
        mut fetch: impl FnMut(&str, &str, u64) -> Result<FetchOutcome>,
    ) -> Result<Vec<String>> {
        let uid = user_id();
        if uid.is_empty() {
            log::debug!("not logged in, ignoring favourite fetch");
            return Ok(vec![]);
        }
        {
            let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(items) = inner.fresh_items(&uid, ttl) {
                return Ok(items);
            }
        }

        // Release `inner` before HTTP so `reset` and `bust` never wait on I/O,
        // while `fetch_lock` coalesces concurrent misses and preserves order.
        let _fetch = self.fetch_lock.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for _ in 0..MAX_FETCH_ATTEMPTS {
            let uid = user_id();
            if uid.is_empty() {
                log::debug!("not logged in, ignoring favourite fetch");
                return Ok(vec![]);
            }
            let (etag, generation) = {
                let inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                if let Some(items) = inner.fresh_items(&uid, ttl) {
                    return Ok(items);
                }
                let etag = inner
                    .cached
                    .as_ref()
                    .filter(|c| c.uid == uid && !c.etag.is_empty())
                    .map_or_else(String::new, |c| c.etag.clone());
                (etag, inner.generation)
            };

            let outcome = match fetch(&uid, &etag, generation) {
                Ok(outcome) => outcome,
                Err(e) => {
                    if self.generation() != generation || user_id() != uid {
                        continue;
                    }
                    return Err(e);
                }
            };
            let fetched = match outcome {
                FetchOutcome::Invalidated => continue,
                FetchOutcome::Items(items, new_etag) => Some((items, new_etag)),
                FetchOutcome::NotModified => None,
            };
            if user_id() != uid {
                continue;
            }

            let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if inner.generation != generation {
                continue;
            }
            if let Some((items, new_etag)) = fetched {
                inner.cached = Some(Cached { uid, etag: new_etag, items: items.clone(), last_fetched: Instant::now() });
                return Ok(items);
            }
            if !etag.is_empty()
                && let Some(cached) = &mut inner.cached
                && cached.uid == uid
                && cached.etag == etag
            {
                cached.last_fetched = Instant::now();
                return Ok(cached.items.clone());
            }
            crate::error::bail!("favourite IDs returned not modified without a matching cached ETag");
        }

        if user_id().is_empty() {
            log::debug!("not logged in, ignoring favourite fetch");
            return Ok(vec![]);
        }
        Err(crate::error::CoreError::Other("favourite cache invalidated during fetch".into()))
    }

    pub fn contains(&self, id: &str) -> Result<bool> {
        Ok(self.get()?.iter().any(|i| i == id))
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    #[test]
    fn ttl_expired_304_refreshes_cache_and_preserves_etag() {
        let cache = FavouriteCache::new(Kind::Albums);
        let mut calls = 0usize;

        let seeded = cache
            .get_with(
                TTL,
                || "account-a".to_owned(),
                |uid, etag, _| {
                    calls += 1;
                    assert_eq!(uid, "account-a");
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::Items(vec!["album-1".to_owned()], "\"e1\"".to_owned()))
                },
            )
            .expect("seed cache");
        assert_eq!(seeded, ["album-1"]);
        assert_eq!(calls, 1);

        let hit = cache
            .get_with(TTL, || "account-a".to_owned(), |_, _, _| panic!("expected cache hit"))
            .expect("cache hit");
        assert_eq!(hit, ["album-1"]);

        let revalidated = cache
            .get_with(
                Duration::ZERO,
                || "account-a".to_owned(),
                |uid, etag, _| {
                    calls += 1;
                    assert_eq!(uid, "account-a");
                    assert_eq!(etag, "\"e1\"");
                    Ok(FetchOutcome::NotModified)
                },
            )
            .expect("304 revalidation");
        assert_eq!(revalidated, ["album-1"]);
        assert_eq!(calls, 2);

        let hit_after_304 = cache
            .get_with(TTL, || "account-a".to_owned(), |_, _, _| panic!("expected fresh cache after 304"))
            .expect("fresh hit after 304");
        assert_eq!(hit_after_304, ["album-1"]);

        let second_304 = cache
            .get_with(
                Duration::ZERO,
                || "account-a".to_owned(),
                |_, etag, _| {
                    calls += 1;
                    assert_eq!(etag, "\"e1\"");
                    Ok(FetchOutcome::NotModified)
                },
            )
            .expect("second 304 revalidation");
        assert_eq!(second_304, ["album-1"]);
        assert_eq!(calls, 3);
    }

    #[test]
    fn ttl_expired_without_etag_omits_if_none_match_and_rejects_304() {
        let cache = FavouriteCache::new(Kind::Albums);

        let seeded = cache
            .get_with(
                TTL,
                || "account-a".to_owned(),
                |_, etag, _| {
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::Items(vec!["album-1".to_owned()], String::new()))
                },
            )
            .expect("seed without etag");
        assert_eq!(seeded, ["album-1"]);
        let seeded_at = cache.inner.lock().expect("lock").cached.as_ref().expect("cached").last_fetched;

        let err = cache
            .get_with(
                Duration::ZERO,
                || "account-a".to_owned(),
                |_, etag, _| {
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::NotModified)
                },
            )
            .expect_err("unexpected 304 without etag must error");
        assert!(err.to_string().contains("not modified"), "unexpected error: {err}");
        assert_eq!(cache.inner.lock().expect("lock").cached.as_ref().expect("cached").last_fetched, seeded_at);
    }

    #[test]
    fn bust_clears_etag_and_prevents_304_restoring_stale_items() {
        let cache = FavouriteCache::new(Kind::Tracks);

        let seeded = cache
            .get_with(
                TTL,
                || "account-a".to_owned(),
                |_, etag, _| {
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::Items(vec!["track-old".to_owned()], "\"e1\"".to_owned()))
                },
            )
            .expect("seed cache");
        assert_eq!(seeded, ["track-old"]);

        cache.bust();

        let err = cache
            .get_with(
                TTL,
                || "account-a".to_owned(),
                |_, etag, _| {
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::NotModified)
                },
            )
            .expect_err("304 after bust must fail");
        assert!(err.to_string().contains("not modified"), "unexpected error: {err}");
        assert!(cache.inner.lock().expect("lock").cached.is_none());

        let fresh = cache
            .get_with(
                TTL,
                || "account-a".to_owned(),
                |_, etag, _| {
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::Items(vec!["track-new".to_owned()], "\"e2\"".to_owned()))
                },
            )
            .expect("fresh fetch after bust");
        assert_eq!(fresh, ["track-new"]);
    }

    #[test]
    fn ttl_expired_304_invalidated_by_bust_retries_without_etag() {
        let cache = FavouriteCache::new(Kind::Tracks);

        cache
            .get_with(
                TTL,
                || "account-a".to_owned(),
                |_, _, _| Ok(FetchOutcome::Items(vec!["track-old".to_owned()], "\"e1\"".to_owned())),
            )
            .expect("seed cache");

        let mut attempts = 0usize;
        let items = cache
            .get_with(
                Duration::ZERO,
                || "account-a".to_owned(),
                |_, etag, _| {
                    attempts += 1;
                    match attempts {
                        1 => {
                            assert_eq!(etag, "\"e1\"");
                            cache.bust();
                            Ok(FetchOutcome::NotModified)
                        }
                        2 => {
                            assert_eq!(etag, "");
                            Ok(FetchOutcome::Items(vec!["track-new".to_owned()], "\"e2\"".to_owned()))
                        }
                        other => panic!("unexpected attempt {other}"),
                    }
                },
            )
            .expect("retry after mid-flight bust");
        assert_eq!(items, ["track-new"]);
        assert_eq!(attempts, 2);

        let cached = cache
            .get_with(TTL, || "account-a".to_owned(), |_, _, _| panic!("expected cached retry result"))
            .expect("cached new items");
        assert_eq!(cached, ["track-new"]);
    }

    #[test]
    fn ttl_expired_304_invalidated_by_account_switch_and_reset_retries_without_etag() {
        let cache = FavouriteCache::new(Kind::Albums);
        let account = RefCell::new("account-a".to_owned());

        cache
            .get_with(
                TTL,
                || account.borrow().clone(),
                |_, _, _| Ok(FetchOutcome::Items(vec!["album-a".to_owned()], "\"e-a\"".to_owned())),
            )
            .expect("seed account-a");

        let mut attempts = 0usize;
        let items = cache
            .get_with(
                Duration::ZERO,
                || account.borrow().clone(),
                |uid, etag, _| {
                    attempts += 1;
                    match attempts {
                        1 => {
                            assert_eq!(uid, "account-a");
                            assert_eq!(etag, "\"e-a\"");
                            *account.borrow_mut() = "account-b".to_owned();
                            cache.reset();
                            Ok(FetchOutcome::NotModified)
                        }
                        2 => {
                            assert_eq!(uid, "account-b");
                            assert_eq!(etag, "");
                            Ok(FetchOutcome::Items(vec!["album-b".to_owned()], "\"e-b\"".to_owned()))
                        }
                        other => panic!("unexpected attempt {other}"),
                    }
                },
            )
            .expect("retry for account-b");
        assert_eq!(items, ["album-b"]);
        assert_eq!(attempts, 2);
    }

    #[test]
    fn ttl_expired_304_invalidated_by_sign_out_and_reset_returns_empty() {
        let cache = FavouriteCache::new(Kind::Albums);
        let account = RefCell::new("account-a".to_owned());

        cache
            .get_with(
                TTL,
                || account.borrow().clone(),
                |_, _, _| Ok(FetchOutcome::Items(vec!["album-a".to_owned()], "\"e-a\"".to_owned())),
            )
            .expect("seed account-a");

        let mut attempts = 0usize;
        let items = cache
            .get_with(
                Duration::ZERO,
                || account.borrow().clone(),
                |uid, etag, _| {
                    attempts += 1;
                    assert_eq!(uid, "account-a");
                    assert_eq!(etag, "\"e-a\"");
                    account.borrow_mut().clear();
                    cache.reset();
                    Ok(FetchOutcome::NotModified)
                },
            )
            .expect("signed out after 304");
        assert_eq!(items, Vec::<String>::new());
        assert_eq!(attempts, 1);
    }

    #[test]
    fn account_switch_without_reset_omits_previous_etag_and_rejects_304() {
        let cache = FavouriteCache::new(Kind::Artists);
        let account = RefCell::new("account-a".to_owned());

        cache
            .get_with(
                TTL,
                || account.borrow().clone(),
                |_, _, _| Ok(FetchOutcome::Items(vec!["artist-a".to_owned()], "\"e-a\"".to_owned())),
            )
            .expect("seed account-a");

        *account.borrow_mut() = "account-b".to_owned();

        let err = cache
            .get_with(
                TTL,
                || account.borrow().clone(),
                |uid, etag, _| {
                    assert_eq!(uid, "account-b");
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::NotModified)
                },
            )
            .expect_err("304 on account switch must not reuse prior account items");
        assert!(err.to_string().contains("not modified"), "unexpected error: {err}");

        let items = cache
            .get_with(
                TTL,
                || account.borrow().clone(),
                |uid, etag, _| {
                    assert_eq!(uid, "account-b");
                    assert_eq!(etag, "");
                    Ok(FetchOutcome::Items(vec!["artist-b".to_owned()], "\"e-b\"".to_owned()))
                },
            )
            .expect("fetch account-b");
        assert_eq!(items, ["artist-b"]);
    }
}

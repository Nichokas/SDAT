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
    etag: String,
    items: Option<Vec<String>>,
    last_fetched: Option<Instant>,
}

#[derive(Debug)]
pub struct FavouriteCache {
    kind: Kind,
    inner: Mutex<Inner>,
}

pub static ALBUMS: FavouriteCache = FavouriteCache::new(Kind::Albums);
pub static ARTISTS: FavouriteCache = FavouriteCache::new(Kind::Artists);
pub static MIXES: FavouriteCache = FavouriteCache::new(Kind::Mixes);
pub static PLAYLISTS: FavouriteCache = FavouriteCache::new(Kind::Playlists);
pub static TRACKS: FavouriteCache = FavouriteCache::new(Kind::Tracks);

const TTL: Duration = Duration::from_mins(5);

impl FavouriteCache {
    const fn new(kind: Kind) -> Self {
        FavouriteCache { kind, inner: Mutex::new(Inner { etag: String::new(), items: None, last_fetched: None }) }
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
        self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner).last_fetched = None;
    }

    pub fn get(&self) -> Result<Vec<String>> {
        let uid = crate::auth::tokens().user_id();
        if uid.is_empty() {
            log::debug!("not logged in, ignoring favourite fetch");
            return Ok(vec![]);
        }
        let mut inner = self.inner.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let (Some(items), Some(t)) = (&inner.items, inner.last_fetched)
            && t.elapsed() < TTL {
                return Ok(items.clone());
            }
        if self.kind == Kind::Mixes {
            let (mut list, mut etag) = api::favourite_mix_ids("", "")?;
            inner.last_fetched = Some(Instant::now());
            let mut items = std::mem::take(&mut list.content);
            while !list.cursor.is_empty() {
                let (l, e) = api::favourite_mix_ids(&list.cursor, &etag)?;
                list = l;
                etag = e;
                items.append(&mut list.content);
            }
            inner.items = Some(items);
            inner.etag = etag;
        } else {
            let res = api::favourite_ids(&uid, &inner.etag)?;
            inner.last_fetched = Some(Instant::now());
            if let Some((lists, etag)) = res {
                let list = match self.kind {
                    Kind::Albums => lists.album,
                    Kind::Artists => lists.artist,
                    Kind::Playlists => lists.playlist,
                    _ => lists.track,
                };
                inner.items = Some(list);
                inner.etag = etag;
            }
        }
        Ok(inner.items.clone().unwrap_or_default())
    }

    pub fn contains(&self, id: &str) -> Result<bool> {
        Ok(self.get()?.iter().any(|i| i == id))
    }
}

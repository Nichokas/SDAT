//! On-disk cache of whole tracks, so a replay or the next track plays without the network.

pub mod hls;

use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock, PoisonError};
use std::time::Duration;

use url::Url;

use crate::error::{CoreError, CoreResult};
use crate::model::{Normalization, TrackManifest, TrackQuality};

/// Size limits a front end offers. 0 turns the cache off: nothing is filled and everything not playing is evicted.
pub const LIMIT_PRESETS: [u64; 6] = [0, 512 << 20, 1 << 30, 2 << 30, 5 << 30, 10 << 30];
pub const LIMIT_DEFAULT: u64 = 2 << 30;

const INDEX: &str = "index.json";
const TRIES: u32 = 3;

/// A TIDAL track id: digits only, because it becomes a file name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TrackId(String);

impl TrackId {
    pub fn parse(s: &str) -> CoreResult<Self> {
        if (1..=20).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit()) {
            Ok(Self(s.to_owned()))
        } else {
            Err(CoreError::Other(format!("invalid track id {s:?}")))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// What a player needs to play a cached track without the network.
#[derive(Debug, Clone, PartialEq)]
pub struct Cached {
    pub track_id: String,
    /// The whole fragmented MP4.
    pub mp4: PathBuf,
    /// A byte-range playlist over `mp4`.
    pub playlist: PathBuf,
    /// Tier TIDAL delivered.
    pub quality: TrackQuality,
    pub formats: Vec<String>,
    pub track_norm: Option<Normalization>,
    pub album_norm: Option<Normalization>,
    pub bytes: u64,
    /// Opaque bytes the front end stored with the fill.
    pub label: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage {
    pub bytes: u64,
    pub tracks: u32,
    pub limit: u64,
}

const fn rank(q: TrackQuality) -> u8 {
    match q {
        TrackQuality::Lossy => 0,
        TrackQuality::High => 1,
        TrackQuality::Lossless => 2,
        TrackQuality::HighResLossless => 3,
    }
}

/// A copy fetched for `requested` and delivered as `delivered` plays when `wanted` asks for no more.
///
/// TIDAL delivers the best tier a track has up to the requested one, so a lossless delivery serves up to its
/// requested tier: comparing only the delivered tier would refill a CD-only track on every hi-res play. A lossy
/// delivery serves only its own tier, because TIDAL also sends AAC without a session or subscription, and that copy
/// must not stand in for lossless once the user signs in.
pub const fn serves(requested: TrackQuality, delivered: TrackQuality, wanted: TrackQuality) -> bool {
    rank(requested) >= rank(wanted) && (rank(delivered) >= rank(wanted) || rank(delivered) >= rank(TrackQuality::Lossless))
}

/// Where a fill gets its bytes.
pub trait Upstream: Send + Sync {
    fn manifest(&self, track: &TrackId, quality: TrackQuality) -> CoreResult<TrackManifest>;
    /// The body and the URL it came from after redirects, which relative playlist entries resolve against.
    fn get(&self, url: &Url, cancel: &AtomicBool) -> CoreResult<(Vec<u8>, Url)>;
}

#[derive(Debug)]
struct Tidal(reqwest::blocking::Client);

impl Upstream for Tidal {
    fn manifest(&self, track: &TrackId, quality: TrackQuality) -> CoreResult<TrackManifest> {
        crate::service::get_track_manifest_hls(track.as_str(), quality)
    }

    fn get(&self, url: &Url, cancel: &AtomicBool) -> CoreResult<(Vec<u8>, Url)> {
        let mut resp = self.0.get(url.clone()).send()?.error_for_status()?;
        let from = resp.url().clone();
        let mut body = Vec::with_capacity(resp.content_length().unwrap_or(0).min(64 << 20) as usize);
        let mut buf = vec![0; 64 * 1024];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(cancelled());
            }
            let n = resp.read(&mut buf).map_err(|e| CoreError::Network(e.to_string()))?;
            if n == 0 {
                return Ok((body, from));
            }
            body.extend_from_slice(&buf[..n]);
        }
    }
}

fn cancelled() -> CoreError {
    CoreError::Cancelled("audio cache fill cancelled".into())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Entry {
    requested: TrackQuality,
    quality: TrackQuality,
    formats: Vec<String>,
    track_norm: Option<Normalization>,
    album_norm: Option<Normalization>,
    bytes: u64,
    last_used: u64,
    #[serde(with = "base64_bytes")]
    label: Vec<u8>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
struct Index {
    clock: u64,
    entries: BTreeMap<String, Entry>,
}

mod base64_bytes {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        base64::engine::general_purpose::STANDARD.decode(String::deserialize(d)?).map_err(serde::de::Error::custom)
    }
}

fn stem(id: &str, requested: TrackQuality) -> String {
    format!("{id}-{}", requested.as_str())
}

fn evictions<'a>(entries: &[(&'a str, u64, u64)], limit: u64, protect: &[&str]) -> Vec<&'a str> {
    let mut total: u64 = entries.iter().map(|e| e.1).sum();
    let mut candidates: Vec<_> = entries.iter().filter(|e| !protect.contains(&e.0)).collect();
    candidates.sort_by_key(|e| e.2);
    let mut out = Vec::new();
    for (id, bytes, _) in candidates {
        if total <= limit {
            break;
        }
        total -= bytes;
        out.push(*id);
    }
    out
}

#[derive(Debug, Default)]
struct Fill {
    /// Names its `.part`, so a fill that replaced a cancelled one never shares its file.
    serial: u64,
    cancel: AtomicBool,
    result: Mutex<Option<CoreResult<Cached>>>,
    done: Condvar,
}

impl Fill {
    fn wait(&self) -> CoreResult<Cached> {
        let mut r = self.result.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some(r) = r.as_ref() {
                return r.clone();
            }
            r = self.done.wait(r).unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn finish(&self, r: CoreResult<Cached>) {
        *self.result.lock().unwrap_or_else(PoisonError::into_inner) = Some(r);
        self.done.notify_all();
    }
}

#[derive(Debug)]
struct State {
    index: Index,
    limit: u64,
    /// The last tracks a player looked up (what plays now and what plays next): their files are never deleted
    /// while they are here, even by `purge` or `remove`; the next `open` removes what nothing indexes.
    recent: VecDeque<String>,
    fills: HashMap<String, Arc<Fill>>,
    next_fill: u64,
}

impl State {
    fn protected(&self, id: &str) -> bool {
        self.recent.iter().any(|r| r == id)
    }
}

pub struct Store {
    dir: PathBuf,
    upstream: Box<dyn Upstream>,
    state: Mutex<State>,
}

impl std::fmt::Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Store").field("dir", &self.dir).finish_non_exhaustive()
    }
}

impl Store {
    /// Opens `dir` and reconciles it with its index: `.part` files of an interrupted fill, entries whose files are
    /// missing or truncated, and files no entry names are deleted. Opening twice changes nothing more.
    pub fn open(dir: PathBuf, limit: u64, upstream: Box<dyn Upstream>) -> CoreResult<Self> {
        fs::create_dir_all(&dir)?;
        let index = fs::read(dir.join(INDEX)).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        let index = reconcile(&dir, index)?;
        let store = Self { dir, upstream, state: Mutex::new(State { index, limit, recent: VecDeque::new(), fills: HashMap::new(), next_fill: 0 }) };
        let mut st = store.lock();
        store.evict(&mut st, None);
        store.save(&st);
        drop(st);
        Ok(store)
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn cached(&self, id: &str, e: &Entry) -> Cached {
        let stem = stem(id, e.requested);
        Cached {
            track_id: id.to_owned(),
            mp4: self.dir.join(format!("{stem}.mp4")),
            playlist: self.dir.join(format!("{stem}.m3u8")),
            quality: e.quality,
            formats: e.formats.clone(),
            track_norm: e.track_norm,
            album_norm: e.album_norm,
            bytes: e.bytes,
            label: e.label.clone(),
        }
    }

    fn save(&self, st: &State) {
        let write = || -> CoreResult<()> {
            let tmp = self.dir.join(format!("{INDEX}.tmp"));
            fs::write(&tmp, serde_json::to_vec(&st.index)?)?;
            fs::rename(tmp, self.dir.join(INDEX))?;
            Ok(())
        };
        if let Err(e) = write() {
            log::warn!("failed to save the audio cache index error={e}");
        }
    }

    fn delete_files(&self, st: &State, id: &str, requested: TrackQuality) {
        if st.protected(id) {
            return;
        }
        let stem = stem(id, requested);
        for ext in ["mp4", "m3u8"] {
            let _ = fs::remove_file(self.dir.join(format!("{stem}.{ext}")));
        }
    }

    fn evict(&self, st: &mut State, filled: Option<&str>) {
        let entries: Vec<(&str, u64, u64)> = st.index.entries.iter().map(|(id, e)| (id.as_str(), e.bytes, e.last_used)).collect();
        let mut protect: Vec<&str> = st.recent.iter().map(String::as_str).collect();
        protect.extend(filled);
        let gone: Vec<String> = evictions(&entries, st.limit, &protect).into_iter().map(str::to_owned).collect();
        for id in gone {
            if let Some(e) = st.index.entries.remove(&id) {
                self.delete_files(st, &id, e.requested);
            }
        }
    }

    /// The cached copy of `track_id` that serves `wanted`, if its files are intact. Counts as a use, and keeps the
    /// copy's files on disk while it plays.
    pub fn lookup(&self, track_id: &str, wanted: TrackQuality) -> Option<Cached> {
        let mut st = self.lock();
        st.recent.retain(|r| r != track_id);
        st.recent.push_front(track_id.to_owned());
        st.recent.truncate(2);
        let e = st.index.entries.get(track_id).filter(|e| serves(e.requested, e.quality, wanted))?;
        let c = self.cached(track_id, e);
        if !intact(&c) {
            log::warn!("cached audio of track {track_id} is missing or truncated; dropping it");
            let requested = e.requested;
            st.index.entries.remove(track_id);
            st.recent.retain(|r| r != track_id);
            self.delete_files(&st, track_id, requested);
            self.save(&st);
            return None;
        }
        st.index.clock += 1;
        let clock = st.index.clock;
        if let Some(e) = st.index.entries.get_mut(track_id) {
            e.last_used = clock;
        }
        self.save(&st);
        Some(c)
    }

    /// Downloads `track_id` at `quality` unless a cached copy already serves it. Blocking. A second call for a track
    /// that is being filled waits for that fill instead of downloading it again.
    pub fn fill(&self, track_id: &str, quality: TrackQuality, label: Vec<u8>) -> CoreResult<Cached> {
        let id = TrackId::parse(track_id)?;
        let fill = loop {
            let mut st = self.lock();
            if st.limit == 0 {
                return Err(CoreError::Cancelled("audio cache is off".into()));
            }
            if let Some(e) = st.index.entries.get(id.as_str()).filter(|e| serves(e.requested, e.quality, quality)) {
                return Ok(self.cached(id.as_str(), e));
            }
            // A cancelled fill may take a while to notice; a new request starts its own instead of failing with it.
            if let Some(running) = st.fills.get(id.as_str()).filter(|f| !f.cancel.load(Ordering::Relaxed)).map(Arc::clone) {
                drop(st);
                running.wait()?;
                continue;
            }
            st.next_fill += 1;
            let fill = Arc::new(Fill { serial: st.next_fill, ..Fill::default() });
            st.fills.insert(id.0.clone(), Arc::clone(&fill));
            break fill;
        };
        let part = self.dir.join(format!("{}.{}.part", id.as_str(), fill.serial));
        let result = self.download(&id, quality, label, &fill.cancel, &part);
        let _ = fs::remove_file(&part);
        let mut st = self.lock();
        if st.fills.get(id.as_str()).is_some_and(|f| Arc::ptr_eq(f, &fill)) {
            st.fills.remove(id.as_str());
        }
        drop(st);
        fill.finish(result.clone());
        result
    }

    fn get_with_retry(&self, url: &Url, cancel: &AtomicBool) -> CoreResult<(Vec<u8>, Url)> {
        let mut attempt = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err(cancelled());
            }
            match self.upstream.get(url, cancel) {
                Ok(r) => return Ok(r),
                Err(e @ CoreError::Cancelled(_)) => return Err(e),
                Err(e) if attempt + 1 >= TRIES => return Err(e),
                Err(e) => log::debug!("audio cache request failed, retrying error={e}"),
            }
            attempt += 1;
            std::thread::sleep(Duration::from_secs(u64::from(attempt)));
        }
    }

    fn download(&self, id: &TrackId, quality: TrackQuality, label: Vec<u8>, cancel: &AtomicBool, part: &Path) -> CoreResult<Cached> {
        use std::io::Write;
        let manifest = self.upstream.manifest(id, quality)?;
        let (mut text, mut base) = if let Some(text) = hls::decode_data_url(&manifest.url)? {
            (text, None)
        } else {
            let (body, from) = self.get_with_retry(&Url::parse(&manifest.url)?, cancel)?;
            (String::from_utf8_lossy(&body).into_owned(), Some(from))
        };
        if hls::is_master(&text) {
            let variant = hls::best_variant(&text, base.as_ref()).ok_or_else(|| hls::unsupported("master playlist"))?;
            let (body, from) = self.get_with_retry(&variant.uri, cancel)?;
            text = String::from_utf8_lossy(&body).into_owned();
            base = Some(from);
        }
        let media = hls::media(&text, base.as_ref())?;
        // Erasing the app's data deletes this folder while the app runs.
        fs::create_dir_all(&self.dir)?;
        let mut file = fs::File::create(part)?;
        let mut sizes = Vec::with_capacity(media.segments.len() + 1);
        for url in std::iter::once(&media.map).chain(media.segments.iter().map(|s| &s.uri)) {
            let (body, _) = self.get_with_retry(url, cancel)?;
            file.write_all(&body)?;
            sizes.push(body.len() as u64);
        }
        drop(file);
        let stem = stem(id.as_str(), quality);
        let mp4 = format!("{stem}.mp4");
        let playlist_tmp = self.dir.join(format!("{stem}.m3u8.tmp"));
        fs::write(&playlist_tmp, hls::local_playlist(&mp4, &media.segments, &sizes))?;

        let mut st = self.lock();
        if cancel.load(Ordering::Relaxed) || st.limit == 0 {
            let _ = fs::remove_file(playlist_tmp);
            return Err(cancelled());
        }
        fs::rename(part, self.dir.join(&mp4))?;
        fs::rename(&playlist_tmp, self.dir.join(format!("{stem}.m3u8")))?;
        st.index.clock += 1;
        let entry = Entry {
            requested: quality,
            quality: manifest.quality,
            formats: manifest.formats,
            track_norm: manifest.track_normalization,
            album_norm: manifest.album_normalization,
            bytes: sizes.iter().sum(),
            last_used: st.index.clock,
            label,
        };
        let cached = self.cached(id.as_str(), &entry);
        if let Some(old) = st.index.entries.insert(id.0.clone(), entry)
            && old.requested != quality
        {
            self.delete_files(&st, id.as_str(), old.requested);
        }
        self.evict(&mut st, Some(id.as_str()));
        self.save(&st);
        Ok(cached)
    }

    /// Stops a running fill of `track_id` at its next read; it deletes its partial file.
    pub fn cancel(&self, track_id: &str) {
        if let Some(f) = self.lock().fills.get(track_id) {
            f.cancel.store(true, Ordering::Relaxed);
        }
    }

    /// Most recently used first.
    pub fn entries(&self) -> Vec<Cached> {
        let st = self.lock();
        let mut v: Vec<(&String, &Entry)> = st.index.entries.iter().collect();
        v.sort_by_key(|(_, e)| std::cmp::Reverse(e.last_used));
        v.into_iter().map(|(id, e)| self.cached(id, e)).collect()
    }

    pub fn usage(&self) -> Usage {
        let st = self.lock();
        let bytes = st.index.entries.values().map(|e| e.bytes).sum();
        Usage { bytes, tracks: u32::try_from(st.index.entries.len()).unwrap_or(u32::MAX), limit: st.limit }
    }

    pub fn set_limit(&self, bytes: u64) {
        let mut st = self.lock();
        st.limit = bytes;
        self.evict(&mut st, None);
        self.save(&st);
    }

    pub fn remove(&self, track_id: &str) {
        let mut st = self.lock();
        if let Some(e) = st.index.entries.remove(track_id) {
            self.delete_files(&st, track_id, e.requested);
            self.save(&st);
        }
    }

    /// Drops every entry and stops every fill.
    pub fn purge(&self) {
        let mut st = self.lock();
        for f in st.fills.values() {
            f.cancel.store(true, Ordering::Relaxed);
        }
        let entries = std::mem::take(&mut st.index.entries);
        for (id, e) in entries {
            self.delete_files(&st, &id, e.requested);
        }
        self.save(&st);
    }
}

fn intact(c: &Cached) -> bool {
    fs::metadata(&c.mp4).is_ok_and(|m| m.len() == c.bytes) && c.playlist.is_file()
}

fn reconcile(dir: &Path, mut index: Index) -> CoreResult<Index> {
    let mut keep = vec![INDEX.to_owned()];
    index.entries.retain(|id, e| {
        let stem = stem(id, e.requested);
        let ok = TrackId::parse(id).is_ok()
            && fs::metadata(dir.join(format!("{stem}.mp4"))).is_ok_and(|m| m.len() == e.bytes)
            && dir.join(format!("{stem}.m3u8")).is_file();
        if ok {
            keep.push(format!("{stem}.mp4"));
            keep.push(format!("{stem}.m3u8"));
        }
        ok
    });
    for f in fs::read_dir(dir)?.flatten() {
        if !keep.iter().any(|k| f.file_name() == k.as_str()) {
            let _ = fs::remove_file(f.path());
        }
    }
    Ok(index)
}

static STORE: OnceLock<Store> = OnceLock::new();

/// Opens the process-wide cache in `dir`. A second call only applies `limit`.
pub fn install(dir: PathBuf, limit: u64) -> CoreResult<()> {
    if let Some(s) = STORE.get() {
        s.set_limit(limit);
        return Ok(());
    }
    let client = crate::http::client_builder().connect_timeout(Duration::from_secs(15)).timeout(Duration::from_secs(20)).build()?;
    let _ = STORE.set(Store::open(dir, limit, Box::new(Tidal(client)))?);
    Ok(())
}

pub fn lookup(track_id: &str, wanted: TrackQuality) -> Option<Cached> {
    STORE.get()?.lookup(track_id, wanted)
}

pub fn fill(track_id: &str, quality: TrackQuality, label: Vec<u8>) -> CoreResult<Cached> {
    STORE.get().ok_or_else(|| CoreError::Other("the audio cache is not open".into()))?.fill(track_id, quality, label)
}

pub fn cancel(track_id: &str) {
    if let Some(s) = STORE.get() {
        s.cancel(track_id);
    }
}

pub fn entries() -> Vec<Cached> {
    STORE.get().map_or_else(Vec::new, Store::entries)
}

pub fn usage() -> Usage {
    STORE.get().map_or(Usage { bytes: 0, tracks: 0, limit: 0 }, Store::usage)
}

pub fn set_limit(bytes: u64) {
    if let Some(s) = STORE.get() {
        s.set_limit(bytes);
    }
}

pub fn remove(track_id: &str) {
    if let Some(s) = STORE.get() {
        s.remove(track_id);
    }
}

pub fn purge() {
    if let Some(s) = STORE.get() {
        s.purge();
    }
}

#[cfg(test)]
mod tests;

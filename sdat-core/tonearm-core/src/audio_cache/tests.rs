use std::sync::atomic::AtomicUsize;

use super::*;

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static N: AtomicUsize = AtomicUsize::new(0);
        let p = std::env::temp_dir().join(format!("tonearm-audio-cache-{}-{}", std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
        let _ = fs::remove_dir_all(&p);
        Self(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

const SEGMENTS: usize = 3;

#[derive(Default)]
struct Fake {
    manifests: AtomicUsize,
    gets: AtomicUsize,
    /// While set, a request for the first fragment waits until it is cleared.
    hold: Mutex<bool>,
    released: Condvar,
    /// Requests for the second fragment that fail before one succeeds.
    flaky: AtomicUsize,
}

impl Fake {
    fn upstream(self: &Arc<Self>) -> Box<dyn Upstream> {
        Box::new(Shared(Arc::clone(self)))
    }

    fn requests(&self) -> usize {
        self.manifests.load(Ordering::SeqCst) + self.gets.load(Ordering::SeqCst)
    }

    fn release(&self) {
        *self.hold.lock().unwrap() = false;
        self.released.notify_all();
    }
}

fn segment(id: &str, q: &str, i: usize) -> Vec<u8> {
    format!("seg{i}:{id}:{q};").repeat(100 + i * 10).into_bytes()
}

struct Shared(Arc<Fake>);

impl Upstream for Shared {
    fn manifest(&self, track: &TrackId, quality: TrackQuality) -> CoreResult<TrackManifest> {
        self.0.manifests.fetch_add(1, Ordering::SeqCst);
        Ok(TrackManifest {
            url: format!("https://cdn.test/{}/{}/master.m3u8", track.as_str(), quality.as_str()),
            quality: if quality == TrackQuality::HighResLossless { TrackQuality::Lossless } else { quality },
            formats: vec!["FLAC".into()],
            track_normalization: Some(Normalization { peak: 0.9, gain: -3.0 }),
            album_normalization: None,
        })
    }

    fn get(&self, url: &Url, _: &AtomicBool) -> CoreResult<(Vec<u8>, Url)> {
        let f = &self.0;
        f.gets.fetch_add(1, Ordering::SeqCst);
        let parts: Vec<&str> = url.path_segments().unwrap().collect();
        let (id, q, file) = (parts[0], parts[1], parts[2]);
        let body = match file {
            "master.m3u8" => "#EXTM3U\n#EXT-X-STREAM-INF:BANDWIDTH=1,CODECS=\"flac\"\nmedia.m3u8\n".into(),
            "media.m3u8" => {
                let segments: String = (0..SEGMENTS).map(|i| format!("#EXTINF:4.0,\ns{i}.m4s\n")).collect();
                format!("#EXTM3U\n#EXT-X-TARGETDURATION:4\n#EXT-X-MAP:URI=\"init.mp4\"\n{segments}#EXT-X-ENDLIST\n").into_bytes()
            }
            "init.mp4" => format!("init:{id}:{q}").into_bytes(),
            "s0.m4s" => {
                let mut hold = f.hold.lock().unwrap();
                while *hold {
                    hold = f.released.wait(hold).unwrap();
                }
                segment(id, q, 0)
            }
            "s1.m4s" if f.flaky.load(Ordering::SeqCst) > 0 => {
                f.flaky.fetch_sub(1, Ordering::SeqCst);
                return Err(CoreError::Network("connection reset".into()));
            }
            s => segment(id, q, s[1..2].parse().unwrap()),
        };
        Ok((body, url.clone()))
    }
}

fn open(dir: &TempDir, limit: u64, fake: &Arc<Fake>) -> Store {
    Store::open(dir.0.clone(), limit, fake.upstream()).unwrap()
}

fn files(dir: &TempDir) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(&dir.0).unwrap().map(|f| f.unwrap().file_name().into_string().unwrap()).collect();
    v.sort();
    v
}

const Q: TrackQuality = TrackQuality::Lossless;

fn track_bytes() -> u64 {
    let init = "init:1:LOSSLESS".len();
    (init + (0..SEGMENTS).map(|i| segment("1", "LOSSLESS", i).len()).sum::<usize>()) as u64
}

#[test]
fn track_ids_are_digits() {
    assert_eq!(TrackId::parse("12345").unwrap().as_str(), "12345");
    for bad in ["", "12a", "../1", "1/2", "123456789012345678901"] {
        TrackId::parse(bad).unwrap_err();
    }
}

#[test]
fn serves_at_or_above_the_wanted_tier() {
    use TrackQuality::{High, HighResLossless as HiRes, Lossless, Lossy};
    assert!(serves(HiRes, HiRes, Lossless));
    assert!(serves(Lossless, Lossless, Lossless));
    assert!(serves(High, High, Lossy));
    assert!(serves(HiRes, Lossless, HiRes), "a CD-only track is as good as it gets");
    assert!(!serves(Lossless, Lossless, HiRes));
    assert!(!serves(High, High, Lossless));
    assert!(!serves(HiRes, Lossy, HiRes), "AAC sent without a session never stands in for lossless");
    assert!(!serves(HiRes, Lossy, High));
    assert!(serves(HiRes, Lossy, Lossy));
}

#[test]
fn evictions_drop_least_recently_used_but_never_protected() {
    let entries = [("a", 10, 1), ("b", 10, 3), ("c", 10, 2)];
    assert_eq!(evictions(&entries, 30, &[]), Vec::<&str>::new());
    assert_eq!(evictions(&entries, 20, &[]), ["a"]);
    assert_eq!(evictions(&entries, 15, &[]), ["a", "c"]);
    assert_eq!(evictions(&entries, 15, &["a"]), ["c", "b"]);
    assert_eq!(evictions(&entries, 0, &["b"]), ["a", "c"]);
}

#[test]
fn a_second_fill_is_free_and_a_reopened_cache_plays_offline() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    assert_eq!(store.lookup("1", Q), None);
    let first = store.fill("1", Q, b"label".to_vec()).unwrap();
    let fetched = fake.requests();
    assert_eq!(fetched, 1 + 2 + 1 + SEGMENTS, "manifest, master, media, init and every fragment once");
    assert_eq!(store.fill("1", Q, Vec::new()).unwrap(), first);
    assert_eq!(fake.requests(), fetched, "filling a cached track fetches nothing");
    drop(store);

    let offline = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &offline);
    let hit = store.lookup("1", Q).expect("cached after reopening");
    assert_eq!(offline.requests(), 0);
    assert_eq!(hit, first);
    assert_eq!(hit.label, b"label");
    assert_eq!(hit.quality, Q);
    assert_eq!(hit.formats, ["FLAC"]);
    assert_eq!(hit.track_norm, Some(Normalization { peak: 0.9, gain: -3.0 }));
    assert_eq!(hit.bytes, track_bytes());
    assert_eq!(store.usage(), Usage { bytes: track_bytes(), tracks: 1, limit: LIMIT_DEFAULT });
}

#[test]
fn the_playlist_indexes_every_part_of_the_file() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let c = open(&dir, LIMIT_DEFAULT, &fake).fill("1", Q, Vec::new()).unwrap();
    let mp4 = fs::read(&c.mp4).unwrap();
    let playlist = fs::read_to_string(&c.playlist).unwrap();
    let file = c.mp4.file_name().unwrap().to_str().unwrap();
    assert!(playlist.contains(&format!("#EXT-X-MAP:URI=\"{file}\",BYTERANGE=\"15@0\"")), "{playlist}");
    let ranges: Vec<(usize, usize)> = playlist
        .lines()
        .filter_map(|l| l.strip_prefix("#EXT-X-BYTERANGE:"))
        .map(|r| {
            let (n, o) = r.split_once('@').unwrap();
            (n.parse().unwrap(), o.parse().unwrap())
        })
        .collect();
    assert_eq!(ranges.len(), SEGMENTS);
    for (i, (n, o)) in ranges.into_iter().enumerate() {
        assert_eq!(mp4[o..o + n], segment("1", "LOSSLESS", i));
    }
    assert!(playlist.ends_with("#EXT-X-ENDLIST\n"));
}

#[test]
fn concurrent_fills_of_one_track_fetch_it_once() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake { hold: Mutex::new(true), ..Fake::default() });
    let store = Arc::new(open(&dir, LIMIT_DEFAULT, &fake));
    let spawn = || {
        let s = Arc::clone(&store);
        std::thread::spawn(move || s.fill("1", Q, Vec::new()))
    };
    let first = spawn();
    while store.lock().fills.get("1").is_none_or(|f| Arc::strong_count(f) < 2) {
        std::thread::yield_now();
    }
    let second = spawn();
    while store.lock().fills.get("1").is_some_and(|f| Arc::strong_count(f) < 3) {
        std::thread::yield_now();
    }
    fake.release();
    let (a, b) = (first.join().unwrap().unwrap(), second.join().unwrap().unwrap());
    assert_eq!(a, b);
    assert_eq!(fake.manifests.load(Ordering::SeqCst), 1);
    assert_eq!(fake.requests(), 1 + 2 + 1 + SEGMENTS);
}

#[test]
fn a_cancelled_fill_leaves_nothing_behind() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake { hold: Mutex::new(true), ..Fake::default() });
    let store = Arc::new(open(&dir, LIMIT_DEFAULT, &fake));
    let s = Arc::clone(&store);
    let fill = std::thread::spawn(move || s.fill("1", Q, Vec::new()));
    while store.lock().fills.get("1").is_none_or(|f| Arc::strong_count(f) < 2) {
        std::thread::yield_now();
    }
    store.cancel("1");
    fake.release();
    assert!(matches!(fill.join().unwrap(), Err(CoreError::Cancelled(_))));
    assert_eq!(store.usage().tracks, 0);
    assert_eq!(files(&dir), [INDEX]);
    assert!(store.lock().fills.is_empty());
}

#[test]
fn a_cancel_after_begin_stops_the_fill_before_any_request() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let job = store.begin("1", Q, Vec::new()).unwrap();
    store.cancel("1");
    assert!(matches!(job.finish(), Err(CoreError::Cancelled(_))));
    assert_eq!(fake.requests(), 0);
    assert_eq!(files(&dir), [INDEX]);
}

#[test]
fn turning_the_cache_off_stops_a_running_fill() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake { hold: Mutex::new(true), ..Fake::default() });
    let store = Arc::new(open(&dir, LIMIT_DEFAULT, &fake));
    let s = Arc::clone(&store);
    let fill = std::thread::spawn(move || s.fill("1", Q, Vec::new()));
    while store.lock().fills.get("1").is_none_or(|f| Arc::strong_count(f) < 2) {
        std::thread::yield_now();
    }
    store.set_limit(0);
    fake.release();
    assert!(matches!(fill.join().unwrap(), Err(CoreError::Cancelled(_))));
    let gets = fake.requests();
    assert!(gets < 1 + 2 + 1 + SEGMENTS, "stopped before the last fragment, after {gets} requests");
}

#[test]
fn a_purge_after_playback_stopped_leaves_no_audio() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    store.fill("1", Q, Vec::new()).unwrap();
    store.lookup("1", Q).unwrap();
    store.purge(true);
    store.fill("2", Q, Vec::new()).unwrap();
    store.lookup("2", Q).unwrap();
    store.purge(false);
    assert_eq!(files(&dir), [INDEX]);
}

#[test]
fn a_removed_track_does_not_come_back_from_its_running_fill() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake { hold: Mutex::new(true), ..Fake::default() });
    let store = Arc::new(open(&dir, LIMIT_DEFAULT, &fake));
    let s = Arc::clone(&store);
    let fill = std::thread::spawn(move || s.fill("1", Q, Vec::new()));
    while store.lock().fills.get("1").is_none_or(|f| Arc::strong_count(f) < 2) {
        std::thread::yield_now();
    }
    store.remove("1");
    fake.release();
    assert!(matches!(fill.join().unwrap(), Err(CoreError::Cancelled(_))));
    assert_eq!(store.usage().tracks, 0);
}

#[test]
fn a_fill_after_a_cancel_starts_its_own_instead_of_failing() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake { hold: Mutex::new(true), ..Fake::default() });
    let store = Arc::new(open(&dir, LIMIT_DEFAULT, &fake));
    let s = Arc::clone(&store);
    let first = std::thread::spawn(move || s.fill("1", Q, Vec::new()));
    while store.lock().fills.get("1").is_none_or(|f| Arc::strong_count(f) < 2) {
        std::thread::yield_now();
    }
    store.cancel("1");
    let s = Arc::clone(&store);
    let second = std::thread::spawn(move || s.fill("1", Q, Vec::new()));
    while store.lock().fills.get("1").is_none_or(|f| f.cancel.load(Ordering::Relaxed)) {
        std::thread::yield_now();
    }
    fake.release();
    assert!(matches!(first.join().unwrap(), Err(CoreError::Cancelled(_))));
    second.join().unwrap().expect("the new fill is not the cancelled one");
    assert_eq!(store.usage().tracks, 1);
    assert_eq!(files(&dir).len(), 3, "index, mp4 and playlist; no stray .part");
    assert!(store.lock().fills.is_empty());
}

#[test]
fn a_fill_recreates_a_folder_the_os_emptied() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    std::fs::remove_dir_all(&dir.0).unwrap();
    store.fill("1", Q, Vec::new()).expect("fills after the folder is gone");
    assert!(store.lookup("1", Q).is_some());
}

#[test]
fn a_failed_request_is_retried() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake { flaky: AtomicUsize::new(1), ..Fake::default() });
    open(&dir, LIMIT_DEFAULT, &fake).fill("1", Q, Vec::new()).unwrap();
    assert_eq!(fake.requests(), 1 + 2 + 1 + SEGMENTS + 1);
}

#[test]
fn eviction_keeps_what_plays_and_drops_the_oldest() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, track_bytes() * 2 + 100, &fake);
    for id in ["1", "2"] {
        store.fill(id, Q, Vec::new()).unwrap();
    }
    store.lookup("1", Q).unwrap();
    store.fill("3", Q, Vec::new()).unwrap();
    let ids: Vec<String> = store.entries().into_iter().map(|c| c.track_id).collect();
    assert_eq!(ids, ["3", "1"], "most recently used first; 2 was the least recently used");
    assert!(!files(&dir).iter().any(|f| f.starts_with("2-")));

    store.set_limit(0);
    assert_eq!(store.entries().into_iter().map(|c| c.track_id).collect::<Vec<_>>(), ["1"], "the playing track stays");
    assert!(matches!(store.fill("4", Q, Vec::new()), Err(CoreError::Cancelled(_))), "off means no fills");
}

#[test]
fn a_higher_tier_refills_and_replaces_the_copy() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    store.fill("1", Q, Vec::new()).unwrap();
    assert_eq!(store.lookup("1", TrackQuality::HighResLossless), None);
    let low = store.lookup("1", TrackQuality::High).expect("a CD copy serves AAC");
    let before = fake.requests();
    let hi = store.fill("1", TrackQuality::HighResLossless, Vec::new()).unwrap();
    assert!(fake.requests() > before);
    assert_eq!(store.usage().tracks, 1);
    assert_eq!(hi.quality, TrackQuality::Lossless, "delivered tier");
    assert_eq!(store.lookup("1", TrackQuality::HighResLossless), Some(hi.clone()), "the requested tier decides");
    assert!(low.mp4.exists(), "the copy that was looked up may still be playing");
    drop(store);
    open(&dir, LIMIT_DEFAULT, &fake);
    assert!(!low.mp4.exists() && hi.mp4.exists());
}

#[test]
fn reconcile_cleans_up_once_and_then_changes_nothing() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    store.fill("1", Q, Vec::new()).unwrap();
    let truncated = store.fill("2", Q, Vec::new()).unwrap();
    drop(store);
    fs::write(&truncated.mp4, b"short").unwrap();
    fs::write(dir.0.join("3-9.part"), b"interrupted").unwrap();
    fs::write(dir.0.join("4-9.mp4"), b"unindexed").unwrap();
    fs::write(dir.0.join("notes.txt"), b"not the cache's").unwrap();

    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let kept = store.entries();
    assert_eq!(kept.iter().map(|c| c.track_id.as_str()).collect::<Vec<_>>(), ["1"]);
    drop(store);
    let (listing, index) = (files(&dir), fs::read(dir.0.join(INDEX)).unwrap());
    let name = |p: &PathBuf| p.file_name().unwrap().to_str().unwrap().to_owned();
    assert_eq!(listing, [name(&kept[0].playlist), name(&kept[0].mp4), INDEX.to_owned(), "notes.txt".to_owned()]);
    drop(open(&dir, LIMIT_DEFAULT, &fake));
    assert_eq!((files(&dir), fs::read(dir.0.join(INDEX)).unwrap()), (listing, index));
}

#[test]
fn a_lookup_drops_a_copy_whose_file_vanished() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let c = store.fill("1", Q, Vec::new()).unwrap();
    fs::remove_file(&c.mp4).unwrap();
    assert_eq!(store.lookup("1", Q), None);
    assert_eq!(store.usage().tracks, 0);
    assert!(!c.playlist.exists());
}

#[test]
fn a_fill_downloads_again_a_copy_whose_file_vanished() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let c = store.fill("1", Q, Vec::new()).unwrap();
    fs::remove_file(&c.mp4).unwrap();
    let again = store.fill("1", Q, Vec::new()).unwrap();
    assert!(again.mp4.exists() && again.playlist.exists());
    assert_eq!(fs::metadata(&again.mp4).unwrap().len(), again.bytes);
}

#[test]
fn a_new_fill_never_writes_over_the_playing_copy() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let playing = store.fill("1", Q, Vec::new()).unwrap();
    store.lookup("1", Q).unwrap();
    let (mp4, playlist) = (fs::read(&playing.mp4).unwrap(), fs::read(&playing.playlist).unwrap());
    store.purge(true);
    let refilled = store.fill("1", Q, Vec::new()).unwrap();
    assert_ne!(refilled.mp4, playing.mp4);
    assert_eq!((fs::read(&playing.mp4).unwrap(), fs::read(&playing.playlist).unwrap()), (mp4, playlist));
}

#[test]
fn a_track_that_stops_playing_leaves_a_cache_that_is_off() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    for id in ["1", "2"] {
        store.fill(id, Q, Vec::new()).unwrap();
        store.lookup(id, Q).unwrap();
    }
    store.set_limit(0);
    assert_eq!(store.usage().tracks, 2, "both may still play");
    store.lookup("3", Q);
    store.lookup("4", Q);
    assert_eq!(store.usage().tracks, 0);
    assert_eq!(files(&dir), [INDEX]);
}

#[test]
fn purge_and_remove_keep_the_playing_files() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let (a, b, c) = (store.fill("1", Q, Vec::new()).unwrap(), store.fill("2", Q, Vec::new()).unwrap(), store.fill("3", Q, Vec::new()).unwrap());
    store.lookup("1", Q).unwrap();
    store.remove("3");
    assert!(!c.mp4.exists());
    store.purge(true);
    assert_eq!(store.usage(), Usage { bytes: 0, tracks: 0, limit: LIMIT_DEFAULT });
    assert_eq!(store.lookup("1", Q), None);
    assert!(a.mp4.exists() && a.playlist.exists(), "still playing");
    assert!(!b.mp4.exists() && !b.playlist.exists());
    store.lookup("5", Q);
    assert!(a.mp4.exists(), "still the previous track");
    store.lookup("6", Q);
    assert_eq!(files(&dir), [INDEX], "gone once it stops playing");
}

#[test]
fn a_removed_copy_that_failed_goes_once_its_track_stops_playing() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let failed = store.fill("1", Q, Vec::new()).unwrap();
    let next = store.fill("2", Q, Vec::new()).unwrap();
    store.lookup("1", Q).unwrap();
    store.remove("1");
    assert!(failed.mp4.exists());
    store.lookup("2", Q).unwrap();
    store.lookup("3", Q);
    assert!(!failed.mp4.exists() && !failed.playlist.exists());
    assert!(next.mp4.exists() && next.playlist.exists(), "indexed files stay");
}

#[test]
fn a_job_dropped_unfinished_releases_the_fill_that_joined_it() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = Arc::new(open(&dir, LIMIT_DEFAULT, &fake));
    let job = store.begin("1", Q, Vec::new()).unwrap();
    let s = Arc::clone(&store);
    let joined = std::thread::spawn(move || s.fill("1", Q, Vec::new()));
    while store.lock().fills.get("1").is_none_or(|f| Arc::strong_count(f) < 3) {
        std::thread::yield_now();
    }
    drop(job);
    assert!(matches!(joined.join().unwrap(), Err(CoreError::Cancelled(_))));
    assert!(store.lock().fills.is_empty());
    assert!(store.fill("1", Q, Vec::new()).is_ok());
}

#[test]
fn a_request_for_a_higher_tier_supersedes_a_lower_fill() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    let low = store.begin("1", TrackQuality::High, Vec::new()).unwrap();
    let joined = store.begin("1", TrackQuality::High, Vec::new()).unwrap();
    let high = store.begin("1", TrackQuality::Lossless, Vec::new()).unwrap();
    let lower = store.begin("1", TrackQuality::High, Vec::new()).unwrap();
    assert_eq!(high.finish().unwrap().quality, TrackQuality::Lossless);
    for job in [low, joined, lower] {
        assert_eq!(job.finish().unwrap().quality, TrackQuality::Lossless, "the higher copy serves the lower requests");
    }
    assert_eq!(store.usage().tracks, 1);
    assert_eq!(fake.manifests.load(Ordering::SeqCst), 1, "the superseded fill never started");
}

#[test]
fn a_hit_that_a_purge_dropped_before_finish_is_cancelled() {
    let dir = TempDir::new();
    let fake = Arc::new(Fake::default());
    let store = open(&dir, LIMIT_DEFAULT, &fake);
    store.fill("1", Q, Vec::new()).unwrap();
    let job = store.begin("1", Q, Vec::new()).unwrap();
    store.purge(false);
    assert!(matches!(job.finish(), Err(CoreError::Cancelled(_))));
}

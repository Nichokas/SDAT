use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use super::{Event, Level};

pub(super) const CAPACITY: usize = 2000;
const MAX_FILE_BYTES: u64 = 512 * 1024;
const CURRENT: &str = "diag.log";
const PREVIOUS: &str = "diag.prev.log";
const ROTATED: &str = "diag.log.1";

/// The event log: the last [`CAPACITY`] events in memory, and with a directory also a rotating text file.
/// File errors are dropped on purpose: logging must never fail the app or log about itself.
pub(super) struct Store {
    ring: VecDeque<Event>,
    next_seq: u64,
    file: Option<LogFile>,
}

struct LogFile {
    dir: PathBuf,
    file: Option<File>,
    size: u64,
}

impl LogFile {
    fn open(dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&dir);
        let _ = fs::rename(dir.join(CURRENT), dir.join(PREVIOUS));
        let mut log = Self { dir, file: None, size: 0 };
        log.start_new();
        log
    }

    fn start_new(&mut self) {
        self.file = OpenOptions::new().create(true).append(true).open(self.dir.join(CURRENT)).ok();
        self.size = 0;
    }

    fn append(&mut self, text: &str) {
        let Some(file) = self.file.as_mut() else { return };
        if file.write_all(text.as_bytes()).is_err() {
            return;
        }
        self.size += text.len() as u64;
        if self.size > MAX_FILE_BYTES {
            self.file = None;
            let _ = fs::rename(self.dir.join(CURRENT), self.dir.join(ROTATED));
            self.start_new();
        }
    }

    fn empty(&mut self) {
        let _ = fs::remove_file(self.dir.join(PREVIOUS));
        let _ = fs::remove_file(self.dir.join(ROTATED));
        self.file = None;
        let _ = fs::remove_file(self.dir.join(CURRENT));
        self.start_new();
    }
}

impl Store {
    /// With a directory, moves the last run's `diag.log` to `diag.prev.log` and starts a new `diag.log`.
    pub(super) fn new(dir: Option<PathBuf>) -> Self {
        Self { ring: VecDeque::with_capacity(CAPACITY), next_seq: 1, file: dir.map(LogFile::open) }
    }

    pub(super) fn push(&mut self, level: Level, target: &str, message: String, backtrace: Option<String>) -> &Event {
        let event = Event {
            seq: self.next_seq,
            at_ms: chrono::Utc::now().timestamp_millis(),
            level,
            target: target.to_owned(),
            message,
            backtrace,
        };
        self.next_seq += 1;
        if let Some(file) = self.file.as_mut() {
            file.append(&format!("{event}\n"));
        }
        if self.ring.len() == CAPACITY {
            self.ring.pop_front();
        }
        self.ring.push_back(event);
        self.ring.back().expect("just pushed")
    }

    pub(super) fn since(&self, seq: u64) -> Vec<Event> {
        let first_newer = self.ring.partition_point(|e| e.seq <= seq);
        self.ring.iter().skip(first_newer).cloned().collect()
    }

    pub(super) fn previous_session(&self) -> String {
        let Some(file) = &self.file else { return String::new() };
        fs::read(file.dir.join(PREVIOUS)).map_or_default(|b| String::from_utf8_lossy(&b).into_owned())
    }

    /// Empties the buffer and the files. `seq` keeps counting.
    pub(super) fn clear(&mut self) {
        self.ring.clear();
        if let Some(file) = self.file.as_mut() {
            file.empty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("tonearm-diag-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
            let _ = fs::remove_file(&self.0);
        }
    }

    fn info(store: &mut Store, message: &str) {
        store.push(Level::Info, "test", message.to_owned(), None);
    }

    #[test]
    fn the_buffer_keeps_only_the_last_events_and_seq_keeps_rising() {
        let mut store = Store::new(None);
        for i in 0..CAPACITY + 500 {
            info(&mut store, &format!("event {i}"));
        }
        let all = store.since(0);
        assert_eq!(all.len(), CAPACITY);
        assert_eq!(all[0].message, "event 500");
        assert_eq!(all.last().map(|e| e.message.as_str()), Some("event 2499"));
        assert!(all.windows(2).all(|w| w[0].seq < w[1].seq));
    }

    #[test]
    fn since_returns_only_newer_events_oldest_first() {
        let mut store = Store::new(None);
        for i in 0..5 {
            info(&mut store, &format!("e{i}"));
        }
        let all = store.since(0);
        let third = all[2].seq;
        let newer: Vec<_> = store.since(third).into_iter().map(|e| e.message).collect();
        assert_eq!(newer, ["e3", "e4"]);
        assert_eq!(store.since(all[4].seq).len(), 0);
        assert_eq!(store.since(u64::MAX).len(), 0);
    }

    #[test]
    fn clear_empties_the_buffer_but_seq_continues() {
        let mut store = Store::new(None);
        info(&mut store, "a");
        let before = store.since(0)[0].seq;
        store.clear();
        assert_eq!(store.since(0).len(), 0);
        info(&mut store, "b");
        assert!(store.since(0)[0].seq > before);
    }

    #[test]
    fn the_next_run_reads_the_last_runs_log() {
        let tmp = TempDir::new("prev");
        let mut first = Store::new(Some(tmp.0.clone()));
        assert_eq!(first.previous_session(), "");
        first.push(Level::Error, "net", "boom\nsecond line".into(), Some("frame 1\nframe 2".into()));
        drop(first);

        let mut second = Store::new(Some(tmp.0.clone()));
        let prev = second.previous_session();
        assert!(prev.contains("ERROR [net] boom"), "{prev}");
        assert!(prev.contains("\n    second line"), "{prev}");
        assert!(prev.contains("\n    frame 2"), "{prev}");
        info(&mut second, "this run");
        assert!(!second.previous_session().contains("this run"));
        let current = fs::read_to_string(tmp.0.join(CURRENT)).expect("diag.log");
        assert!(current.contains("this run") && !current.contains("boom"));

        let third = Store::new(Some(tmp.0.clone()));
        assert!(third.previous_session().contains("this run"));
        assert!(!third.previous_session().contains("boom"));
    }

    #[test]
    fn a_log_over_512_kib_rotates_to_a_second_file() {
        let tmp = TempDir::new("rotate");
        let mut store = Store::new(Some(tmp.0.clone()));
        let line = "x".repeat(400);
        for i in 0..2000 {
            info(&mut store, &format!("{i} {line}"));
        }
        let rotated = fs::metadata(tmp.0.join(ROTATED)).expect("diag.log.1").len();
        let current = fs::metadata(tmp.0.join(CURRENT)).expect("diag.log").len();
        assert!(rotated > MAX_FILE_BYTES, "{rotated}");
        assert!(current > 0 && current <= MAX_FILE_BYTES, "{current}");
    }

    #[test]
    fn clear_removes_the_files_content_and_the_previous_session() {
        let tmp = TempDir::new("clear");
        let mut first = Store::new(Some(tmp.0.clone()));
        info(&mut first, "old run");
        drop(first);
        let mut store = Store::new(Some(tmp.0.clone()));
        info(&mut store, "this run");
        assert_ne!(store.previous_session().len(), 0);
        store.clear();
        assert_eq!(store.previous_session(), "");
        assert_eq!(fs::read_to_string(tmp.0.join(CURRENT)).expect("diag.log"), "");
        info(&mut store, "after");
        assert!(fs::read_to_string(tmp.0.join(CURRENT)).expect("diag.log").contains("after"));
    }

    #[test]
    fn an_unusable_directory_is_not_an_error() {
        let tmp = TempDir::new("bad");
        fs::write(&tmp.0, "a file, not a directory").expect("write");
        let mut store = Store::new(Some(tmp.0.join("sub")));
        info(&mut store, "still logged in memory");
        assert_eq!(store.since(0).len(), 1);
        assert_eq!(store.previous_session(), "");
    }
}

use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;

use super::{Event, Level};

pub(super) const CAPACITY: usize = 2000;
const MAX_FILE_BYTES: u64 = 512 * 1024;
const CURRENT: &str = "diag.log";
const PREVIOUS: &str = "diag.prev.log";
const ROTATED: &str = "diag.log.1";
const PREVIOUS_ROTATED: &str = "diag.prev.log.1";
const LOCK: &str = "diag.lock";
/// Longest message or backtrace kept, so one huge event (an error page logged whole) neither overflows
/// [`MAX_FILE_BYTES`] nor holds the buffer's memory.
const MAX_TEXT_BYTES: usize = 32 * 1024;

fn truncated(mut text: String) -> String {
    if text.len() > MAX_TEXT_BYTES {
        text.truncate(text.floor_char_boundary(MAX_TEXT_BYTES));
        text.push_str(" …(truncated)");
    }
    text
}

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
    /// Held for the life of the process; the lock goes with it.
    _lock: File,
}

impl LogFile {
    /// `None` when another process already logs to `dir`. A second desktop launch runs this before GTK
    /// hands it to the running instance, and must not rotate that instance's live `diag.log` away.
    fn open(dir: PathBuf) -> Option<Self> {
        let _ = fs::create_dir_all(&dir);
        let lock = OpenOptions::new().create(true).truncate(false).write(true).open(dir.join(LOCK)).ok()?;
        // SAFETY: `lock` is an open file, so its descriptor stays valid for the call.
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 {
            return None;
        }
        // A `diag.log.1` can only come from the last run, because each start moves it aside.
        let _ = fs::remove_file(dir.join(PREVIOUS_ROTATED));
        let _ = fs::rename(dir.join(ROTATED), dir.join(PREVIOUS_ROTATED));
        let _ = fs::rename(dir.join(CURRENT), dir.join(PREVIOUS));
        let mut log = Self { dir, file: None, size: 0, _lock: lock };
        log.start_new();
        Some(log)
    }

    fn start_new(&mut self) {
        self.file = OpenOptions::new().create(true).append(true).open(self.dir.join(CURRENT)).ok();
        self.size = self.file.as_ref().and_then(|f| f.metadata().ok()).map_or(0, |m| m.len());
    }

    /// Rotates before writing, so the newest event (a panic, say) is always in `diag.log`.
    fn append(&mut self, text: &str) {
        if self.size > 0 && self.size + text.len() as u64 > MAX_FILE_BYTES {
            self.file = None;
            let _ = fs::rename(self.dir.join(CURRENT), self.dir.join(ROTATED));
            self.start_new();
        }
        let Some(file) = self.file.as_mut() else { return };
        if file.write_all(text.as_bytes()).is_ok() {
            self.size += text.len() as u64;
        } else if let Ok(meta) = file.metadata() {
            // A failed write may still have written part of the event: count what is really on disk.
            self.size = meta.len();
        }
    }

    fn empty(&mut self) {
        for name in [PREVIOUS, PREVIOUS_ROTATED, ROTATED] {
            let _ = fs::remove_file(self.dir.join(name));
        }
        self.file = None;
        let _ = fs::remove_file(self.dir.join(CURRENT));
        self.start_new();
    }
}

impl Store {
    /// With a directory, moves the last run's `diag.log` to `diag.prev.log` and starts a new `diag.log`.
    pub(super) fn new(dir: Option<PathBuf>) -> Self {
        Self { ring: VecDeque::with_capacity(CAPACITY), next_seq: 1, file: dir.and_then(LogFile::open) }
    }

    pub(super) fn push(&mut self, level: Level, target: &str, message: String, backtrace: Option<String>) -> &Event {
        let event = Event {
            seq: self.next_seq,
            at_ms: chrono::Utc::now().timestamp_millis(),
            level,
            target: target.to_owned(),
            message: truncated(message),
            backtrace: backtrace.map(truncated),
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

    /// The last run's log, its rotated older part first.
    pub(super) fn previous_session(&self) -> String {
        let Some(file) = &self.file else { return String::new() };
        [PREVIOUS_ROTATED, PREVIOUS]
            .iter()
            .filter_map(|name| fs::read(file.dir.join(name)).ok())
            .map(|b| String::from_utf8_lossy(&b).into_owned())
            .collect()
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
        drop(second);

        let third = Store::new(Some(tmp.0.clone()));
        assert!(third.previous_session().contains("this run"));
        assert!(!third.previous_session().contains("boom"));
    }

    #[test]
    fn a_second_process_leaves_the_running_ones_log_alone() {
        let tmp = TempDir::new("second");
        let mut running = Store::new(Some(tmp.0.clone()));
        info(&mut running, "live session");
        let second = Store::new(Some(tmp.0.clone()));
        assert_eq!(second.previous_session(), "");
        drop(second);
        info(&mut running, "still live");
        let current = fs::read_to_string(tmp.0.join(CURRENT)).expect("diag.log");
        assert!(current.contains("live session") && current.contains("still live"), "{current}");
        drop(running);

        let next = Store::new(Some(tmp.0.clone()));
        assert!(next.previous_session().contains("still live"));
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
        assert!(rotated > 0 && rotated <= MAX_FILE_BYTES, "{rotated}");
        assert!(current > 0 && current <= MAX_FILE_BYTES, "{current}");
        assert!(fs::read_to_string(tmp.0.join(CURRENT)).expect("diag.log").contains("1999 "));
    }

    #[test]
    fn one_huge_event_is_cut_to_fit_the_file() {
        let tmp = TempDir::new("huge");
        let mut store = Store::new(Some(tmp.0.clone()));
        info(&mut store, &"é".repeat(MAX_FILE_BYTES as usize));
        let message = &store.since(0)[0].message;
        assert!(message.len() < MAX_TEXT_BYTES + 32 && message.ends_with("…(truncated)"));
        let current = fs::metadata(tmp.0.join(CURRENT)).expect("diag.log").len();
        assert!(current < MAX_FILE_BYTES, "{current}");
    }

    #[test]
    fn the_next_run_reads_both_parts_of_a_rotated_log() {
        let tmp = TempDir::new("rotate-prev");
        let mut first = Store::new(Some(tmp.0.clone()));
        let line = "x".repeat(400);
        info(&mut first, "first event");
        for i in 0..2000 {
            info(&mut first, &format!("{i} {line}"));
        }
        first.push(Level::Error, "panic", "the crash".into(), Some("frame".into()));
        drop(first);

        let second = Store::new(Some(tmp.0.clone()));
        let prev = second.previous_session();
        assert!(prev.contains("ERROR [panic] the crash"));
        assert!(prev.rfind("the crash") > prev.find("1999 "), "the crash comes last");
        assert!(!tmp.0.join(ROTATED).exists());
        drop(second);

        let third = Store::new(Some(tmp.0.clone()));
        assert!(!third.previous_session().contains("the crash"));
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

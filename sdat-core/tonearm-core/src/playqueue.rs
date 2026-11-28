//! Port of `internal/player/playqueue`.
use crate::error::{bail, CoreResult as Result};
use rand::seq::SliceRandom;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

use crate::model::Track;
use crate::signals::State;

/// Ephemeral FIFO queue whose entries are observable.
#[derive(Debug)]
pub struct Queue {
    entries: State<Vec<Track>>,
}

impl Default for Queue {
    fn default() -> Self {
        Queue { entries: State::new(Vec::new()) }
    }
}

impl Queue {
    pub fn append(&self, t: Track) {
        self.entries.notify(|mut v| {
            v.push(t);
            v
        });
    }
    pub fn clear(&self) {
        self.entries.set(Vec::new());
    }
    #[cfg(test)]
    pub fn contains(&self, id: &str) -> bool {
        self.entries.with(|v| v.iter().any(|t| t.id() == id))
    }
    pub fn entries(&self) -> &State<Vec<Track>> {
        &self.entries
    }
    pub fn len(&self) -> usize {
        self.entries.with(std::vec::Vec::len)
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, i: usize) -> Option<Track> {
        self.entries.with(|v| v.get(i).cloned())
    }
    #[cfg(test)]
    pub fn insert(&self, t: Track, i: usize) -> Result<()> {
        let n = self.len();
        if i > n {
            bail!("invalid index, must be between 0 and {n}");
        }
        self.entries.notify(|mut v| {
            v.insert(i, t);
            v
        });
        Ok(())
    }
    pub fn peek(&self) -> Option<Track> {
        self.get(0)
    }
    pub fn pop(&self) -> Option<Track> {
        if self.is_empty() {
            return None;
        }
        let first = self.get(0);
        self.entries.notify(|mut v| {
            v.remove(0);
            v
        });
        first
    }
    pub fn prepend(&self, t: Track) {
        self.entries.notify(|mut v| {
            v.insert(0, t);
            v
        });
    }
    pub fn remove_at(&self, i: usize) -> Result<()> {
        let n = self.len();
        if i >= n {
            bail!("invalid index, must be between 0 and {n}");
        }
        self.entries.notify(|mut v| {
            v.remove(i);
            v
        });
        Ok(())
    }
    pub fn set(&self, tracks: Vec<Track>) {
        self.entries.set(tracks);
    }
    /// Removes the first `n` tracks and returns them.
    pub fn skip(&self, n: usize) -> Result<Vec<Track>> {
        if self.len() < n {
            bail!("not enough tracks in queue");
        }
        let skipped = self.entries.with(|v| v[..n].to_vec());
        self.entries.notify(|mut v| {
            v.drain(..n);
            v
        });
        Ok(skipped)
    }
}

/// Queue that remembers its original order so shuffle can be undone.
#[derive(Default)]
#[derive(Debug)]
pub struct DurableQueue {
    queue: Queue,
    source: RefCell<Vec<Rc<Track>>>,
    shuffled: Cell<bool>,
    upcoming: RefCell<Vec<usize>>,
    last_taken: Cell<Option<usize>>,
    /// The source entries that started playing or were skipped, each once, in the order they last did.
    taken: RefCell<Vec<Rc<Track>>>,
}

impl DurableQueue {
    #[cfg(test)]
    pub fn append(&self, t: Track) {
        self.extend(&[t]);
    }

    /// Records that source entry `index` plays or is skipped now.
    fn took(&self, index: usize) {
        let entry = Rc::clone(&self.source.borrow()[index]);
        let mut taken = self.taken.borrow_mut();
        taken.retain(|t| !Rc::ptr_eq(t, &entry));
        taken.push(entry);
    }

    fn publish(&self) {
        let tracks = {
            let source = self.source.borrow();
            self.upcoming.borrow().iter().map(|&i| source[i].as_ref().clone()).collect()
        };
        self.queue.set(tracks);
    }

    pub fn clear(&self) {
        self.shuffled.set(false);
        self.set(Vec::new());
    }

    pub fn entries(&self) -> &State<Vec<Track>> {
        self.queue.entries()
    }

    pub fn source_len(&self) -> usize {
        self.source.borrow().len()
    }

    pub fn is_shuffled(&self) -> bool {
        self.shuffled.get()
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn peek(&self) -> Option<Track> {
        self.queue.peek()
    }

    fn take_head(&self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }
        Some(self.upcoming.borrow_mut().remove(0))
    }

    pub fn pop(&self) -> Option<Track> {
        let index = self.take_head()?;
        let track = self.source.borrow()[index].as_ref().clone();
        self.took(index);
        self.last_taken.set(Some(index));
        self.publish();
        Some(track)
    }

    /// The source tracks that [`DurableQueue::rewind`] brings back and that
    /// play nowhere else: not queued here, not in the user queue
    /// (`user_occurrences`) and not the one playing. In the source's order,
    /// which a rewind plays them in. A rewind of a shuffled queue is shuffled
    /// again: there the ones that played come last, in the order they did,
    /// so the list only grows at its end.
    pub fn played(&self, current_id: &str, user_occurrences: &[Weak<Track>]) -> Vec<Track> {
        let src = self.source.borrow();
        let current = self.last_taken.get().filter(|&i| src[i].id() == current_id);
        let upcoming: std::collections::HashSet<usize> = self.upcoming.borrow().iter().copied().collect();
        let queued: std::collections::HashSet<_> = user_occurrences.iter().map(Weak::as_ptr).collect();
        let taken: std::collections::HashMap<_, _> = self.taken.borrow().iter().enumerate().map(|(at, entry)| (Rc::as_ptr(entry), at + 1)).collect();
        let mut left: Vec<&Rc<Track>> = src
            .iter()
            .enumerate()
            .filter(|(i, entry)| Some(*i) != current && !upcoming.contains(i) && !queued.contains(&Rc::as_ptr(entry)))
            .map(|(_, entry)| entry)
            .collect();
        if self.shuffled.get() {
            left.sort_by_key(|entry| taken.get(&Rc::as_ptr(entry)).copied().unwrap_or(0));
        }
        left.into_iter().map(|entry| entry.as_ref().clone()).collect()
    }

    pub fn take_for_gapless(&self) -> Option<HistoryEntry> {
        let index = self.take_head()?;
        let entry = self.history_entry(index);
        self.publish();
        Some(entry)
    }

    fn history_entry(&self, index: usize) -> HistoryEntry {
        let source = self.source.borrow();
        HistoryEntry { track: source[index].as_ref().clone(), occurrence: Some(Rc::downgrade(&source[index])) }
    }

    pub fn move_entry(&self, from: usize, to: usize) {
        {
            let mut upcoming = self.upcoming.borrow_mut();
            if from >= upcoming.len() { return; }
            let index = upcoming.remove(from);
            let to = to.min(upcoming.len());
            upcoming.insert(to, index);
        }
        self.publish();
    }

    pub fn set_rest(&self, current: Track, rest: Vec<Track>) {
        let selected = self.last_taken.get().and_then(|i| self.source.borrow().get(i).cloned())
            .filter(|entry| entry.id() == current.id() && entry.playlist_item_id == current.playlist_item_id)
            .unwrap_or_else(|| Rc::new(current));
        *self.upcoming.borrow_mut() = (1..=rest.len()).collect();
        *self.source.borrow_mut() = std::iter::once(selected).chain(rest.into_iter().map(Rc::new)).collect();
        self.last_taken.set(Some(0));
        self.taken.borrow_mut().clear();
        self.took(0);
        self.publish();
    }

    pub fn remove_at(&self, i: usize) -> Result<()> {
        {
            let mut upcoming = self.upcoming.borrow_mut();
            if i >= upcoming.len() { bail!("track not found"); }
            let removed = upcoming.remove(i);
            let entry = self.source.borrow_mut().remove(removed);
            self.taken.borrow_mut().retain(|t| !Rc::ptr_eq(t, &entry));
            for index in upcoming.iter_mut() {
                *index -= usize::from(*index > removed);
            }
            self.last_taken.set(self.last_taken.get().filter(|&index| index != removed)
                .map(|index| index - usize::from(index > removed)));
        }
        self.publish();
        Ok(())
    }

    /// Restores source order for upcoming entries that have yet to be taken.
    pub fn restore(&self) {
        {
            let source = self.source.borrow();
            let taken: std::collections::HashSet<_> = self.taken.borrow().iter().map(Rc::as_ptr).collect();
            let mut upcoming = self.upcoming.borrow_mut();
            upcoming.retain(|&index| !taken.contains(&Rc::as_ptr(&source[index])));
            upcoming.sort_unstable();
        }
        self.shuffled.set(false);
        self.publish();
    }

    pub fn rewind(&self) {
        self.last_taken.set(None);
        self.taken.borrow_mut().clear();
        *self.upcoming.borrow_mut() = (0..self.source_len()).collect();
        self.shuffled.set(false);
        self.publish();
    }

    pub fn set(&self, tracks: Vec<Track>) {
        *self.upcoming.borrow_mut() = (0..tracks.len()).collect();
        *self.source.borrow_mut() = tracks.into_iter().map(Rc::new).collect();
        self.last_taken.set(None);
        self.taken.borrow_mut().clear();
        self.publish();
    }

    pub fn shuffle(&self, current_id: &str, user_occurrences: &[Weak<Track>]) {
        let src = self.source.borrow();
        let current = self.last_taken.get().filter(|&i| src[i].id() == current_id);
        let queued: std::collections::HashSet<_> = user_occurrences.iter().map(Weak::as_ptr).collect();
        let mut upcoming: Vec<_> = src.iter().enumerate()
            .filter_map(|(i, entry)| (Some(i) != current && !queued.contains(&Rc::as_ptr(entry))).then_some(i))
            .collect();
        drop(src);
        upcoming.shuffle(&mut rand::rng());
        *self.upcoming.borrow_mut() = upcoming;
        self.shuffled.set(true);
        self.publish();
    }

    pub fn current_occurrence(&self, track: &Track) -> Option<Weak<Track>> {
        let index = self.last_taken.get()?;
        let source = self.source.borrow();
        let entry = &source[index];
        (entry.id() == track.id()).then(|| Rc::downgrade(entry))
    }

    pub fn select_history_occurrence(&self, occurrence: Option<&Weak<Track>>) {
        let origin = occurrence.and_then(Weak::upgrade);
        let index = origin.and_then(|track| self.source.borrow().iter().position(|entry| Rc::ptr_eq(entry, &track)));
        if let Some(i) = index {
            self.took(i);
        }
        self.last_taken.set(index);
    }

    pub fn skip(&self, n: usize) -> Result<Vec<HistoryEntry>> {
        if self.queue.len() < n {
            bail!("not enough tracks in queue");
        }
        let skipped = {
            self.upcoming.borrow_mut().drain(..n).map(|i| {
                self.took(i);
                self.history_entry(i)
            }).collect()
        };
        self.publish();
        Ok(skipped)
    }

    /// Adds only track IDs that have not appeared in the source.
    pub fn extend(&self, tracks: &[Track]) {
        let added = {
            let source = self.source.borrow();
            let mut seen: std::collections::HashSet<&str> = source.iter().map(|t| t.id()).collect();
            tracks.iter().filter(|t| seen.insert(t.id())).cloned().collect()
        };
        self.extend_occurrences(added);
    }

    /// Appends every list occurrence without reordering existing upcoming tracks.
    pub fn extend_occurrences(&self, tracks: Vec<Track>) {
        if tracks.is_empty() {
            return;
        }
        let range = {
            let mut source = self.source.borrow_mut();
            let start = source.len();
            source.extend(tracks.into_iter().map(Rc::new));
            start..source.len()
        };
        {
            let mut upcoming = self.upcoming.borrow_mut();
            if self.shuffled.get() {
                let mut rng = rand::rng();
                for i in range {
                    let pos = rand::RngExt::random_range(&mut rng, 0..=upcoming.len());
                    upcoming.insert(pos, i);
                }
            } else {
                upcoming.extend(range);
            }
        }
        self.publish();
    }
}

#[derive(Clone, Debug)]
pub struct HistoryEntry {
    pub track: Track,
    pub occurrence: Option<Weak<Track>>,
}

impl From<Track> for HistoryEntry {
    fn from(track: Track) -> Self {
        Self { track, occurrence: None }
    }
}

#[derive(Default, Debug)]
pub struct UserQueue {
    rows: RefCell<Vec<HistoryEntry>>,
    display: Queue,
}

impl UserQueue {
    pub fn entries(&self) -> &State<Vec<Track>> {
        self.display.entries()
    }

    pub fn source_occurrences(&self) -> Vec<Weak<Track>> {
        self.rows.borrow().iter().filter_map(|entry| entry.occurrence.clone()).collect()
    }

    pub fn edit<R>(&self, f: impl FnOnce(&mut Vec<HistoryEntry>) -> R) -> R {
        let result = f(&mut self.rows.borrow_mut());
        let tracks = self.rows.borrow().iter().map(|entry| entry.track.clone()).collect();
        self.display.set(tracks);
        result
    }

    pub fn is_empty(&self) -> bool {
        self.rows.borrow().is_empty()
    }

    pub fn peek(&self) -> Option<Track> {
        self.display.peek()
    }

    pub fn pop(&self) -> Option<HistoryEntry> {
        if self.is_empty() {
            return None;
        }
        Some(self.edit(|rows| rows.remove(0)))
    }

    pub fn remove_at(&self, i: usize) -> Result<()> {
        if i >= self.rows.borrow().len() {
            bail!("track not found");
        }
        self.edit(|rows| rows.remove(i));
        Ok(())
    }

    pub fn skip(&self, n: usize) -> Result<Vec<HistoryEntry>> {
        if n > self.rows.borrow().len() {
            bail!("not enough tracks in queue");
        }
        Ok(self.edit(|rows| rows.drain(..n).collect()))
    }

    pub fn append(&self, track: Track) {
        self.edit(|rows| rows.push(track.into()));
    }

    pub fn prepend(&self, entry: HistoryEntry) {
        self.edit(|rows| rows.insert(0, entry));
    }

    pub fn clear(&self) {
        self.edit(Vec::clear);
    }
}

#[derive(Default, Debug)]
pub struct HistoryQueue {
    previous: RefCell<VecDeque<HistoryEntry>>,
    pending: RefCell<Option<HistoryEntry>>,
}

impl HistoryQueue {
    pub fn prepend(&self, entry: HistoryEntry) {
        if let Some(p) = self.pending.borrow_mut().replace(entry) {
            self.previous.borrow_mut().push_front(p);
        }
    }

    pub fn pop(&self) -> Option<HistoryEntry> {
        self.pending.borrow_mut().take();
        self.previous.borrow_mut().pop_front()
    }

    fn pending_matches(entry: &HistoryEntry, track: &Track) -> bool {
        entry.track.id() == track.id() && entry.track.playlist_item_id == track.playlist_item_id
    }

    /// Removes the pending entry for a track that failed to start.
    pub fn forget_failed_start(&self, track: &Track) {
        self.pending.borrow_mut().take_if(|entry| Self::pending_matches(entry, track));
    }

    /// Takes the latest history entry other than the active track.
    pub fn pop_previous(&self, current: Option<&Track>) -> Option<HistoryEntry> {
        let mut pending = self.pending.borrow_mut();
        if matches!((&*pending, current), (Some(entry), Some(track)) if Self::pending_matches(entry, track)) {
            let previous = self.previous.borrow_mut().pop_front()?;
            pending.take();
            return Some(previous);
        }
        pending.take().or_else(|| self.previous.borrow_mut().pop_front())
    }

    pub fn is_empty(&self) -> bool {
        self.previous.borrow().is_empty()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn extend_appends_in_order_and_skips_duplicates() {
        let q = super::DurableQueue::default();
        q.set(vec![t("a"), t("b"), t("c")]);
        q.skip(1).unwrap();
        q.extend(&[t("a"), t("b"), t("c"), t("d"), t("e")]);
        let ids: Vec<String> = q.entries().with(|v| v.iter().map(|x| x.id().to_string()).collect());
        assert_eq!(ids, ["b", "c", "d", "e"]);
        // restoring the order keeps the late tracks after the current one
        q.pop();
        q.restore();
        let ids: Vec<String> = q.entries().with(|v| v.iter().map(|x| x.id().to_string()).collect());
        assert_eq!(ids, ["c", "d", "e"]);
    }

    #[test]
    fn extend_while_shuffled_keeps_queue_and_adds_everything() {
        let q = super::DurableQueue::default();
        q.set((0..20).map(|i| t(&i.to_string())).collect());
        q.pop();
        q.shuffle("0", &[]);
        let before: Vec<String> = q.entries().with(|v| v.iter().map(|x| x.id().to_string()).collect());
        q.extend(&(20..40).map(|i| t(&i.to_string())).collect::<Vec<_>>());
        let after: Vec<String> = q.entries().with(|v| v.iter().map(|x| x.id().to_string()).collect());
        assert!(q.is_shuffled());
        assert_eq!(after.len(), 39);
        // the existing shuffle order is preserved (new tracks only interleave)
        let kept: Vec<&String> = after.iter().filter(|id| id.parse::<u32>().unwrap() < 20).collect();
        assert_eq!(kept, before.iter().collect::<Vec<_>>());
    }

    use super::*;

    fn t(id: &str) -> Track {
        let mut t = Track::default();
        t.info.id = id.into();
        t
    }
    fn ids(q: &Queue) -> Vec<String> {
        q.entries().get().iter().map(|t| t.id().to_string()).collect()
    }

    #[test]
    fn queue_basic_ops() {
        let q = Queue::default();
        assert!(q.pop().is_none());
        q.append(t("1"));
        q.append(t("2"));
        q.prepend(t("0"));
        q.insert(t("x"), 1).unwrap();
        assert!(q.insert(t("y"), 9).is_err());
        assert_eq!(ids(&q), ["0", "x", "1", "2"]);
        q.remove_at(1).unwrap();
        assert!(q.remove_at(5).is_err());
        assert_eq!(q.peek().unwrap().id(), "0");
        assert_eq!(q.pop().unwrap().id(), "0");
        assert!(q.contains("2"));
        let skipped = q.skip(1).unwrap();
        assert_eq!(skipped[0].id(), "1");
        q.skip(5).unwrap_err();
        assert_eq!(ids(&q), ["2"]);
        q.clear();
        assert!(q.is_empty());
    }

    #[test]
    fn durable_shuffle_and_restore() {
        let q = DurableQueue::default();
        q.set((0..20).map(|i| t(&i.to_string())).collect());
        q.append(t("5")); // duplicate ignored
        assert_eq!(q.len(), 20);
        q.skip(3).unwrap();
        q.pop(); // "3" plays
        q.shuffle("3", &[]);
        assert!(q.is_shuffled());
        assert_eq!(q.len(), 19);
        assert!(!ids(&q.queue).contains(&"3".to_string()));
        q.restore();
        assert!(!q.is_shuffled());
        assert_eq!(ids(&q.queue), (4..20).map(|i| i.to_string()).collect::<Vec<_>>());
        q.rewind();
        assert_eq!(q.len(), 20, "rewind brings back played tracks");
        q.remove_at(0).unwrap();
        q.rewind();
        assert_eq!(q.len(), 19, "removed tracks are gone from the source");
    }

    #[test]
    fn played_is_what_a_rewind_brings_back() -> Result<()> {
        let played = |q: &DurableQueue, current: &str| q.played(current, &[]).iter().map(|t| t.id().to_string()).collect::<Vec<_>>();
        let q = DurableQueue::default();
        q.set((0..5).map(|i| t(&i.to_string())).collect());
        assert!(played(&q, "").is_empty());
        // Started at the third track: the first two come back too.
        q.skip(2)?;
        q.pop();
        assert_eq!(played(&q, "2"), ["0", "1"]);
        // Played out of the source's order, which a rewind restores.
        q.move_entry(0, 1);
        q.pop();
        q.pop();
        assert_eq!(played(&q, "3"), ["0", "1", "2", "4"]);
        q.set((0..5).map(|i| t(&i.to_string())).collect());
        q.skip(2)?;
        q.pop();
        // Shuffled: the others follow in the order they play.
        q.shuffle("2", &[]);
        assert!(played(&q, "2").is_empty());
        let next = q.pop().unwrap();
        let after = q.pop().unwrap();
        assert_eq!(played(&q, after.id()), ["2", next.id()]);
        // A track moved to the user queue counts once it plays from there.
        let moved = q.take_for_gapless().unwrap();
        assert_eq!(q.played(after.id(), &[moved.occurrence.clone().unwrap()]).len(), 2);
        q.select_history_occurrence(moved.occurrence.as_ref());
        assert_eq!(played(&q, moved.track.id()), ["2", next.id(), after.id()]);
        let last = q.pop().unwrap();
        assert_eq!(played(&q, last.id()), ["2", next.id(), after.id(), moved.track.id()]);
        q.rewind();
        assert!(played(&q, "").is_empty());
        Ok(())
    }

    #[test]
    fn restore_keeps_unplayed_tracks_before_the_furthest_shuffled_pick() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c", "d", "e", "f", "g"].map(t).to_vec());
        q.shuffle("", &[]);
        for picked in ["f", "b", "d"] {
            let index = q.entries().with(|tracks| tracks.iter().position(|track| track.id() == picked)).unwrap();
            q.move_entry(index, 0);
            assert_eq!(q.pop().unwrap().id(), picked);
        }
        q.restore();
        assert_eq!(ids(&q.queue), ["a", "c", "e", "g"]);
    }

    #[test]
    fn restore_keeps_the_gapless_reservation_out_of_the_base_queue() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c"].map(t).to_vec());
        q.shuffle("", &[]);
        let index = q.entries().with(|tracks| tracks.iter().position(|track| track.id() == "c")).unwrap();
        q.move_entry(index, 0);
        assert_eq!(q.take_for_gapless().unwrap().track.id(), "c");
        q.restore();
        assert_eq!(ids(&q.queue), ["a", "b"]);
    }

    #[test]
    fn restore_never_requeues_played_tracks() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c", "d"].map(t).to_vec());
        q.skip(2).unwrap(); // "a" and "b" played
        q.shuffle("", &[]);
        while q.peek().is_some_and(|x| x.id() != "a") {
            q.pop();
        }
        q.pop(); // "a" again, listed before the restore point
        q.restore();
        assert!(ids(&q.queue).iter().all(|id| id != "a" && id != "b"));
    }

    #[test]
    fn restore_counts_a_repeated_track_once_per_play() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c", "a", "d"].map(t).to_vec());
        q.skip(3).unwrap();
        q.pop(); // the second "a"
        q.restore();
        assert_eq!(ids(&q.queue), ["d"]);
        q.rewind();
        q.pop(); // the first "a"
        q.restore();
        assert_eq!(ids(&q.queue), ["b", "c", "a", "d"]);
    }

    #[test]
    fn shuffle_keeps_the_other_occurrence_of_the_current_track() {
        let q = DurableQueue::default();
        q.set(["a", "b", "a", "c"].map(t).to_vec());
        q.pop();
        q.shuffle("a", &[]);
        let mut upcoming = ids(&q.queue);
        upcoming.sort();
        assert_eq!(upcoming, ["a", "b", "c"]);
    }

    #[test]
    fn removing_one_entry_keeps_other_occurrences_after_restore() -> Result<()> {
        let q = DurableQueue::default();
        q.set(["a", "b", "x", "c", "x", "d"].map(t).to_vec());
        q.pop();
        q.remove_at(1)?;
        q.shuffle("a", &[]);
        q.restore();
        assert_eq!(ids(&q.queue), ["b", "c", "x", "d"]);
        Ok(())
    }

    #[test]
    fn removing_a_reordered_entry_removes_it_from_the_source() -> Result<()> {
        let q = DurableQueue::default();
        q.set(["a", "b", "c", "d"].map(t).to_vec());
        q.pop();
        q.move_entry(1, 0);
        q.remove_at(0)?;
        q.restore();
        assert_eq!(ids(&q.queue), ["b", "d"]);
        Ok(())
    }

    #[test]
    fn shuffled_removal_keeps_the_other_duplicate_in_source_order() -> Result<()> {
        let q = DurableQueue::default();
        let mut later = t("a");
        later.info.title = "later occurrence".into();
        q.set(vec![t("a"), t("b"), later, t("c")]);
        q.shuffle("", &[]);
        let later = q.entries().with(|v| v.iter().position(|t| t.info.title == "later occurrence")).unwrap();
        q.remove_at(later)?;
        q.restore();
        assert_eq!(ids(&q.queue), ["a", "b", "c"]);
        Ok(())
    }

    #[test]
    fn shuffle_and_restore_use_the_played_occurrence() -> Result<()> {
        let q = DurableQueue::default();
        let mut first = t("a");
        first.playlist_item_id = Some("first".into());
        let mut second = t("a");
        second.playlist_item_id = Some("second".into());
        q.set(vec![first, t("b"), second, t("c")]);
        q.move_entry(2, 0);
        assert_eq!(q.pop().unwrap().playlist_item_id.as_deref(), Some("second"));
        q.shuffle("a", &[]);
        let first = q.entries().with(|v| v.iter().position(|t| t.playlist_item_id.as_deref() == Some("first"))).unwrap();
        assert!(q.entries().with(|v| v.iter().all(|t| t.playlist_item_id.as_deref() != Some("second"))));
        q.remove_at(first)?;
        q.restore();
        assert_eq!(ids(&q.queue), ["b", "c"]);
        Ok(())
    }

    #[test]
    fn shuffling_after_a_user_track_preserves_its_base_occurrence() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c"].map(t).to_vec());
        q.pop();
        q.select_history_occurrence(None);
        q.shuffle("c", &[]);
        let mut upcoming = ids(&q.queue);
        upcoming.sort();
        assert_eq!(upcoming, ["a", "b", "c"]);
    }

    #[test]
    fn shuffle_keeps_both_current_and_reserved_gapless_entries_out() {
        let q = DurableQueue::default();
        q.set(["a", "b", "b", "c"].map(t).to_vec());
        q.pop();
        let reserved = q.take_for_gapless().unwrap();
        assert_eq!(reserved.track.id(), "b");
        q.shuffle("a", reserved.occurrence.as_slice());
        let mut upcoming = ids(&q.queue);
        upcoming.sort();
        assert_eq!(upcoming, ["b", "c"]);
        q.select_history_occurrence(reserved.occurrence.as_ref());
        q.shuffle("b", &[]);
        let mut upcoming = ids(&q.queue);
        upcoming.sort();
        assert_eq!(upcoming, ["a", "b", "c"]);
    }

    #[test]
    fn removing_a_played_track_keeps_the_restore_point() {
        let q = DurableQueue::default();
        q.set(["a", "b", "a", "c"].map(t).to_vec());
        q.pop();
        // removing the upcoming "a" drops the played one from the source too
        q.remove_at(1).unwrap();
        q.restore();
        assert_eq!(ids(&q.queue), ["b", "c"]);
    }

    #[test]
    fn history_holds_back_current_track() {
        let h = HistoryQueue::default();
        let entry = |id| HistoryEntry { track: t(id), occurrence: None };
        h.prepend(entry("a"));
        assert!(h.is_empty());
        h.prepend(entry("b"));
        assert_eq!(h.pop().unwrap().track.id(), "a");
        h.prepend(entry("c"));
        assert!(h.is_empty(), "pop clears the pending entry");
    }

    #[test]
    fn previous_returns_the_last_track_when_playback_stopped() {
        let h = HistoryQueue::default();
        let a = t("a");
        let b = t("b");
        h.prepend(a.clone().into());
        assert!(h.pop_previous(Some(&a)).is_none());
        h.prepend(b.into());
        assert_eq!(h.pop_previous(None).unwrap().track.id(), "b");
        assert_eq!(h.pop_previous(None).unwrap().track.id(), "a");
    }

    #[test]
    fn history_recovers_exact_occurrence_after_a_source_removal() -> Result<()> {
        let q = DurableQueue::default();
        let mut first = t("a");
        first.info.title = "previous occurrence".into();
        q.set(vec![t("removed"), first, t("b"), t("a")]);
        q.move_entry(0, 2);
        let previous = q.pop().unwrap();
        let occurrence = q.current_occurrence(&previous);
        q.pop();
        q.remove_at(0)?;
        q.select_history_occurrence(occurrence.as_ref());
        q.shuffle("a", &[]);
        assert!(q.entries().with(|v| v.iter().all(|t| t.info.title != "previous occurrence")));
        assert!(q.entries().with(|v| v.iter().any(|t| t.id() == "a")));
        Ok(())
    }

    #[test]
    fn attaching_a_handoff_list_preserves_the_current_history_occurrence() {
        let q = DurableQueue::default();
        q.set(vec![t("a")]);
        let current = q.pop().unwrap();
        let occurrence = q.current_occurrence(&current);
        let mut duplicate = t("a");
        duplicate.info.title = "upcoming duplicate".into();
        q.set_rest(current, vec![t("b"), duplicate]);

        assert_eq!(q.pop().unwrap().id(), "b");
        q.select_history_occurrence(occurrence.as_ref());
        q.shuffle("a", &[]);
        q.entries().with(|tracks| {
            assert_eq!(tracks.len(), 2);
            assert!(tracks.iter().any(|track| track.info.title == "upcoming duplicate"));
        });
    }
}

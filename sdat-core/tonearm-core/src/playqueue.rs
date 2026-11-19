//! Port of `internal/player/playqueue`.
use crate::error::{bail, CoreResult as Result};
use rand::seq::SliceRandom;
use std::cell::{Cell, RefCell};

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
    source: RefCell<Vec<Track>>,
    shuffled: Cell<bool>,
    /// Source index just past the last track taken from the queue, where
    /// [`DurableQueue::restore`] continues.
    played: Cell<usize>,
    upcoming: RefCell<Vec<usize>>,
    last_taken: Cell<Option<usize>>,
    gapless_taken: Cell<Option<usize>>,
}

impl DurableQueue {
    #[cfg(test)]
    pub fn append(&self, t: Track) {
        self.extend(&[t]);
    }

    fn publish(&self) {
        let tracks = {
            let source = self.source.borrow();
            self.upcoming.borrow().iter().map(|&i| source[i].clone()).collect()
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

    fn take_head(&self, taken: &Cell<Option<usize>>) -> Option<Track> {
        let track = self.peek()?;
        let index = self.upcoming.borrow_mut().remove(0);
        self.played.set(self.played.get().max(index + 1));
        taken.set(Some(index));
        self.publish();
        Some(track)
    }

    pub fn pop(&self) -> Option<Track> {
        self.take_head(&self.last_taken)
    }

    pub fn take_for_gapless(&self) -> Option<Track> {
        self.take_head(&self.gapless_taken)
    }

    pub fn select_user_track(&self, is_gapless: bool) {
        let index = self.gapless_taken.take();
        self.last_taken.set(if is_gapless { index } else { None });
    }

    pub fn forget_gapless(&self) {
        self.gapless_taken.set(None);
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

    pub fn remove_at(&self, i: usize) -> Result<()> {
        {
            let mut upcoming = self.upcoming.borrow_mut();
            if i >= upcoming.len() { bail!("track not found"); }
            let removed = upcoming.remove(i);
            self.source.borrow_mut().remove(removed);
            for index in upcoming.iter_mut() {
                *index -= usize::from(*index > removed);
            }
            for taken in [&self.last_taken, &self.gapless_taken] {
                taken.set(taken.get().filter(|&index| index != removed)
                    .map(|index| index - usize::from(index > removed)));
            }
            if removed < self.played.get() {
                self.played.set(self.played.get() - 1);
            }
        }
        self.publish();
        Ok(())
    }

    /// Restores the original order, continuing after the furthest source entry taken.
    pub fn restore(&self) {
        let len = self.source_len();
        *self.upcoming.borrow_mut() = (self.played.get()..len).collect();
        self.shuffled.set(false);
        self.publish();
    }

    pub fn rewind(&self) {
        self.played.set(0);
        self.last_taken.set(None);
        self.gapless_taken.set(None);
        self.restore();
    }

    pub fn set(&self, tracks: Vec<Track>) {
        *self.upcoming.borrow_mut() = (0..tracks.len()).collect();
        *self.source.borrow_mut() = tracks;
        self.played.set(0);
        self.last_taken.set(None);
        self.gapless_taken.set(None);
        self.publish();
    }

    pub fn shuffle(&self, current_id: &str) {
        let src = self.source.borrow();
        let current = self.last_taken.get().filter(|&i| src[i].id() == current_id);
        let reserved = self.gapless_taken.get();
        let mut upcoming: Vec<_> = (0..src.len())
            .filter(|&i| Some(i) != current && Some(i) != reserved).collect();
        drop(src);
        upcoming.shuffle(&mut rand::rng());
        *self.upcoming.borrow_mut() = upcoming;
        self.shuffled.set(true);
        self.publish();
    }

    pub fn skip(&self, n: usize) -> Result<Vec<Track>> {
        if self.queue.len() < n {
            bail!("not enough tracks in queue");
        }
        let skipped = self.queue.entries().with(|v| v[..n].to_vec());
        if let Some(max) = self.upcoming.borrow_mut().drain(..n).max() {
            self.played.set(self.played.get().max(max + 1));
        }
        self.publish();
        Ok(skipped)
    }

    /// Adds late-arriving tracks without touching what is already queued.
    pub fn extend(&self, tracks: &[Track]) {
        let mut source = self.source.borrow_mut();
        let mut seen: std::collections::HashSet<&str> = source.iter().map(Track::id).collect();
        let added: Vec<_> = tracks.iter().filter(|t| seen.insert(t.id())).cloned().collect();
        if added.is_empty() {
            return;
        }
        let start = source.len();
        source.extend(added);
        let end = source.len();
        drop(source);
        {
            let mut upcoming = self.upcoming.borrow_mut();
            if self.shuffled.get() {
                let mut rng = rand::rng();
                for i in start..end {
                    let pos = rand::RngExt::random_range(&mut rng, 0..=upcoming.len());
                    upcoming.insert(pos, i);
                }
            } else {
                upcoming.extend(start..end);
            }
        }
        self.publish();
    }
}

/// History queue: the most recent entry is held back as "pending" (it is the
/// track currently playing) until the next one arrives.
#[derive(Default)]
#[derive(Debug)]
pub struct HistoryQueue {
    pub queue: Queue,
    pending: RefCell<Option<Track>>,
}

impl HistoryQueue {
    pub fn prepend(&self, t: Track) {
        if let Some(p) = self.pending.borrow_mut().replace(t) {
            self.queue.prepend(p);
        }
    }
    pub fn pop(&self) -> Option<Track> {
        *self.pending.borrow_mut() = None;
        self.queue.pop()
    }
    pub fn peek(&self) -> Option<Track> {
        self.queue.peek()
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
        q.shuffle("0");
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
        q.shuffle("3");
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
    fn restore_continues_after_the_last_played_list_track() {
        let q = DurableQueue::default();
        q.set((0..10).map(|i| t(&i.to_string())).collect());
        q.shuffle("");
        let played: Vec<Track> = (0..3).filter_map(|_| q.pop()).collect();
        // a "play next" track from elsewhere is playing now
        q.restore();
        let last = played.iter().map(|x| x.id().parse::<usize>().unwrap()).max().unwrap();
        assert_eq!(ids(&q.queue), (last + 1..10).map(|i| i.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn restore_never_requeues_played_tracks() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c", "d"].map(t).to_vec());
        q.skip(2).unwrap(); // "a" and "b" played
        q.shuffle("");
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
        q.shuffle("a");
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
        q.shuffle("a");
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
    fn shuffle_and_restore_use_the_played_occurrence() -> Result<()> {
        let q = DurableQueue::default();
        let mut first = t("a");
        first.playlist_item_id = Some("first".into());
        let mut second = t("a");
        second.playlist_item_id = Some("second".into());
        q.set(vec![first, t("b"), second, t("c")]);
        q.move_entry(2, 0);
        assert_eq!(q.pop().unwrap().playlist_item_id.as_deref(), Some("second"));
        q.shuffle("a");
        let first = q.entries().with(|v| v.iter().position(|t| t.playlist_item_id.as_deref() == Some("first"))).unwrap();
        assert!(q.entries().with(|v| v.iter().all(|t| t.playlist_item_id.as_deref() != Some("second"))));
        q.remove_at(first)?;
        q.restore();
        assert_eq!(ids(&q.queue), ["c"]);
        Ok(())
    }

    #[test]
    fn shuffling_after_a_user_track_preserves_its_base_occurrence() {
        let q = DurableQueue::default();
        q.set(["a", "b", "c"].map(t).to_vec());
        q.pop();
        q.select_user_track(false);
        q.shuffle("c");
        let mut upcoming = ids(&q.queue);
        upcoming.sort();
        assert_eq!(upcoming, ["a", "b", "c"]);
    }

    #[test]
    fn shuffle_keeps_both_current_and_reserved_gapless_entries_out() {
        let q = DurableQueue::default();
        q.set(["a", "b", "b", "c"].map(t).to_vec());
        q.pop();
        assert_eq!(q.take_for_gapless().unwrap().id(), "b");
        q.shuffle("a");
        let mut upcoming = ids(&q.queue);
        upcoming.sort();
        assert_eq!(upcoming, ["b", "c"]);
        q.select_user_track(true);
        q.shuffle("b");
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
        h.prepend(t("a"));
        assert!(h.peek().is_none());
        h.prepend(t("b"));
        assert_eq!(h.peek().unwrap().id(), "a");
        assert_eq!(h.pop().unwrap().id(), "a");
        h.prepend(t("c"));
        assert!(h.peek().is_none(), "pop clears the pending entry");
    }
}

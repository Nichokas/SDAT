//! Port of `internal/player/playqueue`.
use anyhow::{bail, Result};
use rand::seq::SliceRandom;
use std::cell::{Cell, RefCell};

use crate::model::Track;
use crate::signals::State;

/// Ephemeral FIFO queue whose entries are observable.
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
pub struct DurableQueue {
    queue: Queue,
    source: RefCell<Vec<Track>>,
    shuffled: Cell<bool>,
}

impl DurableQueue {
    #[cfg(test)]
    pub fn append(&self, t: Track) {
        if self.contains(t.id()) {
            return;
        }
        self.source.borrow_mut().push(t.clone());
        self.queue.append(t);
    }
    pub fn clear(&self) {
        self.source.borrow_mut().clear();
        self.queue.clear();
        self.shuffled.set(false);
    }
    #[cfg(test)]
    pub fn contains(&self, id: &str) -> bool {
        self.source.borrow().iter().any(|t| t.id() == id)
    }
    pub fn entries(&self) -> &State<Vec<Track>> {
        self.queue.entries()
    }
    /// Number of distinct list tracks known (loaded) so far.
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
    pub fn pop(&self) -> Option<Track> {
        self.queue.pop()
    }
    pub fn remove_at(&self, i: usize) -> Result<()> {
        let Some(t) = self.queue.get(i) else { bail!("track not found") };
        self.source.borrow_mut().retain(|s| s.id() != t.id());
        self.queue.remove_at(i)
    }
    /// Restores the original order, continuing after `current_id`.
    pub fn restore(&self, current_id: &str) {
        let src = self.source.borrow().clone();
        let offset = src.iter().position(|t| t.id() == current_id).map_or(0, |i| i + 1);
        self.queue.set(src[offset..].to_vec());
        self.shuffled.set(false);
    }
    pub fn set(&self, tracks: Vec<Track>) {
        self.source.borrow_mut().clone_from(&tracks);
        self.queue.set(tracks);
    }
    /// Shuffles every source track except `current_id`.
    pub fn shuffle(&self, current_id: &str) {
        let mut src: Vec<Track> = self.source.borrow().iter().filter(|t| t.id() != current_id).cloned().collect();
        src.shuffle(&mut rand::rng());
        self.queue.set(src);
        self.shuffled.set(true);
    }
    pub fn skip(&self, n: usize) -> Result<Vec<Track>> {
        self.queue.skip(n)
    }
    /// Adds late-arriving tracks (rest of a paginated list) without touching
    /// what is already queued: appended in order, or scattered at random
    /// positions among the upcoming tracks when shuffled. Duplicates are skipped.
    pub fn extend(&self, tracks: &[Track]) {
        let new: Vec<Track> = {
            let src = self.source.borrow();
            let mut seen: std::collections::HashSet<&str> = src.iter().map(Track::id).collect();
            tracks.iter().filter(|t| seen.insert(t.id())).cloned().collect()
        };
        if new.is_empty() {
            return;
        }
        self.source.borrow_mut().extend(new.iter().cloned());
        let shuffled = self.shuffled.get();
        self.queue.entries.notify(|mut v| {
            if shuffled {
                let mut rng = rand::rng();
                for t in new {
                    let i = rand::Rng::random_range(&mut rng, 0..=v.len());
                    v.insert(i, t);
                }
            } else {
                v.extend(new);
            }
            v
        });
    }
}

/// History queue: the most recent entry is held back as "pending" (it is the
/// track currently playing) until the next one arrives.
#[derive(Default)]
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
        q.extend(&[t("c"), t("d"), t("e")]);
        let ids: Vec<String> = q.entries().with(|v| v.iter().map(|x| x.id().to_string()).collect());
        assert_eq!(ids, ["b", "c", "d", "e"]);
        // restoring the order keeps the late tracks after the current one
        q.restore("b");
        let ids: Vec<String> = q.entries().with(|v| v.iter().map(|x| x.id().to_string()).collect());
        assert_eq!(ids, ["c", "d", "e"]);
    }

    #[test]
    fn extend_while_shuffled_keeps_queue_and_adds_everything() {
        let q = super::DurableQueue::default();
        q.set((0..20).map(|i| t(&i.to_string())).collect());
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
        q.shuffle("3");
        assert!(q.is_shuffled());
        assert_eq!(q.len(), 19);
        assert!(!ids(&q.queue).contains(&"3".to_string()));
        q.restore("3");
        assert!(!q.is_shuffled());
        assert_eq!(ids(&q.queue), (4..20).map(|i| i.to_string()).collect::<Vec<_>>());
        q.restore("");
        assert_eq!(q.len(), 20);
        q.pop();
        q.restore("");
        assert_eq!(q.len(), 20, "restore brings back popped tracks");
        q.remove_at(0).unwrap();
        q.restore("");
        assert_eq!(q.len(), 19, "removed tracks are gone from the source");
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

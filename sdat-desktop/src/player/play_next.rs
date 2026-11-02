//! "Play next" bookkeeping for the user queue.
//!
//! The user queue always plays before the base queue, so "play next" means
//! inserting at the front of the user queue. To keep repeated "play next"
//! calls in order, we remember how many entries at the front were added this
//! way (`pinned`) and insert after them; "add to queue" still appends.

use std::collections::BTreeMap;

/// Inserts `items` after the first `pinned` entries of `queue` (and never
/// before index `floor`, e.g. an entry a gapless handoff already prepared),
/// keeping their order. Returns the new pinned count.
pub fn insert<T>(queue: &mut Vec<T>, pinned: usize, floor: usize, items: Vec<T>) -> usize {
    let at = pinned.max(floor).min(queue.len());
    let n = items.len();
    queue.splice(at..at, items);
    at + n
}

/// Pinned count after the entry at `index` was removed.
pub fn after_remove(pinned: usize, index: usize) -> usize {
    if index < pinned { pinned - 1 } else { pinned }
}

/// Pinned count after the first `n` entries were removed.
pub fn after_skip(pinned: usize, n: usize) -> usize {
    pinned.saturating_sub(n)
}

/// Pinned count after an entry was dragged from `src` to `dst` (`dst` is the
/// index after removing it). Dragging out of the pinned prefix shrinks it,
/// dropping into it (or back at its end, if it came from it) grows it.
pub fn after_move(pinned: usize, src: usize, dst: usize) -> usize {
    let from_pinned = src < pinned;
    let p = pinned - usize::from(from_pinned);
    if dst < p || (from_pinned && dst == p) { p + 1 } else { p }
}

/// Applies asynchronously fetched "play next" batches in the order the user
/// requested them, even when a later fetch finishes first.
#[derive(Debug)]
pub struct Sequencer<T> {
    next_ticket: u64,
    next_apply: u64,
    ready: BTreeMap<u64, Vec<T>>,
}

impl<T> Default for Sequencer<T> {
    fn default() -> Self {
        Sequencer { next_ticket: 0, next_apply: 0, ready: BTreeMap::new() }
    }
}

impl<T> Sequencer<T> {
    /// Reserves a position in the order, at the moment the user acts.
    pub fn ticket(&mut self) -> u64 {
        self.next_ticket += 1;
        self.next_ticket - 1
    }
    /// Records the batch for `ticket` (empty on failure) and returns every
    /// batch that can now be applied, in request order.
    pub fn complete(&mut self, ticket: u64, items: Vec<T>) -> Vec<Vec<T>> {
        self.ready.insert(ticket, items);
        let mut out = vec![];
        while let Some(items) = self.ready.remove(&self.next_apply) {
            out.push(items);
            self.next_apply += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_at_front_of_empty_queue() {
        let mut q: Vec<u8> = vec![];
        assert_eq!(insert(&mut q, 0, 0, vec![1]), 1);
        assert_eq!(q, [1]);
    }

    #[test]
    fn repeated_play_next_queues_up_in_order_before_appended_tracks() {
        let mut q = vec![10, 11];
        let p = insert(&mut q, 0, 0, vec![1]);
        let p = insert(&mut q, p, 0, vec![2, 3]);
        assert_eq!(p, 3);
        assert_eq!(q, [1, 2, 3, 10, 11]);
    }

    #[test]
    fn stale_pinned_count_is_clamped() {
        let mut q = vec![10];
        assert_eq!(insert(&mut q, 5, 0, vec![1]), 2);
        assert_eq!(q, [10, 1]);
    }

    #[test]
    fn removal_and_skip_shrink_pinned_region() {
        assert_eq!(after_remove(2, 0), 1);
        assert_eq!(after_remove(2, 2), 2);
        assert_eq!(after_skip(2, 1), 1);
        assert_eq!(after_skip(2, 5), 0);
    }

    #[test]
    fn never_inserts_ahead_of_prepared_gapless_entry() {
        let mut q = vec![10, 11];
        let p = insert(&mut q, 0, 1, vec![1]);
        assert_eq!(q, [10, 1, 11]);
        // StreamStart pops the prepared entry.
        assert_eq!(after_remove(p, 0), 1);
    }

    #[test]
    fn dragging_updates_pinned_prefix() {
        // [P1, P2, A, B]: drag B (3) to between P1 and P2 (1) joins the prefix.
        assert_eq!(after_move(2, 3, 1), 3);
        // Drag P1 (0) to the end (index 3 after removal) leaves the prefix.
        assert_eq!(after_move(2, 0, 3), 1);
        // Reorder inside the prefix keeps it.
        assert_eq!(after_move(2, 0, 1), 2);
        // Reorder among appended tracks keeps it.
        assert_eq!(after_move(2, 3, 2), 2);
    }

    #[test]
    fn sequencer_applies_in_request_order() {
        let mut s = Sequencer::default();
        let (a, b, c) = (s.ticket(), s.ticket(), s.ticket());
        assert!(s.complete(b, vec!["playlist"]).is_empty());
        assert_eq!(s.complete(a, vec!["album"]), vec![vec!["album"], vec!["playlist"]]);
        assert_eq!(s.complete(c, vec![]), vec![Vec::<&str>::new()]);
    }
}

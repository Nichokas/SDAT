//! "Play next" bookkeeping for the user queue.
//!
//! The user queue always plays before the base queue, so "play next" means
//! inserting at the front of the user queue. To keep repeated "play next"
//! calls in order, we remember how many entries at the front were added this
//! way (`pinned`) and insert after them; "add to queue" still appends.

/// Inserts `items` after the first `pinned` entries of `queue`, keeping their
/// order. Returns the new pinned count.
pub fn insert<T>(queue: &mut Vec<T>, pinned: usize, items: Vec<T>) -> usize {
    let at = pinned.min(queue.len());
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inserts_at_front_of_empty_queue() {
        let mut q: Vec<u8> = vec![];
        assert_eq!(insert(&mut q, 0, vec![1]), 1);
        assert_eq!(q, [1]);
    }

    #[test]
    fn repeated_play_next_queues_up_in_order_before_appended_tracks() {
        let mut q = vec![10, 11];
        let p = insert(&mut q, 0, vec![1]);
        let p = insert(&mut q, p, vec![2, 3]);
        assert_eq!(p, 3);
        assert_eq!(q, [1, 2, 3, 10, 11]);
    }

    #[test]
    fn stale_pinned_count_is_clamped() {
        let mut q = vec![10];
        assert_eq!(insert(&mut q, 5, vec![1]), 2);
        assert_eq!(q, [10, 1]);
    }

    #[test]
    fn removal_and_skip_shrink_pinned_region() {
        assert_eq!(after_remove(2, 0), 1);
        assert_eq!(after_remove(2, 2), 2);
        assert_eq!(after_skip(2, 1), 1);
        assert_eq!(after_skip(2, 5), 0);
    }
}

//! Picks the track that plays next from the user and base queues, for a skip
//! and for gapless playback, without mutating them until a pick is committed.

use std::cell::Cell;
use std::hash::{BuildHasher, RandomState};

use super::playqueue::HistoryEntry;
use super::statemachine::{track_changed, GaplessEntry, GaplessSource};
use super::{
    base_queue, current_track_id, list_pending, play_next, repeat_mode_changed, shuffle_state_changed, track_matches,
    update_pinned, user_queue, RepeatMode,
};
use crate::model::Track;
use crate::signals::State;

pub(super) fn random_index(len: usize) -> usize {
    thread_local! {
        static COUNTER: Cell<u64> = const { Cell::new(0) };
    }
    let n = len as u64;
    if n <= 1 {
        return 0;
    }
    let threshold = n.wrapping_neg() % n;
    loop {
        let step = COUNTER.with(|c| {
            let next = c.get().wrapping_add(1);
            c.set(next);
            next
        });
        let sample = RandomState::new().hash_one(step);
        if sample >= threshold {
            return (sample % n) as usize;
        }
    }
}

pub(super) fn find_streamable_index(tracks: &[Track], target: Option<&Track>) -> Option<usize> {
    tracks
        .iter()
        .position(|t| t.info.streamable && target.is_none_or(|want| track_matches(t, want)))
}

pub(super) fn leading_unstreamable_count(tracks: &[Track]) -> usize {
    tracks.iter().take_while(|t| !t.info.streamable).count()
}

pub(super) fn first_streamable(entries: &State<Vec<Track>>) -> Option<Track> {
    entries.with(|rows| find_streamable_index(rows, None).map(|i| rows[i].clone()))
}

pub(super) fn can_repeat_base_queue() -> bool {
    !list_pending() && repeat_mode_changed().get() == RepeatMode::Queue
}

pub(super) fn rewind_base_queue() {
    log::debug!("queue repeat mode is enabled, replaying base queue");
    base_queue().rewind();
    if shuffle_state_changed().get() {
        base_queue().shuffle(&current_track_id().unwrap_or_default(), &user_queue().source_occurrences());
    }
}

pub(super) fn drain_unstreamable_user_rows(rows: &mut Vec<HistoryEntry>, limit: usize) -> usize {
    let leading = rows[..limit].iter().take_while(|r| !r.track.info.streamable).count();
    if leading > 0 {
        rows.drain(..leading);
    }
    leading
}

pub(super) fn drain_unstreamable_user_prefix() {
    if user_queue().entries().with(|rows| leading_unstreamable_count(rows)) == 0 {
        return;
    }
    user_queue().edit(|rows| {
        let len = rows.len();
        let leading = drain_unstreamable_user_rows(rows, len);
        update_pinned(|p| play_next::after_skip(p, leading));
    });
}

pub(super) fn drain_unstreamable_base_prefix() {
    let leading = base_queue().entries().with(|rows| leading_unstreamable_count(rows));
    if leading > 0 {
        let _ = base_queue().skip(leading);
    }
}

pub(super) fn drain_unstreamable_queues() {
    drain_unstreamable_user_prefix();
    drain_unstreamable_base_prefix();
}

pub(super) fn peek_rewound_base_queue() -> Option<Track> {
    // Sentinel id never matches a real track, so all source tracks are visible
    // without mutating `base_queue` during a peek.
    let mut streamable = base_queue()
        .played("\0", &user_queue().source_occurrences())
        .into_iter()
        .filter(|t| t.info.streamable);
    if !shuffle_state_changed().get() {
        return streamable.next();
    }
    let mut candidates: Vec<_> = streamable.collect();
    if candidates.is_empty() {
        return None;
    }
    if let Some(current_id) = current_track_id()
        && candidates.iter().any(|t| t.id() != current_id)
    {
        candidates.retain(|t| t.id() != current_id);
    }
    Some(candidates.swap_remove(random_index(candidates.len())))
}

pub(super) fn consume_user_queue_entry(target: &Track) -> Option<Track> {
    let idx = user_queue().entries().with(|rows| find_streamable_index(rows, Some(target)));
    let removed = idx.and_then(|idx| {
        user_queue().edit(|rows| {
            if idx >= rows.len() {
                return None;
            }
            let row = rows.remove(idx);
            let leading = drain_unstreamable_user_rows(rows, idx);
            update_pinned(|p| play_next::after_skip(play_next::after_remove(p, idx), leading));
            Some(row)
        })
    });
    base_queue().select_history_occurrence(removed.as_ref().and_then(|r| r.occurrence.as_ref()));
    removed.map(|r| r.track)
}

pub(super) fn consume_base_queue_entry(target: &Track, allow_rewind: bool) -> Option<Track> {
    let mut idx = base_queue().entries().with(|rows| find_streamable_index(rows, Some(target)));
    if idx.is_none() && allow_rewind {
        rewind_base_queue();
        idx = base_queue().entries().with(|rows| find_streamable_index(rows, Some(target)));
    }
    let Some(i) = idx else {
        base_queue().select_history_occurrence(None);
        return None;
    };
    let leading_unstreamable = i > 0
        && base_queue().entries().with(|rows| leading_unstreamable_count(rows) >= i);
    if leading_unstreamable {
        let _ = base_queue().skip(i);
    } else if i > 0 {
        base_queue().move_entry(i, 0);
    }
    base_queue().pop()
}

pub(super) fn peek_next_queue_entry() -> Option<(Track, GaplessSource)> {
    if let Some(t) = first_streamable(user_queue().entries()) {
        return Some((t, GaplessSource::UserQueue));
    }
    if let Some(t) = first_streamable(base_queue().entries()) {
        return Some((t, GaplessSource::BaseQueue { rewound: false }));
    }
    if !can_repeat_base_queue() {
        return None;
    }
    peek_rewound_base_queue().map(|next| (next, GaplessSource::BaseQueue { rewound: true }))
}

pub(super) fn commit_queue_entry(source: GaplessSource, track: &Track) -> Option<Track> {
    match source {
        GaplessSource::RepeatTrack => Some(track.clone()),
        GaplessSource::UserQueue => consume_user_queue_entry(track),
        // The rewind was decided when the entry was armed; a later repeat change must not undo it.
        GaplessSource::BaseQueue { rewound } => {
            drain_unstreamable_user_prefix();
            consume_base_queue_entry(track, rewound)
        }
    }
}

pub(super) fn pop_next_streamable_from_queue() -> Option<Track> {
    let Some((track, source)) = peek_next_queue_entry() else {
        drain_unstreamable_queues();
        return None;
    };
    commit_queue_entry(source, &track)
}

/// Returns the next streamable track and whether it is a track repeat.
pub(super) fn next_track(peek: bool) -> (Option<Track>, bool) {
    let Some(entry) = peek_gapless() else {
        if !peek && !crate::live::is_guest() && repeat_mode_changed().get() != RepeatMode::Track {
            drain_unstreamable_queues();
        }
        return (None, false);
    };
    if !peek {
        commit_gapless(&entry);
    }
    (Some(entry.track), entry.did_repeat)
}

pub(super) fn peek_gapless() -> Option<GaplessEntry> {
    if crate::live::is_guest() {
        return None;
    }
    if repeat_mode_changed().get() == RepeatMode::Track {
        return track_changed().with(|t| {
            t.as_ref().map(|e| GaplessEntry {
                track: e.track.clone(),
                did_repeat: true,
                source: GaplessSource::RepeatTrack,
            })
        });
    }
    peek_next_queue_entry().map(|(track, source)| GaplessEntry { track, did_repeat: false, source })
}

pub(super) fn commit_gapless(entry: &GaplessEntry) {
    let _ = commit_queue_entry(entry.source, &entry.track);
}

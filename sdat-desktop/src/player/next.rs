//! Picks the track that plays next from the user and base queues, for a skip
//! and for gapless playback, without mutating them until a pick is committed.

use std::cell::Cell;

use super::playqueue::HistoryEntry;
use super::statemachine::{track_changed, GaplessEntry, GaplessSource};
use super::{
    base_queue, current_track_id, list_pending, play_next, repeat_mode_changed, shuffle_state_changed, track_matches,
    update_pinned, user_queue, RepeatMode,
};
use crate::model::Track;
use crate::signals::State;

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
    Some(candidates.swap_remove(rand::random_range(..candidates.len())))
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

/// Takes the next streamable track and indicates whether it is a track repeat.
pub(super) fn next_track(manual: bool) -> (Option<Track>, bool) {
    if crate::live::is_guest() {
        return (None, false);
    }
    if !manual && let Some(current) = peek_track_repeat() {
        return (Some(current), true);
    }
    (pop_next_streamable_from_queue(), false)
}

pub(super) fn peek_gapless() -> Option<GaplessEntry> {
    if crate::live::is_guest() {
        return None;
    }
    if let Some(track) = peek_track_repeat() {
        return Some(GaplessEntry { track, did_repeat: true, source: GaplessSource::RepeatTrack });
    }
    peek_next_queue_entry().map(|(track, source)| GaplessEntry { track, did_repeat: false, source })
}

fn peek_track_repeat() -> Option<Track> {
    if repeat_mode_changed().get() != RepeatMode::Track {
        return None;
    }
    track_changed().with(|t| t.as_ref().map(|e| e.track.clone()))
}

pub(super) fn commit_gapless(entry: &GaplessEntry) {
    let _ = commit_queue_entry(entry.source, &entry.track);
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use super::*;

    fn track(id: &str, item: Option<&str>) -> Track {
        let mut track = Track::default();
        track.info.id = id.into();
        track.info.streamable = true;
        track.playlist_item_id = item.map(str::to_owned);
        track
    }

    #[test]
    fn gapless_commit_ignores_user_queue_addition_and_repeat_toggle() {
        clear_queues();
        set_repeat_mode(RepeatMode::None);
        base_queue().set(vec![track("A", None), track("B", None), track("C", None)]);
        let a = base_queue().pop().unwrap();
        track_changed().set(Some(statemachine::TrackChangedEvent { track: a, skip_history: false }));

        // Arm Track B from the base queue.
        let armed = peek_gapless().unwrap();
        assert_eq!(armed.track.id(), "B");
        assert!(!armed.did_repeat);
        assert_eq!(armed.source, GaplessSource::BaseQueue { rewound: false });

        // User adds Track X to an empty user_queue and toggles RepeatMode::Track on during PlayingGapless.
        user_queue().append(track("X", None));
        set_repeat_mode(RepeatMode::Track);

        // StreamStart commits the exact armed Track B from base_queue without popping X.
        commit_gapless(&armed);
        assert_eq!(user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["X"]);
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["C"]);

        // Now arm a RepeatMode::Track repeat, then turn repeat off before commit.
        let repeated = peek_gapless().unwrap();
        assert_eq!(repeated.track.id(), "A");
        assert!(repeated.did_repeat);
        assert_eq!(repeated.source, GaplessSource::RepeatTrack);
        set_repeat_mode(RepeatMode::None);
        commit_gapless(&repeated);
        assert_eq!(user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["X"]);
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["C"]);

        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn gapless_commit_drops_leading_unstreamable_user_entries_and_updates_pinned() {
        clear_queues();
        set_repeat_mode(RepeatMode::None);

        let mut unstreamable = track("A", None);
        unstreamable.info.streamable = false;

        // Both the skipped unstreamable head and the chosen streamable entry are pinned.
        insert_tracklist_next(vec![unstreamable.clone(), track("B", None), track("C", None)]);
        user_queue().append(track("D", None));
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 3);

        let armed = peek_gapless().unwrap();
        assert_eq!(armed.track.id(), "B");
        assert_eq!(armed.source, GaplessSource::UserQueue);

        commit_gapless(&armed);
        assert_eq!(
            user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()),
            vec!["C", "D"],
        );
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 1);

        // Only the leading unstreamable entry is pinned; the chosen streamable entry is appended.
        clear_queues();
        insert_tracklist_next(vec![unstreamable.clone()]);
        user_queue().append(track("B", None));
        user_queue().append(track("C", None));
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 1);

        let armed = peek_gapless().unwrap();
        assert_eq!(armed.track.id(), "B");
        assert_eq!(armed.source, GaplessSource::UserQueue);

        commit_gapless(&armed);
        assert_eq!(
            user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()),
            vec!["C"],
        );
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 0);

        // A streamable Play Next entry inserted before commit is preserved while the leading
        // unstreamable entry and the armed track are both removed.
        clear_queues();
        insert_tracklist_next(vec![unstreamable.clone()]);
        user_queue().append(track("B", None));
        let armed = peek_gapless().unwrap();
        assert_eq!(armed.track.id(), "B");
        insert_tracklist_next(vec![track("X", None)]);
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 2);

        commit_gapless(&armed);
        assert_eq!(
            user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()),
            vec!["X"],
        );
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 1);

        // When user_queue only has unstreamable entries, committing a base_queue gapless entry
        // also drops the skipped unstreamable prefix and updates the pinned count.
        clear_queues();
        insert_tracklist_next(vec![unstreamable]);
        base_queue().set(vec![track("B", None), track("C", None)]);
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 1);

        let armed = peek_gapless().unwrap();
        assert_eq!(armed.track.id(), "B");
        assert_eq!(armed.source, GaplessSource::BaseQueue { rewound: false });

        commit_gapless(&armed);
        assert!(user_queue().is_empty());
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 0);
        assert_eq!(
            base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()),
            vec!["C"],
        );

        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn play_next_lands_after_an_armed_user_entry_behind_unstreamable_rows() {
        clear_queues();
        set_repeat_mode(RepeatMode::None);
        let mut unstreamable = track("U", None);
        unstreamable.info.streamable = false;
        user_queue().append(unstreamable);
        user_queue().append(track("B", None));
        let armed = peek_gapless().unwrap();
        assert_eq!((armed.track.id(), armed.source), ("B", GaplessSource::UserQueue));

        insert_tracklist_next_after(vec![track("B", None), track("C", None)], Some(armed.clone()));
        commit_gapless(&armed);
        // The armed B played; the requested extra B and then C come next.
        assert_eq!(
            user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()),
            vec!["B", "C"],
        );

        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn play_next_lands_after_an_armed_base_entry_behind_unstreamable_rows() {
        clear_queues();
        set_repeat_mode(RepeatMode::None);
        let mut unstreamable = track("U", None);
        unstreamable.info.streamable = false;
        base_queue().set(vec![unstreamable, track("B", None), track("C", None)]);
        let armed = peek_gapless().unwrap();
        assert_eq!((armed.track.id(), armed.source), ("B", GaplessSource::BaseQueue { rewound: false }));

        assert!(insert_tracklist_next_after(vec![track("X", None)], Some(armed.clone())));
        // The armed B moves ahead of the Play Next entries, so the queue view shows the playback order.
        assert_eq!(
            user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()),
            vec!["B", "X"],
        );
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["C"]);

        // The track change commits it from the user queue, where the armed entry now points.
        commit_gapless(&GaplessEntry { source: GaplessSource::UserQueue, ..armed });
        assert_eq!(user_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["X"]);
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["C"]);

        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn next_track_pop_skips_unstreamable_prefixes_and_updates_pinned() {
        clear_queues();
        set_repeat_mode(RepeatMode::None);

        let mut unstreamable_u = track("U1", None);
        unstreamable_u.info.streamable = false;
        let mut unstreamable_b = track("U2", None);
        unstreamable_b.info.streamable = false;

        insert_tracklist_next(vec![unstreamable_u.clone(), track("B", None)]);
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 2);
        assert_eq!(next_track(false).0.as_ref().map(Track::id), Some("B"));
        assert!(user_queue().is_empty());
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 0);

        insert_tracklist_next(vec![unstreamable_u]);
        base_queue().set(vec![unstreamable_b, track("C", None), track("D", None)]);
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 1);
        assert_eq!(next_track(false).0.as_ref().map(Track::id), Some("C"));
        assert!(user_queue().is_empty());
        assert_eq!(PLAY_NEXT_PINNED.with(Cell::get), 0);
        assert_eq!(base_queue().entries().with(|v| v.iter().map(|t| t.id().to_string()).collect::<Vec<_>>()), vec!["D"]);

        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn repeat_one_only_repeats_at_the_natural_end() {
        clear_queues();
        set_repeat_mode(RepeatMode::Track);
        let current = track("A", None);
        base_queue().set(vec![track("B", None)]);
        track_changed().set(Some(statemachine::TrackChangedEvent { track: current, skip_history: false }));

        let (automatic, skip_history) = next_track(false);
        assert_eq!(automatic.as_ref().map(Track::id), Some("A"));
        assert!(skip_history);
        let (manual, skip_history) = next_track(true);
        assert_eq!(manual.as_ref().map(Track::id), Some("B"));
        assert!(!skip_history);

        set_repeat_mode(RepeatMode::None);
        track_changed().set(None);
    }

    #[test]
    fn peek_next_track_does_not_mutate_base_queue_on_repeat_or_shuffle() {
        clear_queues();
        set_repeat_mode(RepeatMode::Queue);
        base_queue().set(vec![track("A", None), track("B", None)]);
        assert_eq!(base_queue().pop().unwrap().id(), "A");
        let b = base_queue().pop().unwrap();
        track_changed().set(Some(statemachine::TrackChangedEvent { track: b, skip_history: false }));
        assert!(base_queue().is_empty());

        // Peeking under RepeatMode::Queue must inspect without rewinding.
        let peeked = peek_gapless().unwrap();
        assert!(!peeked.did_repeat);
        assert_eq!(peeked.track.id(), "A");
        assert!(base_queue().is_empty(), "peek mutated base_queue by rewinding");

        // Same when shuffle is enabled: peeking must neither rewind nor reshuffle.
        shuffle_state_changed().set(true);
        assert!(peek_gapless().is_some());
        assert!(base_queue().is_empty(), "peek mutated base_queue while shuffled");

        // Popping actually rewinds, shuffles, and pops one entry (leaving the other).
        let (popped, did_repeat) = next_track(false);
        assert!(!did_repeat);
        assert!(popped.is_some());
        assert_eq!(base_queue().entries().with(Vec::len), 1);

        shuffle_state_changed().set(false);
        set_repeat_mode(RepeatMode::None);
        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn shuffle_repeat_queue_wraparound_peek_is_randomized_and_non_mutating() {
        use std::collections::HashSet;

        clear_queues();
        set_repeat_mode(RepeatMode::Queue);
        let mut unavailable = track("U", Some("u1"));
        unavailable.info.streamable = false;
        let tracks = vec![
            track("A", Some("a1")),
            track("B", Some("b1")),
            track("B", Some("b2")),
            track("C", Some("c1")),
            unavailable,
            track("D", Some("d1")),
        ];
        let all_streamable: HashSet<(String, Option<String>)> = tracks
            .iter()
            .filter(|t| t.info.streamable)
            .map(|t| (t.id().to_string(), t.playlist_item_id.clone()))
            .collect();
        base_queue().set(tracks);
        set_shuffle(true, "");

        let mut last_streamable = None;
        while let Some(t) = base_queue().pop() {
            if t.info.streamable {
                last_streamable = Some(t);
            }
        }
        let current = last_streamable.unwrap();
        let current_id = current.id().to_string();
        track_changed().set(Some(statemachine::TrackChangedEvent { track: current, skip_history: false }));
        assert!(base_queue().is_empty());

        let played_before: Vec<(String, Option<String>)> = base_queue()
            .played("\0", &[])
            .into_iter()
            .map(|t| (t.id().to_string(), t.playlist_item_id))
            .collect();
        let first_streamable_key = base_queue()
            .played("\0", &[])
            .into_iter()
            .find(|t| t.info.streamable)
            .map(|t| (t.id().to_string(), t.playlist_item_id));
        let expected: HashSet<(String, Option<String>)> = all_streamable
            .iter()
            .filter(|(id, _)| id != &current_id)
            .cloned()
            .collect();
        assert!(expected.len() >= 3);

        let mut seen = HashSet::new();
        let mut chosen_non_first = None;
        for _ in 0..256 {
            let entry = peek_gapless().unwrap();
            assert!(!entry.did_repeat);
            assert_eq!(entry.source, GaplessSource::BaseQueue { rewound: true });
            assert!(entry.track.info.streamable);
            assert_ne!(entry.track.id(), current_id, "wraparound peek must avoid immediately repeating the current track");
            assert!(base_queue().is_empty(), "wraparound peek must not mutate base_queue");

            let key = (entry.track.id().to_string(), entry.track.playlist_item_id.clone());
            if first_streamable_key.as_ref() != Some(&key) && chosen_non_first.is_none() {
                chosen_non_first = Some(entry);
            }
            seen.insert(key);
        }

        let played_after: Vec<(String, Option<String>)> = base_queue()
            .played("\0", &[])
            .into_iter()
            .map(|t| (t.id().to_string(), t.playlist_item_id))
            .collect();
        assert_eq!(played_after, played_before, "repeated peeks must leave base_queue history untouched");
        assert_eq!(seen, expected, "wraparound peek should randomize across all eligible non-current candidates");

        let chosen = chosen_non_first.unwrap();
        let chosen_key = (chosen.track.id().to_string(), chosen.track.playlist_item_id.clone());
        commit_gapless(&chosen);
        assert!(base_queue().is_shuffled());
        assert!(base_queue().current_occurrence(&chosen.track).is_some());
        let remaining_streamable: HashSet<(String, Option<String>)> = base_queue().entries().with(|rows| {
            assert!(!rows.iter().any(|t| track_matches(t, &chosen.track)));
            rows.iter()
                .filter(|t| t.info.streamable)
                .map(|t| (t.id().to_string(), t.playlist_item_id.clone()))
                .collect()
        });
        let mut expected_remaining = all_streamable;
        assert!(expected_remaining.remove(&chosen_key));
        assert_eq!(remaining_streamable, expected_remaining);

        // When all streamable tracks share the current track id, wraparound must still succeed.
        clear_queues();
        let mut only_unstreamable = track("U2", Some("u2"));
        only_unstreamable.info.streamable = false;
        base_queue().set(vec![track("S", Some("s1")), track("S", Some("s2")), only_unstreamable]);
        set_shuffle(true, "");
        let mut only = None;
        while let Some(t) = base_queue().pop() {
            if t.info.streamable {
                only = Some(t);
            }
        }
        track_changed().set(Some(statemachine::TrackChangedEvent { track: only.unwrap(), skip_history: false }));
        let wrapped = peek_gapless().unwrap();
        assert_eq!(wrapped.track.id(), "S");
        assert_eq!(wrapped.source, GaplessSource::BaseQueue { rewound: true });
        assert!(base_queue().is_empty());
        commit_gapless(&wrapped);
        assert!(base_queue().current_occurrence(&wrapped.track).is_some());
        assert!(base_queue().entries().with(|rows| !rows.iter().any(|t| track_matches(t, &wrapped.track))));

        shuffle_state_changed().set(false);
        set_repeat_mode(RepeatMode::None);
        clear_queues();
        track_changed().set(None);
    }

    #[test]
    fn armed_rewound_entry_commits_after_repeat_is_turned_off() {
        clear_queues();
        set_repeat_mode(RepeatMode::Queue);
        base_queue().set(vec![track("A", Some("a")), track("B", Some("b"))]);
        while base_queue().pop().is_some() {}
        let armed = peek_gapless().unwrap();
        assert_eq!(armed.source, GaplessSource::BaseQueue { rewound: true });
        set_repeat_mode(RepeatMode::None);
        commit_gapless(&armed);
        assert!(base_queue().current_occurrence(&armed.track).is_some());
        assert_eq!(base_queue().entries().with(Vec::len), 1);
        clear_queues();
    }
}

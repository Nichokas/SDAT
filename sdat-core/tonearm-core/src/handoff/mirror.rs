//! What TIDAL holds, and the writes that bring it to the local playback.
//! Pure: time comes in as Unix ms, every effect goes out as a value.

use super::wire::{Entry, Head, State, Status};
use super::{Device, Elsewhere, HandoffEvent, LocalPlayback, Repeat};

/// Tracks written after the current one.
pub(super) const MAX_FUTURE: usize = 100;
/// A position this far from the extrapolated one is a seek.
const SEEK_MS: i64 = 3_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Write {
    /// Make this device the active player.
    Claim,
    /// Create a queue and select it (the account has none).
    NewQueue,
    /// Replace the queue; the first track becomes current.
    Queue(Vec<String>),
    /// Add tracks after the written ones, to their batch. `capped`: the local list goes on after them.
    Append { batch: String, tracks: Vec<String>, capped: bool },
    Repeat(Repeat),
    /// Select written entry `index` at this position.
    Current { index: usize, position_ms: i64 },
    Status { playing: bool },
}

/// Outcome of a fetch.
#[derive(Debug, Clone)]
pub(super) enum Out {
    Event(Box<HandoffEvent>),
    /// Another device made this one active: fetch the queue and play it.
    Arrive,
}

/// The queue as this device last wrote it.
#[derive(Debug, Clone)]
struct Written {
    /// Local track ids, current first.
    wrote: Vec<String>,
    /// TIDAL's entries for them, as far as read back.
    entries: Vec<Entry>,
    /// TIDAL's batch of a queue this device wrote: `wrote` is all of it, in
    /// its order. `None` for a queue taken over as another device wrote it.
    batch: Option<String>,
    /// The last write stopped at the cap: the local list goes on after `wrote`.
    capped: bool,
    cur: usize,
    revision: String,
    repeat: Repeat,
    position_ms: i64,
    at_ms: i64,
    playing: bool,
}

impl Written {
    /// Index of the written entry the local list starts at, the current one
    /// first, and how many of `tracks` need no write: the rest is appended.
    /// `repeat`: the mode the queue is written with.
    fn find(&self, tracks: &[String], repeat: Repeat) -> Option<(usize, usize)> {
        // More follows a capped local list than a capped write holds: what is
        // written may lag behind it, and is topped up once half of it is played.
        let lags = self.capped && tracks.len() == MAX_FUTURE + 1;
        // Only a batch of this device's own is known in full.
        let own = self.batch.is_some();
        let written = |k: usize| {
            let (before, rest) = self.wrote.split_at(k);
            if lags && 2 * rest.len() >= tracks.len() && tracks.starts_with(rest) {
                return Some(tracks.len());
            }
            match repeat {
                // TIDAL repeats the whole batch: `[1, 2, 3]` written, with `2` current, is `[2, 3, 1]`. The list
                // has to be exactly that rotation: an append would land inside the repeated list.
                Repeat::All => {
                    let rotated = tracks.len() == self.wrote.len() && tracks.starts_with(rest) && tracks.ends_with(before);
                    // A queue taken over was read as its whole batch only if it repeated then: under any
                    // other mode the batch may hold tracks from before its current one.
                    (rotated && (own || (k == 0 && self.repeat == Repeat::All))).then_some(tracks.len())
                }
                _ if rest == tracks => Some(tracks.len()),
                Repeat::Off if own => tracks.starts_with(rest).then_some(rest.len()),
                // Under repeat one TIDAL shows nothing after the current item: the entry of an appended track could not be read.
                _ => None,
            }
        };
        (self.cur..self.wrote.len()).chain(0..self.cur.min(self.wrote.len())).find_map(|k| Some((k, written(k)?)))
    }

    /// TIDAL's entry for written index `index`, if it was read back and still matches.
    fn known(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index).filter(|e| self.wrote.get(index) == Some(&e.track_id))
    }
}

#[derive(Debug)]
pub(super) struct Mirror {
    me: String,
    state: State,
    head: Option<Head>,
    /// Last active player that was not `null`: a lapsed attachment reads as
    /// `null` until it is refreshed, and is not a change of player.
    last_active: Option<String>,
    fetched: bool,
    written: Option<Written>,
    local_playing: bool,
    /// The next report writes the position (this device just became active).
    need_position: bool,
    sent_devices: Option<Vec<Device>>,
    /// `None` until the first fetch was told.
    sent_elsewhere: Option<Sent>,
}

/// The last [`HandoffEvent::Elsewhere`] sent, as what a banner shows of it.
#[derive(Debug, PartialEq)]
struct Sent(Option<ElsewhereKey>);

/// What a banner shows of an [`Elsewhere`].
type ElsewhereKey = (Device, String, i64, bool, i64);

fn key(e: &Elsewhere) -> ElsewhereKey {
    (e.device.clone(), e.track.id().to_string(), e.position_ms, e.playing, e.updated_at_ms)
}

/// Where a player that was at `position_ms` at `at_ms` is at `now_ms`.
pub(super) fn position_now(position_ms: i64, at_ms: i64, now_ms: i64, playing: bool, duration_ms: i64) -> i64 {
    // Saturating: a timestamp from long ago must not overflow.
    let p = if playing { position_ms.saturating_add(now_ms.saturating_sub(at_ms).max(0)) } else { position_ms };
    p.min(duration_ms.saturating_sub(1_000)).max(0)
}

impl Mirror {
    pub(super) fn new(me: String) -> Self {
        Self {
            me,
            state: State::default(),
            head: None,
            last_active: None,
            fetched: false,
            written: None,
            local_playing: false,
            need_position: false,
            sent_devices: None,
            sent_elsewhere: None,
        }
    }

    pub(super) fn me(&self) -> &str {
        &self.me
    }

    pub(super) fn has_fetched(&self) -> bool {
        self.fetched
    }

    pub(super) fn is_active(&self) -> bool {
        self.last_active.as_deref() == Some(self.me.as_str())
    }

    pub(super) fn state(&self) -> &State {
        &self.state
    }

    pub(super) fn queue_id(&self) -> Option<&str> {
        self.state.queue_id.as_deref()
    }

    pub(super) fn device(&self, id: &str) -> Option<Device> {
        self.state.players.iter().find(|p| p.id == id).cloned()
    }

    /// Writes that make TIDAL match `local`. Only the active player writes;
    /// starting to play claims that role. The worker applies them before it
    /// plans again, so nothing is planned twice.
    pub(super) fn plan(&mut self, local: &LocalPlayback, now_ms: i64) -> Vec<Write> {
        let started = local.playing && !self.local_playing;
        self.local_playing = local.playing;
        let mut out = Vec::new();
        if self.state.active.as_deref() != Some(self.me.as_str()) {
            // Also after this device's own attachment lapsed, or when who is active
            // is unknown: claiming is harmless then. Without a start, nothing is
            // written over what another device may own.
            if !(started && local.current.is_some()) {
                return out;
            }
            out.push(Write::Claim);
        }
        let Some(cur) = &local.current else {
            if self.state.status == Status::Playing {
                out.push(Write::Status { playing: false });
            }
            return out;
        };
        if self.state.queue_id.is_none() {
            out.push(Write::NewQueue);
        }
        let repeat = local.queue_repeat();
        // Repeat all plays the tracks before the current one again after the last. They are written
        // behind it, as one batch that TIDAL repeats: the device that takes the queue over reads the whole list.
        let before = if repeat == Repeat::All { local.past.as_slice() } else { &[] };
        let tracks: Vec<String> = std::iter::once(cur).chain(&local.future).chain(before).take(MAX_FUTURE + 1).cloned().collect();
        // An entry never read back can't be selected: the queue is written again.
        let found = self.written.as_ref().and_then(|w| w.find(&tracks, repeat).filter(|&(i, _)| w.known(i).is_some()).map(|f| (w, f)));
        if let Some((w, (index, written))) = found {
            // First, so that what follows an append is read as it is queued, not as another mode shows it.
            if w.repeat != repeat {
                out.push(Write::Repeat(repeat));
            }
            if let Some(batch) = w.batch.clone().filter(|_| written < tracks.len()) {
                out.push(Write::Append { batch, tracks: tracks[written..].to_vec(), capped: tracks.len() == MAX_FUTURE + 1 });
            }
            let expected = if w.playing { w.position_ms.saturating_add(now_ms.saturating_sub(w.at_ms)) } else { w.position_ms };
            if index != w.cur || self.need_position || w.playing != local.playing || local.position_ms.saturating_sub(expected).saturating_abs() > SEEK_MS {
                out.push(Write::Current { index, position_ms: local.position_ms });
            }
        } else {
            out.push(Write::Queue(tracks));
            out.push(Write::Current { index: 0, position_ms: local.position_ms });
        }
        let status = if local.playing { Status::Playing } else { Status::Paused };
        if self.state.status != status {
            out.push(Write::Status { playing: local.playing });
        }
        out
    }

    /// TIDAL's entry for written index `index`, if it still matches.
    pub(super) fn entry(&self, index: usize) -> Option<Entry> {
        self.written.as_ref()?.known(index).cloned()
    }

    pub(super) fn claimed(&mut self, active: Option<String>) {
        if active.is_some() {
            self.last_active.clone_from(&active);
        }
        self.state.active = active;
        self.need_position = true;
    }

    pub(super) fn force_position(&mut self) {
        self.need_position = true;
    }

    pub(super) fn released_to(&mut self, id: &str) {
        self.state.active = Some(id.to_string());
        self.last_active = Some(id.to_string());
        self.written = None;
    }

    pub(super) fn queue_created(&mut self, id: String) {
        self.state.queue_id = Some(id);
        self.written = None;
    }

    pub(super) fn queue_written(&mut self, wrote: Vec<String>, entries: Vec<Entry>, batch: String, revision: String, repeat: Repeat) {
        let capped = wrote.len() == MAX_FUTURE + 1;
        self.written = Some(Written { wrote, entries, batch: Some(batch), capped, cur: 0, revision, repeat, position_ms: 0, at_ms: 0, playing: false });
    }

    /// `future`: the entries after TIDAL's current one, read after the append.
    pub(super) fn appended(&mut self, tracks: Vec<String>, capped: bool, future: Vec<Entry>, revision: String) {
        if let Some(w) = &mut self.written {
            w.wrote.extend(tracks);
            w.capped = capped;
            w.revision = revision;
            // They follow the current entry, the last one still valid.
            w.entries.truncate(w.cur + 1);
            if w.entries.len() == w.cur + 1 {
                w.entries.extend(future);
                w.entries.truncate(w.wrote.len());
            }
        }
    }

    pub(super) fn repeat_written(&mut self, repeat: Repeat, revision: String) {
        if let Some(w) = &mut self.written {
            w.repeat = repeat;
            w.revision = revision;
        }
    }

    pub(super) fn current_written(&mut self, index: usize, position_ms: i64, playing: bool, now_ms: i64, revision: String) {
        if let Some(w) = &mut self.written {
            *w = Written { cur: index, position_ms, at_ms: now_ms, playing, revision, ..w.clone() };
        }
        self.need_position = false;
    }

    pub(super) fn status_written(&mut self, playing: bool) {
        self.state.status = if playing { Status::Playing } else { Status::Paused };
    }

    /// A write failed: the queue on TIDAL is unknown now, and a failed
    /// claim is tried again on the next report.
    pub(super) fn write_failed(&mut self) {
        self.written = None;
        // Also after this device's own attachment lapsed, when TIDAL names no player.
        if self.state.active.as_deref() != Some(self.me.as_str()) {
            self.local_playing = false;
        }
    }

    /// TIDAL's active player could not be read after a change: the next
    /// playing report claims before it writes anything.
    pub(super) fn ownership_unknown(&mut self) {
        self.state.active = None;
        self.local_playing = false;
        self.written = None;
    }

    /// Local playback still runs from before (a send that went elsewhere): a
    /// playing report of it is no new start, so it claims nothing.
    pub(super) fn still_playing(&mut self) {
        self.local_playing = true;
    }

    /// The next playing report counts as a start, and claims the active player.
    pub(super) fn reset_start(&mut self) {
        self.local_playing = false;
    }

    /// Takes over the queue as read for a resume, its current entry first,
    /// so the first report after it only writes the position and status.
    pub(super) fn adopt(&mut self, current: Entry, head: &Head, position_ms: i64, now_ms: i64) {
        let written = Written {
            wrote: vec![current.track_id.clone()],
            entries: vec![current],
            batch: None,
            capped: false,
            cur: 0,
            revision: head.revision.clone(),
            repeat: head.repeat,
            position_ms,
            at_ms: now_ms,
            playing: false,
        };
        self.written = Some(written);
        // The queue read for this is the baseline now: the next read must not take it for a new push.
        self.head = Some(head.clone());
        self.need_position = true;
    }

    /// The queue as written still is an adopted current entry alone, at `revision`.
    pub(super) fn holds_adoption(&self, revision: &str) -> bool {
        self.written.as_ref().is_some_and(|w| w.revision == revision && w.cur == 0 && w.wrote.len() == 1 && w.entries.len() == 1)
    }

    /// The entries after an adopted current one, read after it at `revision`.
    /// Dropped once anything else is known of the queue; `false` then.
    pub(super) fn adopted_future(&mut self, revision: &str, future: Vec<Entry>) -> bool {
        if !self.holds_adoption(revision) {
            return false;
        }
        let Some(w) = self.written.as_mut() else { return false };
        w.wrote.extend(future.iter().map(|e| e.track_id.clone()));
        w.entries.extend(future);
        w.capped = w.wrote.len() == MAX_FUTURE + 1;
        true
    }

    /// From a state read alone, before its queue: the active player left this
    /// device for another, which the app is told at once. [`Mirror::fetched`]
    /// does not tell it again.
    pub(super) fn moved(&mut self, state: &State) -> Option<HandoffEvent> {
        let a = state.active.as_deref().filter(|a| *a != self.me && self.fetched && self.is_active())?;
        let d = state.players.iter().find(|p| p.id == a).cloned().unwrap_or_else(|| Device { id: a.to_string(), name: String::new() });
        self.last_active = Some(a.to_string());
        // Nothing is written as the active player any more, even if the queue read fails.
        self.state.active = Some(a.to_string());
        self.written = None;
        Some(HandoffEvent::MovedTo(d))
    }

    /// A fresh read of TIDAL's state. `updated_by` is the device that last
    /// wrote the position, with its name.
    pub(super) fn fetched(&mut self, state: State, head: Option<Head>, updated_by: Option<Device>) -> Vec<Out> {
        let mut out = Vec::new();
        if state.queue_id != self.state.queue_id || self.written.as_ref().is_some_and(|w| head.as_ref().is_none_or(|h| h.revision != w.revision)) {
            self.written = None;
        }
        let was_me = self.is_active();
        if state.active.is_some() {
            self.last_active.clone_from(&state.active);
        }
        // Another device wrote the queue while this one stayed active: a push
        // that this device never saw itself lose the active player for.
        let pushed = match (&self.head, &head) {
            (Some(old), Some(new)) => new.revision != old.revision && new.updated_by.as_deref().is_some_and(|u| u != self.me),
            // A queue that appeared, written by another device.
            (None, Some(new)) => self.fetched && new.updated_by.as_deref().is_some_and(|u| u != self.me),
            (_, None) => false,
        };
        self.state = state;
        self.head = head;
        let first = !std::mem::replace(&mut self.fetched, true);
        if !first {
            match self.last_active.as_deref() {
                Some(a) if was_me && a != self.me => {
                    self.written = None;
                    let d = self.device(a).unwrap_or_else(|| Device { id: a.to_string(), name: String::new() });
                    out.push(Out::Event(Box::new(HandoffEvent::MovedTo(d))));
                }
                Some(a) if a == self.me && (!was_me || pushed) => out.push(Out::Arrive),
                _ => {}
            }
        }
        let devices: Vec<Device> = self.state.players.iter().filter(|p| p.id != self.me).cloned().collect();
        if self.sent_devices.as_ref() != Some(&devices) {
            self.sent_devices = Some(devices.clone());
            out.push(Out::Event(Box::new(HandoffEvent::Devices(devices))));
        }
        let elsewhere = self.elsewhere(updated_by);
        let k = Sent(elsewhere.as_ref().map(key));
        if self.sent_elsewhere.as_ref() != Some(&k) {
            self.sent_elsewhere = Some(k);
            out.push(Out::Event(Box::new(HandoffEvent::Elsewhere(elsewhere))));
        }
        out
    }

    fn elsewhere(&self, updated_by: Option<Device>) -> Option<Elsewhere> {
        let h = self.head.as_ref()?;
        let track = h.track.clone()?;
        let (device, playing) = match self.state.active.as_deref() {
            Some(a) if a == self.me => return None,
            Some(a) => {
                let d = self.device(a).or_else(|| updated_by.filter(|u| u.id == a)).unwrap_or_else(|| Device { id: a.to_string(), name: String::new() });
                (d, self.state.status == Status::Playing)
            }
            None => (updated_by.filter(|u| u.id != self.me)?, false),
        };
        Some(Elsewhere { device, track, position_ms: h.position_ms.unwrap_or(0), playing, updated_at_ms: h.position_at_ms.unwrap_or(0) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Track, TrackInfo};

    const ME: &str = "me";

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    fn local(current: &str, future: &[&str], position_ms: i64, playing: bool) -> LocalPlayback {
        LocalPlayback { current: Some(current.into()), future: ids(future), position_ms, playing, repeat: Repeat::Off, ..LocalPlayback::default() }
    }

    fn dev(id: &str, name: &str) -> Device {
        Device { id: id.into(), name: name.into() }
    }

    fn state(active: Option<&str>, status: Status) -> State {
        State {
            status,
            active: active.map(Into::into),
            players: vec![dev(ME, "Desktop"), dev("phone", "iPhone")],
            queue_id: Some("q".into()),
            stream: None,
        }
    }

    fn track(id: &str) -> Track {
        Track { info: TrackInfo { id: id.into(), duration: std::time::Duration::from_secs(200), ..TrackInfo::default() }, ..Track::default() }
    }

    fn head(rev: &str, current: &str, by: &str) -> Head {
        Head {
            revision: rev.into(),
            current: Some(Entry { track_id: current.into(), item_id: format!("i{current}") }),
            position_ms: Some(10_000),
            position_at_ms: Some(1_000),
            updated_by: Some(by.into()),
            track: Some(track(current)),
            ..Head::default()
        }
    }

    /// A mirror that is the active player and wrote `tracks`.
    fn active_with(tracks: &[&str]) -> Mirror {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some(ME), Status::Playing), None, None);
        let entries = tracks.iter().map(|t| Entry { track_id: (*t).into(), item_id: format!("i{t}") }).collect();
        m.queue_written(ids(tracks), entries, "b".into(), "r".into(), Repeat::Off);
        m.current_written(0, 0, true, 0, "r".into());
        m.local_playing = true;
        m
    }

    fn events(out: &[Out]) -> Vec<&HandoffEvent> {
        out.iter().filter_map(|o| if let Out::Event(e) = o { Some(e.as_ref()) } else { None }).collect()
    }

    #[test]
    fn idle_devices_do_not_write() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Playing), None, None);
        assert!(m.plan(&local("1", &["2"], 0, false), 0).is_empty());
    }

    #[test]
    fn starting_to_play_claims_and_writes_everything() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Paused), None, None);
        assert_eq!(
            m.plan(&local("1", &["2", "3"], 5_000, true), 0),
            vec![Write::Claim, Write::Queue(ids(&["1", "2", "3"])), Write::Current { index: 0, position_ms: 5_000 }, Write::Status { playing: true }]
        );
    }

    #[test]
    fn still_playing_after_a_move_does_not_claim_back() {
        let mut m = active_with(&["1", "2"]);
        m.fetched(state(Some("phone"), Status::Playing), None, None);
        // The player has not paused yet: no claim until it starts again.
        assert!(m.plan(&local("1", &["2"], 1_000, true), 1_000).is_empty());
        assert!(m.plan(&local("1", &["2"], 1_000, false), 1_000).is_empty());
        assert_eq!(m.plan(&local("1", &["2"], 1_000, true), 2_000).first(), Some(&Write::Claim));
    }

    #[test]
    fn a_track_change_moves_current_only() {
        let mut m = active_with(&["1", "2", "3"]);
        assert_eq!(m.plan(&local("2", &["3"], 0, true), 10_000), vec![Write::Current { index: 1, position_ms: 0 }]);
        m.current_written(1, 0, true, 10_000, "r".into());
        // Previous goes back to a written entry.
        assert_eq!(m.plan(&local("1", &["2", "3"], 0, true), 11_000), vec![Write::Current { index: 0, position_ms: 0 }]);
    }

    #[test]
    fn steady_playback_writes_nothing() {
        let mut m = active_with(&["1", "2"]);
        assert!(m.plan(&local("1", &["2"], 60_000, true), 60_500).is_empty());
    }

    #[test]
    fn seeks_and_pauses_write_the_position() {
        let mut m = active_with(&["1", "2"]);
        assert_eq!(m.plan(&local("1", &["2"], 90_000, true), 10_000), vec![Write::Current { index: 0, position_ms: 90_000 }]);
        m.current_written(0, 90_000, true, 10_000, "r".into());
        assert_eq!(
            m.plan(&local("1", &["2"], 95_000, false), 15_000),
            vec![Write::Current { index: 0, position_ms: 95_000 }, Write::Status { playing: false }]
        );
    }

    #[test]
    fn a_queue_edit_rewrites() {
        let mut m = active_with(&["1", "2", "3"]);
        assert_eq!(m.plan(&local("1", &["3", "2"], 0, true), 0), vec![Write::Queue(ids(&["1", "3", "2"])), Write::Current { index: 0, position_ms: 0 }]);
    }

    #[test]
    fn long_queues_are_capped_and_topped_up() {
        let all: Vec<String> = (0..300).map(|i| i.to_string()).collect();
        let mut m = active_with(&[]);
        let l = |from: usize| LocalPlayback { current: Some(all[from].clone()), future: all[from + 1..].to_vec(), position_ms: 0, playing: true, repeat: Repeat::Off, ..LocalPlayback::default() };
        let Some(Write::Queue(wrote)) = m.plan(&l(0), 0).into_iter().next() else { panic!("expected a rewrite") };
        assert_eq!(wrote.len(), MAX_FUTURE + 1);
        let entries = wrote.iter().map(|t| Entry { track_id: t.clone(), item_id: t.clone() }).collect();
        m.queue_written(wrote, entries, "b".into(), "r".into(), Repeat::Off);
        m.current_written(0, 0, true, 0, "r".into());
        assert_eq!(m.plan(&l(10), 0), vec![Write::Current { index: 10, position_ms: 0 }]);
        // Less than half of the cap is left written: the rest of it is added.
        assert_eq!(m.plan(&l(80), 0), vec![Write::Append { batch: "b".into(), tracks: all[101..181].to_vec(), capped: true }, Write::Current { index: 80, position_ms: 0 }]);
    }

    fn append(tracks: &[&str]) -> Write {
        Write::Append { batch: "b".into(), tracks: ids(tracks), capped: false }
    }

    fn entries(tracks: &[&str]) -> Vec<Entry> {
        tracks.iter().map(|t| Entry { track_id: (*t).into(), item_id: format!("i{t}") }).collect()
    }

    #[test]
    fn tracks_added_at_the_end_are_appended() {
        let mut m = active_with(&["1", "2", "3"]);
        assert_eq!(m.plan(&local("1", &["2", "3", "4"], 0, true), 0), vec![append(&["4"])]);
        m.appended(ids(&["4"]), false, entries(&["2", "3", "4"]), "r".into());
        assert!(m.plan(&local("1", &["2", "3", "4"], 0, true), 0).is_empty());
        assert_eq!(m.plan(&local("4", &[], 0, true), 0), vec![Write::Current { index: 3, position_ms: 0 }]);
        assert_eq!(m.entry(3).map(|e| e.item_id), Some("i4".into()));
    }

    #[test]
    fn a_track_added_to_a_queue_of_one_can_be_selected() {
        let mut m = active_with(&["1"]);
        assert_eq!(m.plan(&local("1", &["2"], 0, true), 0), vec![append(&["2"])]);
        m.appended(ids(&["2"]), false, entries(&["2"]), "r".into());
        assert_eq!(m.plan(&local("2", &[], 0, true), 0), vec![Write::Current { index: 1, position_ms: 0 }]);
    }

    #[test]
    fn repeat_turned_off_is_written_before_an_append() {
        let mut m = active_with(&["1", "2"]);
        m.repeat_written(Repeat::All, "r".into());
        assert_eq!(m.plan(&local("1", &["2", "3"], 0, true), 0), vec![Write::Repeat(Repeat::Off), append(&["3"])]);
    }

    #[test]
    fn an_appended_track_tidal_does_not_return_rewrites() {
        let mut m = active_with(&["1", "2"]);
        m.appended(ids(&["3"]), false, entries(&["2"]), "r".into());
        assert!(matches!(m.plan(&local("3", &[], 0, true), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn a_short_queue_grown_past_the_cap_is_topped_up_at_once() {
        let all: Vec<String> = (0..300).map(|i| i.to_string()).collect();
        let l = |upto: usize| LocalPlayback { current: Some(all[0].clone()), future: all[1..upto].to_vec(), playing: true, ..LocalPlayback::default() };
        let mut m = active_with(&[]);
        let Some(Write::Queue(wrote)) = m.plan(&l(60), 0).into_iter().next() else { panic!("expected a rewrite") };
        let entries = wrote.iter().map(|t| Entry { track_id: t.clone(), item_id: t.clone() }).collect();
        m.queue_written(wrote, entries, "b".into(), "r".into(), Repeat::Off);
        m.current_written(0, 0, true, 0, "r".into());
        // More than half of the new list is written, but nothing of it was left out before.
        assert_eq!(m.plan(&l(300), 0), vec![Write::Append { batch: "b".into(), tracks: all[60..101].to_vec(), capped: true }]);
    }

    #[test]
    fn repeat_one_writes_an_added_track_with_the_queue() {
        let mut m = active_with(&["1", "2"]);
        m.repeat_written(Repeat::One, "r".into());
        let one = |current: &str, future: &[&str]| LocalPlayback { repeat: Repeat::One, ..local(current, future, 0, true) };
        // TIDAL shows only the current item under repeat one: the entry of an appended track could not be read.
        assert!(matches!(m.plan(&one("1", &["2", "3"]), 0).first(), Some(Write::Queue(_))));
    }

    fn repeating(current: &str, future: &[&str], past: &[&str]) -> LocalPlayback {
        LocalPlayback { past: ids(past), repeat: Repeat::All, ..local(current, future, 0, true) }
    }

    #[test]
    fn repeat_all_writes_the_whole_list_once() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some(ME), Status::Playing), None, None);
        m.local_playing = true;
        // Started at the second of three: the first follows the last.
        assert_eq!(m.plan(&repeating("2", &["3"], &["1"]), 0).first(), Some(&Write::Queue(ids(&["2", "3", "1"]))));
        m.queue_written(ids(&["2", "3", "1"]), entries(&["2", "3", "1"]), "b".into(), "r".into(), Repeat::All);
        m.current_written(0, 0, true, 0, "r".into());
        // TIDAL repeats the batch: every track of the list, and the wrap to its first, only moves current.
        assert_eq!(m.plan(&repeating("3", &[], &["1", "2"]), 0), vec![Write::Current { index: 1, position_ms: 0 }]);
        m.current_written(1, 0, true, 0, "r".into());
        assert_eq!(m.plan(&repeating("1", &["2", "3"], &[]), 0), vec![Write::Current { index: 2, position_ms: 0 }]);
        m.current_written(2, 0, true, 0, "r".into());
        assert_eq!(m.plan(&repeating("2", &["3"], &["1"]), 0), vec![Write::Current { index: 0, position_ms: 0 }]);
        // A track added to the list is no append: it would land after the repeated ones.
        assert!(matches!(m.plan(&repeating("1", &["2", "3", "4"], &[]), 0).first(), Some(Write::Queue(_))));
        // Nor is a list that differs after the tracks that follow, or before them, the written one.
        assert!(matches!(m.plan(&repeating("2", &["3"], &["9"]), 0).first(), Some(Write::Queue(_))));
        assert!(matches!(m.plan(&repeating("3", &["9"], &["2"]), 0).first(), Some(Write::Queue(_))));
        assert!(matches!(m.plan(&repeating("3", &[], &["2", "1"]), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn repeat_all_turned_on_and_off_keeps_a_list_written_from_its_start() {
        let mut m = active_with(&["1", "2", "3"]);
        m.current_written(1, 0, true, 0, "r".into());
        assert_eq!(m.plan(&repeating("2", &["3"], &["1"]), 0), vec![Write::Repeat(Repeat::All)]);
        m.repeat_written(Repeat::All, "r".into());
        let off = LocalPlayback { past: ids(&["1"]), ..local("2", &["3"], 0, true) };
        assert_eq!(m.plan(&off, 0), vec![Write::Repeat(Repeat::Off)]);
    }

    #[test]
    fn repeat_all_writes_a_shorter_list_again() {
        let mut m = active_with(&["1", "2", "3"]);
        m.repeat_written(Repeat::All, "r".into());
        // TIDAL would repeat the first track with the two that are left.
        assert!(matches!(m.plan(&repeating("2", &["3"], &[]), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn repeat_all_turned_on_writes_a_list_started_later_again() {
        let mut m = active_with(&["2", "3"]);
        assert_eq!(m.plan(&repeating("2", &["3"], &["1"]), 0), vec![Write::Queue(ids(&["2", "3", "1"])), Write::Current { index: 0, position_ms: 0 }]);
    }

    #[test]
    fn discover_does_not_repeat() {
        let mut m = active_with(&["2", "3"]);
        let l = LocalPlayback { discover: true, ..repeating("2", &["3"], &["1"]) };
        assert!(m.plan(&l, 0).is_empty());
        // Its refill is appended.
        let more = LocalPlayback { discover: true, ..repeating("2", &["3", "4"], &["1"]) };
        assert_eq!(m.plan(&more, 0), vec![append(&["4"])]);
    }

    #[test]
    fn a_queue_taken_over_is_written_again_when_it_changes() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some(ME), Status::Playing), Some(head("r", "1", "phone")), None);
        m.adopt(Entry { track_id: "1".into(), item_id: "i1".into() }, &head("r", "1", "phone"), 0, 0);
        m.local_playing = true;
        assert!(m.adopted_future("r", entries(&["2"])));
        assert_eq!(m.plan(&local("1", &["2"], 0, true), 0), vec![Write::Current { index: 0, position_ms: 0 }]);
        m.current_written(0, 0, true, 0, "r".into());
        // Its batch on TIDAL is not known: nothing is appended to it, and it is not taken for a repeated list.
        assert!(matches!(m.plan(&local("1", &["2", "3"], 0, true), 0).first(), Some(Write::Queue(_))));
        assert!(matches!(m.plan(&repeating("2", &[], &["1"]), 0).first(), Some(Write::Queue(_))));
        // Not even as it is: TIDAL would repeat the tracks its batch holds from before the one taken over.
        assert!(matches!(m.plan(&repeating("1", &["2"], &[]), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn a_repeating_queue_taken_over_is_kept_as_it_is() {
        let all = Head { repeat: Repeat::All, ..head("r", "1", "phone") };
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some(ME), Status::Playing), Some(all.clone()), None);
        m.adopt(Entry { track_id: "1".into(), item_id: "i1".into() }, &all, 0, 0);
        m.local_playing = true;
        // What follows it was read as TIDAL repeats it: the rest of the batch, then its start.
        assert!(m.adopted_future("r", entries(&["2", "3"])));
        assert_eq!(m.plan(&repeating("1", &["2", "3"], &[]), 0), vec![Write::Current { index: 0, position_ms: 0 }]);
    }

    #[test]
    fn starting_after_a_lapsed_attachment_claims_again() {
        let mut m = active_with(&["1"]);
        m.fetched(state(None, Status::Paused), None, None);
        m.local_playing = false;
        assert_eq!(m.plan(&local("1", &[], 0, true), 0).first(), Some(&Write::Claim));
    }

    #[test]
    fn a_failed_claim_after_a_lapse_is_retried() {
        let mut m = active_with(&["1"]);
        m.fetched(state(None, Status::Paused), None, None);
        m.local_playing = false;
        assert_eq!(m.plan(&local("1", &[], 0, true), 0).first(), Some(&Write::Claim));
        m.write_failed();
        assert_eq!(m.plan(&local("1", &[], 0, true), 500).first(), Some(&Write::Claim));
    }

    #[test]
    fn unknown_ownership_claims_before_writing() {
        let mut m = active_with(&["1", "2"]);
        m.ownership_unknown();
        assert_eq!(m.plan(&local("1", &["2"], 0, true), 0).first(), Some(&Write::Claim));
    }

    #[test]
    fn a_failed_claim_is_retried() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Playing), None, None);
        assert_eq!(m.plan(&local("1", &[], 0, true), 0).first(), Some(&Write::Claim));
        m.write_failed();
        assert_eq!(m.plan(&local("1", &[], 0, true), 500).first(), Some(&Write::Claim));
    }

    #[test]
    fn a_push_to_the_active_device_arrives() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some(ME), Status::Playing), Some(head("r1", "1", ME)), None);
        // The sender claimed and handed back between two reads.
        let out = m.fetched(state(Some(ME), Status::Paused), Some(head("r2", "5", "phone")), None);
        assert!(out.iter().any(|o| matches!(o, Out::Arrive)));
        // A read of the same queue again is no new push.
        assert!(!m.fetched(state(Some(ME), Status::Paused), Some(head("r2", "5", "phone")), None).iter().any(|o| matches!(o, Out::Arrive)));
    }

    #[test]
    fn stopping_locally_pauses() {
        let mut m = active_with(&["1"]);
        let stopped = LocalPlayback { current: None, future: vec![], position_ms: 0, playing: false, repeat: Repeat::Off, ..LocalPlayback::default() };
        assert_eq!(m.plan(&stopped, 0), vec![Write::Status { playing: false }]);
    }

    #[test]
    fn a_queue_without_a_selection_is_created_first() {
        let mut m = Mirror::new(ME.into());
        m.fetched(State { queue_id: None, ..state(Some(ME), Status::Idle) }, None, None);
        assert_eq!(m.plan(&local("1", &[], 0, true), 0).first(), Some(&Write::NewQueue));
    }

    #[test]
    fn someone_else_writing_the_queue_forces_a_rewrite() {
        let mut m = active_with(&["1", "2"]);
        m.fetched(state(Some(ME), Status::Playing), Some(head("other", "1", "phone")), None);
        assert!(matches!(m.plan(&local("1", &["2"], 0, true), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn being_made_active_by_another_device_arrives() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Playing), Some(head("r", "1", "phone")), None);
        let out = m.fetched(state(Some(ME), Status::Paused), Some(head("r", "1", "phone")), None);
        assert!(out.iter().any(|o| matches!(o, Out::Arrive)));
        assert!(m.is_active());
    }

    #[test]
    fn active_at_start_is_not_an_arrival() {
        let mut m = Mirror::new(ME.into());
        assert!(!m.fetched(state(Some(ME), Status::Paused), None, None).iter().any(|o| matches!(o, Out::Arrive)));
    }

    #[test]
    fn losing_the_active_player_moves_to_the_new_one() {
        let mut m = active_with(&["1"]);
        let out = m.fetched(state(Some("phone"), Status::Playing), Some(head("r2", "1", "phone")), None);
        assert!(events(&out).iter().any(|e| matches!(e, HandoffEvent::MovedTo(d) if *d == dev("phone", "iPhone"))));
        assert!(!m.is_active());
    }

    #[test]
    fn a_lapsed_attachment_is_not_a_move() {
        let mut m = active_with(&["1"]);
        assert!(events(&m.fetched(state(None, Status::Paused), None, None)).iter().all(|e| !matches!(e, HandoffEvent::MovedTo(_))));
        assert!(!m.fetched(state(Some(ME), Status::Paused), None, None).iter().any(|o| matches!(o, Out::Arrive)));
    }

    #[test]
    fn elsewhere_follows_the_other_device() {
        let mut m = Mirror::new(ME.into());
        let out = m.fetched(state(Some("phone"), Status::Playing), Some(head("r", "7", "phone")), None);
        let e = events(&out);
        assert!(e.iter().any(|e| matches!(e, HandoffEvent::Devices(d) if *d == vec![dev("phone", "iPhone")])));
        let Some(HandoffEvent::Elsewhere(Some(el))) = e.iter().find(|e| matches!(e, HandoffEvent::Elsewhere(_))) else { panic!("no elsewhere") };
        assert_eq!((el.device.name.as_str(), el.track.id(), el.playing, el.position_ms), ("iPhone", "7", true, 10_000));
        // Same state again: no repeated events.
        assert!(m.fetched(state(Some("phone"), Status::Playing), Some(head("r", "7", "phone")), None).is_empty());
    }

    #[test]
    fn a_closed_device_still_offers_its_paused_queue() {
        let mut m = Mirror::new(ME.into());
        let out = m.fetched(State { players: vec![dev(ME, "Desktop")], ..state(None, Status::Paused) }, Some(head("r", "7", "laptop")), Some(dev("laptop", "Laptop")));
        assert!(events(&out).iter().any(|e| matches!(e, HandoffEvent::Elsewhere(Some(el)) if el.device.name == "Laptop" && !el.playing)));
    }

    #[test]
    fn own_playback_is_not_elsewhere() {
        let mut m = Mirror::new(ME.into());
        let out = m.fetched(state(None, Status::Paused), Some(head("r", "7", ME)), Some(dev(ME, "Desktop")));
        assert!(events(&out).iter().any(|e| matches!(e, HandoffEvent::Elsewhere(None))));
        let out = m.fetched(state(Some(ME), Status::Playing), Some(head("r", "7", "phone")), None);
        assert!(!events(&out).iter().any(|e| matches!(e, HandoffEvent::Elsewhere(Some(_)))));
    }

    #[test]
    fn positions_extrapolate_while_playing() {
        assert_eq!(position_now(10_000, 1_000, 6_000, true, 200_000), 15_000);
        assert_eq!(position_now(10_000, 1_000, 6_000, false, 200_000), 10_000);
        assert_eq!(position_now(10_000, 1_000, 900_000, true, 200_000), 199_000);
        assert_eq!(position_now(10_000, 5_000, 1_000, true, 200_000), 10_000);
        assert_eq!(position_now(i64::MAX - 5, i64::MIN, i64::MAX, true, 200_000), 199_000);
    }

    #[test]
    fn an_adopted_queue_writes_only_the_position() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Paused), None, None);
        m.claimed(Some(ME.into()));
        let h = head("r", "1", "phone");
        m.adopt(Entry { track_id: "1".into(), item_id: "a".into() }, &h, 10_000, 0);
        // The app plays the current track before what follows arrives.
        assert_eq!(m.plan(&local("1", &[], 10_000, true), 100), vec![Write::Current { index: 0, position_ms: 10_000 }, Write::Status { playing: true }]);
    }

    #[test]
    fn the_future_read_after_an_adoption_writes_only_the_position() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Paused), None, None);
        m.claimed(Some(ME.into()));
        m.adopt(Entry { track_id: "1".into(), item_id: "a".into() }, &head("r", "1", "phone"), 10_000, 0);
        let future = vec![Entry { track_id: "2".into(), item_id: "b".into() }, Entry { track_id: "3".into(), item_id: "c".into() }];
        assert!(m.adopted_future("r", future.clone()));
        // A second read of it is not appended again.
        assert!(!m.adopted_future("r", future));
        assert_eq!(
            m.plan(&local("1", &["2", "3"], 10_000, true), 100),
            vec![Write::Current { index: 0, position_ms: 10_000 }, Write::Status { playing: true }]
        );
        assert_eq!(m.entry(1), Some(Entry { track_id: "2".into(), item_id: "b".into() }));
        m.current_written(0, 10_000, true, 100, "r2".into());
        m.status_written(true);
        assert_eq!(m.plan(&local("2", &["3"], 0, true), 200), vec![Write::Current { index: 1, position_ms: 0 }]);
    }

    #[test]
    fn a_future_read_at_another_revision_is_not_adopted() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Paused), None, None);
        m.claimed(Some(ME.into()));
        m.adopt(Entry { track_id: "1".into(), item_id: "a".into() }, &head("r", "1", "phone"), 10_000, 0);
        assert!(!m.adopted_future("r2", vec![Entry { track_id: "2".into(), item_id: "b".into() }]));
        assert!(matches!(m.plan(&local("1", &["2"], 10_000, true), 100).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn entries_not_read_back_are_written_again() {
        let mut m = active_with(&[]);
        // A send reads back only the current entry.
        m.queue_written(ids(&["1", "2", "3"]), vec![Entry { track_id: "1".into(), item_id: "i1".into() }], "b".into(), "r".into(), Repeat::Off);
        m.current_written(0, 0, true, 0, "r".into());
        assert_eq!(m.plan(&local("1", &["2", "3"], 30_000, false), 30_000), vec![Write::Current { index: 0, position_ms: 30_000 }, Write::Status { playing: false }]);
        assert_eq!(m.plan(&local("2", &["3"], 0, true), 31_000).first(), Some(&Write::Queue(ids(&["2", "3"]))));
    }

    #[test]
    fn a_move_is_told_from_the_state_alone_and_once() {
        let mut m = active_with(&["1"]);
        let s = state(Some("phone"), Status::Playing);
        assert!(matches!(m.moved(&s), Some(HandoffEvent::MovedTo(d)) if d == dev("phone", "iPhone")));
        assert!(!m.is_active());
        // The pause this causes is not written over the new player's state.
        assert!(m.plan(&local("1", &[], 5_000, false), 5_000).is_empty());
        // The queue read failed: the next read tells nothing again.
        assert!(m.moved(&s).is_none());
        let out = m.fetched(s, Some(head("r2", "1", "phone")), None);
        assert!(events(&out).iter().all(|e| !matches!(e, HandoffEvent::MovedTo(_))));
    }

    #[test]
    fn only_a_named_other_player_is_an_early_move() {
        let mut m = active_with(&["1"]);
        assert!(m.moved(&state(None, Status::Paused)).is_none());
        assert!(m.moved(&state(Some(ME), Status::Paused)).is_none());
        let mut idle = Mirror::new(ME.into());
        assert!(idle.moved(&state(Some("phone"), Status::Playing)).is_none());
        idle.fetched(state(Some("laptop"), Status::Playing), None, None);
        assert!(idle.moved(&state(Some("phone"), Status::Playing)).is_none());
    }
}

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
    /// TIDAL's entries for them, as read back.
    entries: Vec<Entry>,
    cur: usize,
    revision: String,
    repeat: Repeat,
    position_ms: i64,
    at_ms: i64,
    playing: bool,
}

impl Written {
    /// Index of the written entry the local list starts at, the current one first.
    fn find(&self, tracks: &[String]) -> Option<usize> {
        // Only a capped write may lag behind a still capped local list; it
        // is topped up once half of it is played.
        let capped = self.wrote.len() == MAX_FUTURE + 1 && tracks.len() == MAX_FUTURE + 1;
        let matches = |k: usize| {
            let rest = &self.wrote[k..];
            rest == tracks || (capped && 2 * rest.len() >= tracks.len() && tracks.starts_with(rest))
        };
        (self.cur..self.wrote.len()).chain(0..self.cur.min(self.wrote.len())).find(|&k| matches(k))
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
    let p = if playing { position_ms + (now_ms - at_ms).max(0) } else { position_ms };
    p.min(duration_ms - 1_000).max(0)
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
    /// starting to play claims that role.
    pub(super) fn plan(&mut self, local: &LocalPlayback, now_ms: i64) -> Vec<Write> {
        let started = local.playing && !self.local_playing;
        self.local_playing = local.playing;
        let mut out = Vec::new();
        if self.state.active.as_deref() != Some(self.me.as_str()) {
            // Also after this device's own attachment lapsed: claiming is harmless then.
            if started && local.current.is_some() {
                out.push(Write::Claim);
            } else if !self.is_active() {
                return out;
            }
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
        let tracks: Vec<String> = std::iter::once(cur).chain(local.future.iter().take(MAX_FUTURE)).cloned().collect();
        if let Some((w, index)) = self.written.as_ref().and_then(|w| w.find(&tracks).map(|i| (w, i))) {
            if w.repeat != local.repeat {
                out.push(Write::Repeat(local.repeat));
            }
            let expected = if w.playing { w.position_ms + (now_ms - w.at_ms) } else { w.position_ms };
            if index != w.cur || self.need_position || w.playing != local.playing || (local.position_ms - expected).abs() > SEEK_MS {
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
        let w = self.written.as_ref()?;
        w.entries.get(index).filter(|e| w.wrote.get(index) == Some(&e.track_id)).cloned()
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

    pub(super) fn queue_written(&mut self, wrote: Vec<String>, entries: Vec<Entry>, revision: String, repeat: Repeat) {
        self.written = Some(Written { wrote, entries, cur: 0, revision, repeat, position_ms: 0, at_ms: 0, playing: false });
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
        if !self.is_active() {
            self.local_playing = false;
        }
    }

    /// Takes over the queue as read for a resume, so the first report after
    /// it only writes the position and status.
    pub(super) fn adopt(&mut self, entries: Vec<Entry>, head: &Head, position_ms: i64, now_ms: i64) {
        let wrote = entries.iter().map(|e| e.track_id.clone()).collect();
        self.written =
            Some(Written { wrote, entries, cur: 0, revision: head.revision.clone(), repeat: head.repeat, position_ms, at_ms: now_ms, playing: false });
        self.need_position = true;
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
            _ => false,
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
        LocalPlayback { current: Some(current.into()), future: ids(future), position_ms, playing, repeat: Repeat::Off }
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
        Track { info: TrackInfo { id: id.into(), duration: std::time::Duration::from_secs(200), ..TrackInfo::default() }, artists: vec![], album: None }
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
        m.queue_written(ids(tracks), entries, "r".into(), Repeat::Off);
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
        let l = |from: usize| LocalPlayback { current: Some(all[from].clone()), future: all[from + 1..].to_vec(), position_ms: 0, playing: true, repeat: Repeat::Off };
        let Some(Write::Queue(wrote)) = m.plan(&l(0), 0).into_iter().next() else { panic!("expected a rewrite") };
        assert_eq!(wrote.len(), MAX_FUTURE + 1);
        let entries = wrote.iter().map(|t| Entry { track_id: t.clone(), item_id: t.clone() }).collect();
        m.queue_written(wrote, entries, "r".into(), Repeat::Off);
        m.current_written(0, 0, true, 0, "r".into());
        assert_eq!(m.plan(&l(10), 0), vec![Write::Current { index: 10, position_ms: 0 }]);
        assert!(matches!(m.plan(&l(80), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn tracks_added_to_a_short_queue_are_written() {
        let mut m = active_with(&["1", "2", "3"]);
        assert!(matches!(m.plan(&local("1", &["2", "3", "4"], 0, true), 0).first(), Some(Write::Queue(_))));
        m.current_written(1, 0, true, 0, "r".into());
        assert!(matches!(m.plan(&local("2", &["3", "4"], 0, true), 0).first(), Some(Write::Queue(_))));
    }

    #[test]
    fn starting_after_a_lapsed_attachment_claims_again() {
        let mut m = active_with(&["1"]);
        m.fetched(state(None, Status::Paused), None, None);
        m.local_playing = false;
        assert_eq!(m.plan(&local("1", &[], 0, true), 0).first(), Some(&Write::Claim));
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
        let stopped = LocalPlayback { current: None, future: vec![], position_ms: 0, playing: false, repeat: Repeat::Off };
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
    }

    #[test]
    fn an_adopted_queue_writes_only_the_position() {
        let mut m = Mirror::new(ME.into());
        m.fetched(state(Some("phone"), Status::Paused), None, None);
        m.claimed(Some(ME.into()));
        let h = head("r", "1", "phone");
        m.adopt(vec![Entry { track_id: "1".into(), item_id: "a".into() }, Entry { track_id: "2".into(), item_id: "b".into() }], &h, 10_000, 0);
        assert_eq!(
            m.plan(&local("1", &["2"], 10_000, true), 100),
            vec![Write::Current { index: 0, position_ms: 10_000 }, Write::Status { playing: true }]
        );
        assert_eq!(m.entry(1), Some(Entry { track_id: "2".into(), item_id: "b".into() }));
    }
}

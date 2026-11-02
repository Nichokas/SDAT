//! Host and guest state machines. Pure: the transport feeds them messages and
//! the current monotonic time, and sends whatever they return.

use std::collections::BTreeMap;

use super::clock::{ClockSync, Sample};
use super::drift::{Correction, DriftController};
use super::proto::{Anchor, EndReason, FEATURE_PLAY_NEXT, GuestCommand, Msg, RejectReason};
use super::queue::{Accepted, GuestApply, GuestQueue, HostQueue, Op, OpKind, QueueError, QueueItem};
use super::{HEARTBEAT_NS, HOST_TIMEOUT_NS, PROTO_VERSION, new_id};

/// Guests per session in the MVP star topology.
pub const MAX_GUESTS: usize = 8;
/// Host playback changes closer than this are coalesced (seek spam).
pub const DEBOUNCE_NS: i64 = 300_000_000;

#[derive(Debug, Clone, PartialEq)]
pub enum To {
    Peer(String),
    All,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Out {
    pub to: To,
    pub msg: Msg,
}

fn to(peer: &str, msg: Msg) -> Out {
    Out { to: To::Peer(peer.to_owned()), msg }
}
/// Whether a catch-up batch fits one frame; otherwise a snapshot is sent.
fn fits_frame(ops: &[super::queue::VersionedOp]) -> bool {
    super::proto::encode(&Msg::Ops { ops: ops.to_vec() }).is_ok()
}
fn all(msg: Msg) -> Out {
    Out { to: To::All, msg }
}

/// Most track ids one `Enqueue` command may carry.
pub const MAX_ENQUEUE: usize = 50;
/// Longest accepted TIDAL track id (digits only).
pub const MAX_TRACK_ID_LEN: usize = 20;
/// Per guest: at most [`CMD_BURST`] commands in any [`CMD_WINDOW_NS`].
pub const CMD_BURST: usize = 5;
pub const CMD_WINDOW_NS: i64 = 2_000_000_000;
/// Accepted commands waiting for the app; older ones are dropped beyond this.
pub const MAX_PENDING_CMDS: usize = 32;

/// A guest command accepted by the host, for the app to apply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCommand {
    pub peer: String,
    pub name: String,
    pub cmd: GuestCommand,
    /// Guest Play Next: `cmd` is an `Enqueue` whose tracks go right after
    /// the playing one. Insert them at the index from
    /// [`HostSession::play_next_offset`]. Only set when the app enabled it
    /// with [`HostSession::set_play_next`]; otherwise it is a plain enqueue.
    pub next: bool,
}

/// Whether `cmd` is well formed: known, and for `Enqueue` 1..=[`MAX_ENQUEUE`]
/// TIDAL track ids (ASCII digits, at most [`MAX_TRACK_ID_LEN`]).
pub fn valid_command(cmd: &GuestCommand) -> bool {
    match cmd {
        GuestCommand::Play | GuestCommand::Pause | GuestCommand::Next => true,
        GuestCommand::Enqueue { track_ids } => {
            !track_ids.is_empty()
                && track_ids.len() <= MAX_ENQUEUE
                && track_ids.iter().all(|t| !t.is_empty() && t.len() <= MAX_TRACK_ID_LEN && t.bytes().all(|b| b.is_ascii_digit()))
        }
        GuestCommand::Unknown => false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuestInfo {
    pub peer: String,
    pub name: String,
}

#[derive(Debug)]
pub struct HostSession {
    pub session_id: String,
    pub secret: String,
    pub name: String,
    epoch: u64,
    queue: HostQueue,
    anchor: Anchor,
    pending_anchor: bool,
    last_anchor_sent_ns: i64,
    last_heartbeat_ns: i64,
    guests: BTreeMap<String, GuestInfo>,
    /// Last answered `Resync` per guest (see [`RESYNC_MIN_NS`]).
    last_resync_ns: BTreeMap<String, i64>,
    /// Guests may send playback commands. Off by default.
    guest_control: bool,
    /// Recent command times per guest (rate limit).
    cmd_times: BTreeMap<String, std::collections::VecDeque<i64>>,
    pending_cmds: std::collections::VecDeque<HostCommand>,
    /// The app can insert guest Play Next tracks after the playing one.
    play_next: bool,
    /// The current Play Next run, see [`Self::play_next_offset`].
    run: PlayNextRun,
    /// The app's whole queue with its own entry ids, when it uses
    /// [`Self::replace_queue_entries`]; `None` on the inferring path.
    app_queue: Option<Vec<QueueItem>>,
}

/// Most entries [`HostSession::replace_queue_entries`] accepts (the app's
/// whole queue; only the first [`super::MAX_QUEUE`] are shared).
pub const MAX_APP_QUEUE: usize = 20_000;

/// Guest Play Next tracks inserted after one playing entry, by identity.
#[derive(Debug, Default)]
struct PlayNextRun {
    /// Playing entry the run belongs to.
    after: Option<String>,
    /// Entry ids of inserted tracks, in insertion order.
    items: Vec<String>,
    /// Track ids handed out and not yet seen in a queue update, in order
    /// (at most [`MAX_APP_QUEUE`]).
    pending: Vec<String>,
    /// Handed-out tracks past `pending`'s bound: they only count for offsets.
    hidden: usize,
}

impl PlayNextRun {
    fn offset(&self) -> usize {
        self.items.len() + self.pending.len() + self.hidden
    }

    /// Keeps `pending` bounded; the overflow is only counted.
    fn set_pending(&mut self, pending: Vec<String>) {
        self.pending = pending;
        if self.pending.len() > MAX_APP_QUEUE {
            self.hidden += self.pending.len() - MAX_APP_QUEUE;
            self.pending.truncate(MAX_APP_QUEUE);
        }
    }

    fn add_pending(&mut self, track_ids: &[String]) {
        let mut pending = std::mem::take(&mut self.pending);
        pending.extend_from_slice(track_ids);
        self.set_pending(pending);
    }
}

/// Whether `new` is `old` with the `batch` of tracks inserted at `at` and
/// the overflow pushed past the cap (and not simply `old` unchanged).
fn shifted_by_insert(old: &[&str], new: &[String], at: usize, batch: &[String]) -> bool {
    let len = new.len();
    let vis = batch.len().min(len.saturating_sub(at));
    let same = |n: &[String], o: &[&str]| n.len() == o.len() && n.iter().zip(o).all(|(a, b)| a == b);
    old.len() == len && vis > 0 && new[at..at + vis] == batch[..vis] && same(&new[at + vis..], &old[at..len - vis]) && !same(&new[at..], &old[at..])
}

/// The run entries right after `start` in `items`, in queue order.
fn contiguous_run(items: &[QueueItem], start: usize, run: &[String]) -> Vec<String> {
    let run: std::collections::HashSet<&str> = run.iter().map(String::as_str).collect();
    items[start..].iter().take_while(|i| run.contains(i.item_id.as_str())).map(|i| i.item_id.clone()).collect()
}

/// Entry ids for a queue replaced by `new` track ids: entries kept from
/// `old` (a longest common subsequence of tracks, in order) keep their id,
/// the rest get fresh ones. The flag tells whether the id is new. When the
/// match is ambiguous (two copies of a track, one removed), entries in
/// `prefer_drop` (guest Play Next entries) are the ones taken as removed, so
/// a removed Play Next never borrows the identity of another copy, and the
/// `playing` entry is the copy kept.
fn stable_ids(old: &[QueueItem], new: &[String], prefer_drop: &[String], playing: Option<&str>) -> Vec<(String, bool)> {
    let (old_len, new_len) = (old.len(), new.len());
    // Weight of keeping old[oi]: every match counts the same (longest
    // subsequence first); keeping the playing entry, then a non-run entry,
    // breaks ties.
    let drop: std::collections::HashSet<&str> = prefer_drop.iter().map(String::as_str).collect();
    let weights: Vec<u32> = old
        .iter()
        .map(|i| {
            // A match (2048) outweighs every tie-break together; keeping the
            // playing entry (1024) outweighs all non-run bonuses (1 each,
            // at most 500).
            2048 + if Some(i.item_id.as_str()) == playing {
                1024
            } else {
                u32::from(!drop.contains(i.item_id.as_str()))
            }
        })
        .collect();
    let weight = |oi: usize| weights[oi];
    // best[oi][ni]: best weight for old[oi..] and new[ni..]. At most 501 x 501.
    let mut best = vec![vec![0u32; new_len + 1]; old_len + 1];
    for oi in (0..old_len).rev() {
        for ni in (0..new_len).rev() {
            let keep = if old[oi].track_id == new[ni] { best[oi + 1][ni + 1] + weight(oi) } else { 0 };
            best[oi][ni] = keep.max(best[oi + 1][ni]).max(best[oi][ni + 1]);
        }
    }
    let mut out = Vec::with_capacity(new_len);
    let (mut oi, mut ni) = (0, 0);
    while ni < new_len {
        if oi < old_len && old[oi].track_id == new[ni] && best[oi][ni] == best[oi + 1][ni + 1] + weight(oi) {
            out.push((old[oi].item_id.clone(), false));
            oi += 1;
            ni += 1;
        } else if oi < old_len && best[oi][ni] == best[oi + 1][ni] {
            oi += 1;
        } else {
            out.push((new_id(), true));
            ni += 1;
        }
    }
    out
}

impl HostSession {
    pub fn new(name: String, now_ns: i64) -> Self {
        Self {
            session_id: new_id(),
            secret: new_id(),
            name: match name.char_indices().nth(40) {
                Some((cut, _)) => name[..cut].to_owned(),
                None => name,
            },
            // Distinct per run so a restarted host always wins over stale state.
            epoch: u64::try_from(now_ns.max(0)).unwrap_or(0) ^ u64::from(rand::random::<u32>()) | 1,
            queue: HostQueue::new(),
            anchor: Anchor { epoch: 0, seq: 0, queue_version: 0, item_id: None, track_id: None, position_ms: 0, host_mono_ns: now_ns, playing: false },
            pending_anchor: false,
            last_anchor_sent_ns: i64::MIN / 2,
            last_heartbeat_ns: now_ns,
            guests: BTreeMap::new(),
            last_resync_ns: BTreeMap::new(),
            guest_control: false,
            cmd_times: BTreeMap::new(),
            pending_cmds: std::collections::VecDeque::new(),
            play_next: false,
            run: PlayNextRun::default(),
            app_queue: None,
        }
        .with_epoch()
    }

    fn with_epoch(mut self) -> Self {
        self.anchor.epoch = self.epoch;
        self
    }

    pub fn guests(&self) -> Vec<GuestInfo> {
        self.guests.values().cloned().collect()
    }

    pub fn anchor(&self) -> &Anchor {
        &self.anchor
    }

    pub fn queue_items(&self) -> &[QueueItem] {
        &self.queue.queue().items
    }

    pub fn guest_control(&self) -> bool {
        self.guest_control
    }

    /// Allows or forbids guest playback commands and tells every guest.
    /// Forbidding also drops commands not yet taken by the app.
    pub fn set_guest_control(&mut self, allowed: bool) -> Vec<Out> {
        if allowed == self.guest_control {
            return Vec::new();
        }
        self.guest_control = allowed;
        if !allowed {
            self.pending_cmds.clear();
        }
        vec![all(Msg::Control { guest_control: allowed })]
    }

    /// Declares that the app applies guest Play Next ([`HostCommand::next`]).
    /// Off by default, so apps that do not handle it keep getting plain
    /// enqueues. Advertised as [`FEATURE_PLAY_NEXT`] in the `Welcome`, and
    /// to guests already joined with a `Features` message.
    pub fn set_play_next(&mut self, enabled: bool) -> Vec<Out> {
        if enabled == self.play_next {
            return Vec::new();
        }
        self.play_next = enabled;
        vec![all(Msg::Features { features: self.features() })]
    }

    pub fn play_next(&self) -> bool {
        self.play_next
    }

    fn features(&self) -> Vec<String> {
        if self.play_next { vec![FEATURE_PLAY_NEXT.to_owned()] } else { Vec::new() }
    }

    /// Where to insert play-next `track_ids`: this many entries after the
    /// playing one (insert at `index_of_playing + 1 + offset`, clamped to the
    /// queue length). Several Play Next requests keep their request order
    /// and all stay ahead of the rest of the queue.
    ///
    /// The run is tracked by entry identity: the next [`Self::replace_queue`]
    /// binds the handed-out tracks to the new entries found at those slots,
    /// and entry ids survive refreshes. Play-next entries the host removed or
    /// moved away stop counting; other copies of the same track never count.
    /// The run restarts when another entry starts playing.
    pub fn play_next_offset(&mut self, track_ids: &[String]) -> usize {
        self.sync_run();
        if let Some(app) = &self.app_queue {
            // Exact: the run is the run entries right after the playing one
            // in the app's whole queue.
            self.run.items = match self.anchor_start(app) {
                Some(p) => contiguous_run(app, p, &self.run.items),
                None => Vec::new(),
            };
            let offset = self.run.offset();
            self.run.add_pending(track_ids);
            return offset;
        }
        let items = &self.queue.queue().items;
        self.run.items = if let Some(p) = self.run_start() {
            // The run entries still right after the playing one.
            contiguous_run(items, p, &self.run.items)
        } else {
            // Playing entry past the queue cap:
            // keep counting the run so requests still keep their order.
            let present: std::collections::HashSet<&str> = items.iter().map(|i| i.item_id.as_str()).collect();
            self.run.items.iter().filter(|r| present.contains(r.as_str())).cloned().collect()
        };
        let offset = self.run.offset();
        self.run.add_pending(track_ids);
        offset
    }

    /// Where Play Next inserts in the shared queue: after the playing entry,
    /// or the front when nothing plays; `None` when the playing entry is
    /// outside the shared queue.
    fn run_start(&self) -> Option<usize> {
        match &self.anchor.item_id {
            None => Some(0),
            Some(id) => self.queue.queue().items.iter().position(|i| &i.item_id == id).map(|p| p + 1),
        }
    }

    /// Insertion point after the playing entry in `items` (front when
    /// nothing plays); `None` when the playing entry is not in `items`.
    fn anchor_start(&self, items: &[QueueItem]) -> Option<usize> {
        match &self.anchor.item_id {
            None => Some(0),
            Some(id) => items.iter().position(|i| &i.item_id == id).map(|p| p + 1),
        }
    }

    /// Replaces the shared queue from the app's whole queue, with the app's
    /// own stable entry ids (at most [`MAX_APP_QUEUE`]; the first
    /// [`super::MAX_QUEUE`] are shared). Identity is exact: no diffing, and
    /// entries past the shared cap still count, so a Play Next insertion is
    /// told apart from an existing copy of the same track even at the cap.
    /// Pass the playing entry's id to [`Self::set_playback`].
    pub fn replace_queue_entries(&mut self, entries: Vec<QueueItem>) -> Result<Vec<Out>, QueueError> {
        if entries.len() > MAX_APP_QUEUE {
            return Err(QueueError::Full);
        }
        let mut seen = std::collections::HashSet::new();
        for e in &entries {
            if !e.is_valid() {
                return Err(QueueError::InvalidItem);
            }
            if !seen.insert(e.item_id.as_str()) {
                return Err(QueueError::DuplicateItem(e.item_id.clone()));
            }
        }
        self.sync_run();
        let known: std::collections::HashSet<&str> = match &self.app_queue {
            Some(app) => app.iter().map(|i| i.item_id.as_str()).collect(),
            None => self.queue.queue().items.iter().map(|i| i.item_id.as_str()).collect(),
        };
        // Bind handed-out tracks to entries the app added right after the
        // playing entry and the run; an entry the app already had never
        // joins the run.
        // Requests the app has not published yet stay pending.
        let mut run = Vec::new();
        let mut bound = 0;
        if let Some(start) = self.anchor_start(&entries) {
            run = contiguous_run(&entries, start, &self.run.items);
            let next = entries[start + run.len()..].iter().zip(&self.run.pending);
            for (e, t) in next.take_while(|(e, t)| &e.track_id == *t && !known.contains(e.item_id.as_str())) {
                debug_assert_eq!(&e.track_id, t);
                run.push(e.item_id.clone());
                bound += 1;
            }
        }
        let rest = self.run.pending[bound..].to_vec();
        let shared = entries.iter().take(super::MAX_QUEUE).cloned().collect();
        let out = self.apply_shared(OpKind::Replace { items: shared })?;
        self.run.items = run;
        // The whole queue is known here: uncounted overflow cannot be told
        // apart from dropped requests any more, so it is retired.
        self.run.hidden = 0;
        self.run.set_pending(rest);
        self.app_queue = Some(entries);
        Ok(out)
    }

    /// Starts a new run when another entry is playing.
    fn sync_run(&mut self) {
        if self.run.after != self.anchor.item_id {
            self.run = PlayNextRun { after: self.anchor.item_id.clone(), ..PlayNextRun::default() };
        }
    }

    /// Replaces the shared queue with `track_ids` (at most
    /// [`super::MAX_QUEUE`]); returns the entry id of each track. Entries
    /// that stay keep their id, so the playing entry and the Play Next run
    /// survive refreshes.
    pub fn replace_queue(&mut self, track_ids: Vec<String>) -> Result<(Vec<Out>, Vec<String>), QueueError> {
        self.sync_run();
        let track_ids: Vec<String> = track_ids.into_iter().take(super::MAX_QUEUE).collect();
        if self.app_queue.is_some() {
            // Back to inferring: the run's ids are the app's, start over.
            self.run = PlayNextRun { after: self.anchor.item_id.clone(), ..PlayNextRun::default() };
        }
        let old_tracks: Vec<&str> = self.queue.queue().items.iter().map(|i| i.track_id.as_str()).collect();
        let mut ids = stable_ids(&self.queue.queue().items, &track_ids, &self.run.items, self.anchor.item_id.as_deref());
        let capped = ids.len() == super::MAX_QUEUE;
        // Bind handed-out tracks to the entries right after the playing one
        // and the run. Pending requests inside the shared queue are consumed
        // by every replace (they landed now or the app dropped them); the run
        // only changes once the queue accepted the replacement.
        let mut run = self.run.items.clone();
        // Requests that would land past the cap stay pending: the app still
        // has them, the shared queue just cannot show them.
        let mut beyond_cap = Vec::new();
        // Nothing playing: Play Next goes to the front of the queue.
        let start = match &self.anchor.item_id {
            None => Some(0),
            Some(id) => ids.iter().position(|(i, _)| i == id).map(|p| p + 1),
        };
        if let Some(start) = start {
            // Run entries moved away from the playing one stop counting.
            let kept: std::collections::HashSet<&str> = run.iter().map(String::as_str).collect();
            let contiguous: Vec<String> = ids[start..].iter().take_while(|(i, _)| kept.contains(i.as_str())).map(|(i, _)| i.clone()).collect();
            run = contiguous;
            let first = start + run.len();
            let batch = &self.run.pending;
            let inferred = capped && !batch.is_empty() && shifted_by_insert(&old_tracks, &track_ids, first, batch);
            if inferred {
                // The whole batch landed at the cap and pushed old entries
                // out of view: bind the visible part, keep the rest pending.
                let vis = batch.len().min(ids.len() - first);
                run.extend(ids[first..first + vis].iter().map(|(i, _)| i.clone()));
                beyond_cap = batch[vis..].to_vec();
            }
            for (n, t) in batch.iter().enumerate().filter(|_| !inferred) {
                let j = first + n;
                if j >= ids.len() {
                    if capped {
                        beyond_cap = self.run.pending[n..].to_vec();
                    }
                    break;
                }
                if &track_ids[j] != t {
                    break;
                }
                if !ids[j].1 {
                    // Inserted before another copy of the same track: the
                    // diff gave the old id to this copy. The old entry is the
                    // nearest later copy that got a fresh id, so they swap.
                    let fresh = (j + 1..ids.len()).find(|&k| ids[k].1 && &track_ids[k] == t);
                    match fresh {
                        Some(k) => ids.swap(j, k),
                        None => break,
                    }
                }
                run.push(ids[j].0.clone());
            }
        } else if capped {
            // The playing entry itself is past the cap.
            beyond_cap.clone_from(&self.run.pending);
        }
        let items: Vec<QueueItem> = ids.iter().zip(&track_ids).map(|((item_id, _), t)| QueueItem { item_id: item_id.clone(), track_id: t.clone() }).collect();
        let out = self.queue_op(OpKind::Replace { items })?;
        self.app_queue = None;
        self.run.items = run;
        if !capped {
            self.run.hidden = 0;
        }
        self.run.set_pending(beyond_cap);
        Ok((out, ids.into_iter().map(|(id, _)| id).collect()))
    }

    /// Accepted guest commands, oldest first. The app applies them through
    /// its normal playback path, which then broadcasts the new state.
    pub fn take_commands(&mut self) -> Vec<HostCommand> {
        self.pending_cmds.drain(..).collect()
    }

    /// Records a command attempt; `false` when `peer` is over its budget.
    /// The budget outlives `Bye` and reconnects, so leaving and rejoining
    /// does not refill it; idle entries are pruned to bound the map.
    fn within_budget(&mut self, peer: &str, now_ns: i64) -> bool {
        if self.cmd_times.len() > 4 * MAX_GUESTS {
            self.cmd_times.retain(|_, t| t.back().is_some_and(|last| now_ns - last < CMD_WINDOW_NS));
        }
        let times = self.cmd_times.entry(peer.to_owned()).or_default();
        while times.front().is_some_and(|t| now_ns - t >= CMD_WINDOW_NS) {
            times.pop_front();
        }
        if times.len() >= CMD_BURST {
            return false;
        }
        times.push_back(now_ns);
        true
    }

    pub fn on_message(&mut self, peer: &str, msg: Msg, now_ns: i64) -> Vec<Out> {
        match msg {
            // A joined guest repeating Hello gets the same per-guest limit as
            // Resync, so it cannot make the host clone full snapshots in a loop.
            Msg::Hello { .. } if self.guests.contains_key(peer) && self.last_resync_ns.get(peer).is_some_and(|t| now_ns - t < RESYNC_MIN_NS) => Vec::new(),
            Msg::Hello { proto, secret, name } => {
                if proto != PROTO_VERSION {
                    return vec![to(peer, Msg::Reject { reason: RejectReason::IncompatibleVersion, proto: PROTO_VERSION })];
                }
                if secret != self.secret {
                    return vec![to(peer, Msg::Reject { reason: RejectReason::BadSecret, proto: PROTO_VERSION })];
                }
                if !self.guests.contains_key(peer) && self.guests.len() >= MAX_GUESTS {
                    return vec![to(peer, Msg::Reject { reason: RejectReason::Full, proto: PROTO_VERSION })];
                }
                let name: String = name.chars().take(40).collect();
                self.last_resync_ns.insert(peer.to_owned(), now_ns);
                self.guests.insert(peer.to_owned(), GuestInfo { peer: peer.to_owned(), name });
                let mut anchor = self.anchor.clone();
                anchor.position_ms = anchor.position_at(now_ns);
                anchor.host_mono_ns = now_ns;
                vec![to(
                    peer,
                    Msg::Welcome {
                        proto: PROTO_VERSION,
                        session_id: self.session_id.clone(),
                        host_name: self.name.clone(),
                        snapshot: self.queue.snapshot(),
                        anchor,
                        guest_control: self.guest_control,
                        features: self.features(),
                    },
                )]
            }
            _ if !self.guests.contains_key(peer) => Vec::new(), // must say Hello first
            Msg::Ping { id, t0 } => vec![to(peer, Msg::Pong { id, t0, t1: now_ns, t2: now_ns })],
            // At most one answer per guest per RESYNC_MIN_NS: a guest asking
            // for full snapshots in a loop cannot make the host buffer them.
            Msg::Resync { .. } if self.last_resync_ns.get(peer).is_some_and(|t| now_ns - t < RESYNC_MIN_NS) => Vec::new(),
            Msg::Resync { have } => {
                self.last_resync_ns.insert(peer.to_owned(), now_ns);
                match self.queue.ops_since(have).filter(|ops| fits_frame(ops)) {
                Some(ops) => {
                    let mut v = vec![to(peer, Msg::Ops { ops })];
                    v.push(to(peer, Msg::Anchor(self.anchor.clone())));
                    v
                }
                None => vec![to(peer, Msg::Snapshot(self.queue.snapshot())), to(peer, Msg::Anchor(self.anchor.clone()))],
                }
            }
            // Forbidden: tell the guest so a stale UI locks its controls.
            Msg::Command { .. } if !self.guest_control => {
                if self.within_budget(peer, now_ns) { vec![to(peer, Msg::Control { guest_control: false })] } else { Vec::new() }
            }
            Msg::Command { cmd, next } => {
                if self.within_budget(peer, now_ns) && valid_command(&cmd) {
                    // `next` only means something on an enqueue the app can place.
                    let next = next && self.play_next && matches!(cmd, GuestCommand::Enqueue { .. });
                    if self.pending_cmds.len() >= MAX_PENDING_CMDS {
                        self.pending_cmds.pop_front();
                    }
                    let name = self.guests.get(peer).map(|g| g.name.clone()).unwrap_or_default();
                    self.pending_cmds.push_back(HostCommand { peer: peer.to_owned(), name, cmd, next });
                }
                Vec::new()
            }
            Msg::Bye => {
                self.guests.remove(peer);
                self.last_resync_ns.remove(peer);
                Vec::new()
            }
            // Guests cannot send host messages.
            _ => Vec::new(),
        }
    }

    pub fn peer_disconnected(&mut self, peer: &str) {
        self.guests.remove(peer);
        self.last_resync_ns.remove(peer);
    }

    /// Host player state changed (play, pause, seek, new track). Coalesced
    /// within [`DEBOUNCE_NS`]; the rest goes out on [`Self::tick`].
    pub fn set_playback(&mut self, item_id: Option<String>, track_id: Option<String>, position_ms: i64, playing: bool, now_ns: i64) -> Vec<Out> {
        let track_changed = item_id != self.anchor.item_id || track_id != self.anchor.track_id;
        self.anchor = Anchor {
            epoch: self.epoch,
            seq: self.anchor.seq + 1,
            queue_version: self.queue.queue().version,
            item_id,
            track_id,
            position_ms: position_ms.max(0),
            host_mono_ns: now_ns,
            playing,
        };
        self.sync_run();
        if track_changed || now_ns - self.last_anchor_sent_ns >= DEBOUNCE_NS {
            self.flush_anchor(now_ns)
        } else {
            self.pending_anchor = true;
            Vec::new()
        }
    }

    fn flush_anchor(&mut self, now_ns: i64) -> Vec<Out> {
        self.pending_anchor = false;
        self.last_anchor_sent_ns = now_ns;
        vec![all(Msg::Anchor(self.anchor.clone()))]
    }

    /// Changes the shared queue (host only in the MVP). The app's whole
    /// queue from [`Self::replace_queue_entries`] no longer matches after a
    /// direct change, so Play Next falls back to the shared queue (by entry
    /// id) until the next `replace_queue_entries`.
    pub fn queue_op(&mut self, kind: OpKind) -> Result<Vec<Out>, QueueError> {
        let out = self.apply_shared(kind)?;
        self.app_queue = None;
        Ok(out)
    }

    fn apply_shared(&mut self, kind: OpKind) -> Result<Vec<Out>, QueueError> {
        match self.queue.apply(Op { op_id: new_id(), kind })? {
            Accepted::New(v) => Ok(vec![all(Msg::Op(v))]),
            Accepted::Duplicate => Ok(Vec::new()),
        }
    }

    /// Call about every 250 ms: heartbeats and debounced anchors.
    pub fn tick(&mut self, now_ns: i64) -> Vec<Out> {
        let mut out = Vec::new();
        if self.pending_anchor && now_ns - self.last_anchor_sent_ns >= DEBOUNCE_NS {
            out.extend(self.flush_anchor(now_ns));
        }
        if now_ns - self.last_heartbeat_ns >= HEARTBEAT_NS {
            self.last_heartbeat_ns = now_ns;
            out.push(all(Msg::Heartbeat { host_mono_ns: now_ns }));
        }
        out
    }

    pub fn end(&mut self) -> Vec<Out> {
        self.guests.clear();
        self.pending_cmds.clear();
        vec![all(Msg::End { reason: EndReason::HostEnded })]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuestPhase {
    Joining,
    Joined,
    Rejected(RejectReason),
    Ended(EndReason),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GuestEvent {
    Joined { host_name: String },
    Rejected(RejectReason),
    QueueChanged,
    PlaybackChanged,
    /// The host allowed or forbade guest playback commands.
    ControlChanged(bool),
    Ended(EndReason),
}

/// Where the local player should be.
#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub item_id: Option<String>,
    pub track_id: Option<String>,
    pub position_ms: i64,
    pub playing: bool,
}

/// Ping burst on join, then one every few seconds.
pub const BURST_PINGS: u64 = 8;
pub const BURST_INTERVAL_NS: i64 = 150_000_000;
pub const PING_INTERVAL_NS: i64 = 3_000_000_000;
/// A queue gap older than this triggers a resync request.
pub const GAP_TIMEOUT_NS: i64 = 2_000_000_000;
/// Minimum time between two answered `Resync`s from one guest. Below
/// [`GAP_TIMEOUT_NS`], so an honest guest's retry is always answered.
pub const RESYNC_MIN_NS: i64 = 500_000_000;

#[derive(Debug)]
pub struct GuestSession {
    secret: String,
    name: String,
    phase: GuestPhase,
    pub host_name: String,
    clock: ClockSync,
    queue: GuestQueue,
    anchor: Option<Anchor>,
    drift: DriftController,
    last_heard_ns: i64,
    next_ping_id: u64,
    pings_sent: u64,
    last_ping_ns: i64,
    outstanding: BTreeMap<u64, i64>,
    gap_since: Option<i64>,
    resync_sent_ns: Option<i64>,
    /// A snapshot was requested and has not arrived; `tick` retries it
    /// (the host may drop a request under [`RESYNC_MIN_NS`]).
    need_snapshot: bool,
    /// Whether the host currently accepts our playback commands.
    guest_control: bool,
    /// Our recent command times: the same budget as the host's, so honest
    /// commands are never dropped there and the outgoing queue stays small.
    cmd_times: std::collections::VecDeque<i64>,
    /// The host advertised [`FEATURE_PLAY_NEXT`].
    host_play_next: bool,
}

impl GuestSession {
    pub fn new(secret: String, name: String, now_ns: i64) -> Self {
        Self {
            secret,
            name,
            phase: GuestPhase::Joining,
            host_name: String::new(),
            clock: ClockSync::new(),
            queue: GuestQueue::new(),
            anchor: None,
            drift: DriftController::new(),
            last_heard_ns: now_ns,
            next_ping_id: 0,
            pings_sent: 0,
            last_ping_ns: i64::MIN / 2,
            outstanding: BTreeMap::new(),
            gap_since: None,
            resync_sent_ns: None,
            need_snapshot: false,
            guest_control: false,
            cmd_times: std::collections::VecDeque::new(),
            host_play_next: false,
        }
    }

    pub fn phase(&self) -> &GuestPhase {
        &self.phase
    }
    pub fn queue_items(&self) -> &[QueueItem] {
        &self.queue.queue().items
    }
    pub fn anchor(&self) -> Option<&Anchor> {
        self.anchor.as_ref()
    }
    pub fn clock(&self) -> &ClockSync {
        &self.clock
    }
    pub fn set_seek_only(&mut self, on: bool) {
        self.drift.seek_only = on;
    }

    /// Whether the host allows our playback commands (lock the UI otherwise).
    pub fn guest_control(&self) -> bool {
        self.guest_control
    }

    /// The message for a playback request, or `None` when it cannot be sent
    /// (not joined, control not allowed, malformed, or over the budget of
    /// [`CMD_BURST`] per [`CMD_WINDOW_NS`]).
    pub fn command(&mut self, cmd: GuestCommand, now_ns: i64) -> Option<Msg> {
        if self.phase != GuestPhase::Joined || !self.guest_control || !valid_command(&cmd) {
            return None;
        }
        while self.cmd_times.front().is_some_and(|t| now_ns - t >= CMD_WINDOW_NS) {
            self.cmd_times.pop_front();
        }
        if self.cmd_times.len() >= CMD_BURST {
            return None;
        }
        self.cmd_times.push_back(now_ns);
        Some(Msg::Command { cmd, next: false })
    }

    /// Whether the host places guest Play Next after the playing track.
    /// When `false`, [`Self::play_next`] falls back to appending.
    pub fn host_supports_play_next(&self) -> bool {
        self.host_play_next
    }

    /// Play Next request for `track_ids`, with the same checks as
    /// [`Self::command`]. Hosts without [`FEATURE_PLAY_NEXT`] get a plain
    /// enqueue (the tracks are appended).
    pub fn play_next(&mut self, track_ids: Vec<String>, now_ns: i64) -> Option<Msg> {
        let next = self.host_play_next;
        match self.command(GuestCommand::Enqueue { track_ids }, now_ns)? {
            Msg::Command { cmd, .. } => Some(Msg::Command { cmd, next }),
            m => Some(m),
        }
    }

    fn set_control(&mut self, allowed: bool, ev: &mut Vec<GuestEvent>) {
        if self.guest_control != allowed {
            self.guest_control = allowed;
            ev.push(GuestEvent::ControlChanged(allowed));
        }
    }

    pub fn hello(&self) -> Msg {
        Msg::Hello { proto: PROTO_VERSION, secret: self.secret.clone(), name: self.name.clone() }
    }

    /// New connection to the same host (network change, app resumed): the
    /// clock is re-measured and the queue caught up through `Resync`.
    pub fn on_reconnect(&mut self, now_ns: i64) -> Vec<Msg> {
        if matches!(self.phase, GuestPhase::Ended(_) | GuestPhase::Rejected(_)) {
            return Vec::new();
        }
        self.clock.reset();
        self.drift.reset();
        self.pings_sent = 0;
        self.outstanding.clear();
        self.last_heard_ns = now_ns;
        let mut v = vec![self.hello()];
        if self.queue.has_snapshot() {
            v.push(Msg::Resync { have: self.queue.queue().version });
        }
        v
    }

    fn ended(&mut self, r: EndReason) -> Vec<GuestEvent> {
        if matches!(self.phase, GuestPhase::Ended(_)) {
            return Vec::new();
        }
        self.phase = GuestPhase::Ended(r);
        vec![GuestEvent::Ended(r)]
    }

    fn take_anchor(&mut self, a: Anchor) -> bool {
        let newer = self.anchor.as_ref().is_none_or(|cur| a.is_newer_than(cur));
        if newer {
            if self.anchor.as_ref().is_none_or(|cur| cur.item_id != a.item_id || cur.track_id != a.track_id || !a.playing || !cur.playing) {
                self.drift.reset();
            }
            self.anchor = Some(a);
        }
        newer
    }

    /// `before`: queue version before the update, so a partial advance (one
    /// that still leaves a gap) is reported too.
    fn queue_result(&mut self, r: GuestApply, before: u64, now_ns: i64, out: &mut Vec<Msg>, ev: &mut Vec<GuestEvent>) {
        if !matches!(r, GuestApply::Changed) && self.queue.queue().version != before {
            ev.push(GuestEvent::QueueChanged);
        }
        match r {
            GuestApply::Changed => {
                self.gap_since = None;
                ev.push(GuestEvent::QueueChanged);
            }
            GuestApply::Ignored => {}
            GuestApply::Gap { .. } => {
                self.gap_since.get_or_insert(now_ns);
            }
            GuestApply::NeedSnapshot => {
                self.gap_since = None;
                self.need_snapshot = true;
                // Version 0 is never in the log: forces a snapshot.
                out.push(Msg::Resync { have: u64::MAX });
                self.resync_sent_ns = Some(now_ns);
            }
        }
    }

    /// Handles a host message. Returns messages to send and events for the UI.
    pub fn on_message(&mut self, msg: Msg, now_ns: i64) -> (Vec<Msg>, Vec<GuestEvent>) {
        let mut out = Vec::new();
        let mut ev = Vec::new();
        if matches!(self.phase, GuestPhase::Ended(_) | GuestPhase::Rejected(_)) {
            return (out, ev);
        }
        self.last_heard_ns = now_ns;
        if self.phase == GuestPhase::Joining && !matches!(msg, Msg::Welcome { .. } | Msg::Reject { .. }) {
            return (out, ev); // state arrives with the Welcome
        }
        match msg {
            Msg::Welcome { proto, host_name, snapshot, anchor, guest_control, features, .. } => {
                if proto != PROTO_VERSION {
                    self.phase = GuestPhase::Rejected(RejectReason::IncompatibleVersion);
                    ev.push(GuestEvent::Rejected(RejectReason::IncompatibleVersion));
                    return (out, ev);
                }
                let first = self.phase == GuestPhase::Joining;
                self.phase = GuestPhase::Joined;
                self.host_name.clone_from(&host_name);
                // A different host epoch means the host restarted: start over.
                if self.anchor.as_ref().is_some_and(|a| a.epoch != anchor.epoch) {
                    self.queue = GuestQueue::new();
                    self.anchor = None;
                }
                let before = self.queue.queue().version;
                let r = self.queue.apply_snapshot(snapshot);
                self.queue_result(r, before, now_ns, &mut out, &mut ev);
                if self.take_anchor(anchor) {
                    ev.push(GuestEvent::PlaybackChanged);
                }
                self.set_control(guest_control, &mut ev);
                self.host_play_next = features.iter().any(|f| f == FEATURE_PLAY_NEXT);
                if first {
                    ev.insert(0, GuestEvent::Joined { host_name });
                }
            }
            Msg::Reject { reason, .. } => {
                self.phase = GuestPhase::Rejected(reason);
                ev.push(GuestEvent::Rejected(reason));
            }
            Msg::Pong { id, t0, t1, t2 } => {
                if self.outstanding.remove(&id) == Some(t0) {
                    self.clock.add(Sample { t0, t1, t2, t3: now_ns });
                }
            }
            Msg::Anchor(a) => {
                if self.take_anchor(a) {
                    ev.push(GuestEvent::PlaybackChanged);
                }
            }
            Msg::Op(op) => {
                let before = self.queue.queue().version;
                let r = self.queue.apply(op);
                self.queue_result(r, before, now_ns, &mut out, &mut ev);
            }
            Msg::Ops { ops } => {
                let before = self.queue.queue().version;
                let mut last = GuestApply::Ignored;
                for op in ops {
                    last = self.queue.apply(op);
                    if last == GuestApply::NeedSnapshot {
                        break;
                    }
                }
                self.resync_sent_ns = None;
                let changed = self.queue.queue().version != before;
                let r = if changed && matches!(last, GuestApply::Ignored) { GuestApply::Changed } else { last };
                self.queue_result(r, before, now_ns, &mut out, &mut ev);
            }
            Msg::Snapshot(s) => {
                self.resync_sent_ns = None;
                self.need_snapshot = false;
                let before = self.queue.queue().version;
                let r = self.queue.apply_snapshot(s);
                self.queue_result(r, before, now_ns, &mut out, &mut ev);
            }
            Msg::End { reason } => ev.extend(self.ended(reason)),
            Msg::Control { guest_control } => self.set_control(guest_control, &mut ev),
            Msg::Features { features } => self.host_play_next = features.iter().any(|f| f == FEATURE_PLAY_NEXT),
            Msg::Heartbeat { .. } | Msg::Hello { .. } | Msg::Ping { .. } | Msg::Resync { .. } | Msg::Command { .. } | Msg::Bye | Msg::Unknown => {}
        }
        (out, ev)
    }

    /// Call about every 100-250 ms. Sends pings and resyncs, detects a lost host.
    pub fn tick(&mut self, now_ns: i64) -> (Vec<Msg>, Vec<GuestEvent>) {
        let mut out = Vec::new();
        let mut ev = Vec::new();
        match self.phase {
            GuestPhase::Ended(_) | GuestPhase::Rejected(_) => return (out, ev),
            GuestPhase::Joining | GuestPhase::Joined => {}
        }
        if now_ns - self.last_heard_ns > HOST_TIMEOUT_NS {
            ev.extend(self.ended(EndReason::HostLost));
            return (out, ev);
        }
        if self.phase == GuestPhase::Joined {
            let interval = if self.pings_sent < BURST_PINGS { BURST_INTERVAL_NS } else { PING_INTERVAL_NS };
            if now_ns - self.last_ping_ns >= interval {
                let id = self.next_ping_id;
                self.next_ping_id += 1;
                self.pings_sent += 1;
                self.last_ping_ns = now_ns;
                self.outstanding.insert(id, now_ns);
                // Lost pongs must not pile up.
                while self.outstanding.len() > 16 {
                    self.outstanding.pop_first();
                }
                out.push(Msg::Ping { id, t0: now_ns });
            }
            if let Some(since) = self.gap_since
                && now_ns - since > GAP_TIMEOUT_NS
                && self.resync_sent_ns.is_none_or(|t| now_ns - t > GAP_TIMEOUT_NS)
            {
                out.push(Msg::Resync { have: self.queue.queue().version });
                self.resync_sent_ns = Some(now_ns);
            } else if self.need_snapshot && self.resync_sent_ns.is_none_or(|t| now_ns - t > GAP_TIMEOUT_NS) {
                out.push(Msg::Resync { have: u64::MAX });
                self.resync_sent_ns = Some(now_ns);
            }
        }
        (out, ev)
    }

    /// Where the player should be now (`latency_ms`: audio output latency
    /// plus the user's manual offset). `None` until the clock is calibrated.
    pub fn target(&self, now_ns: i64, latency_ms: i64) -> Option<Target> {
        let a = self.anchor.as_ref()?;
        let host_now = self.clock.to_host(now_ns)?;
        let pos = a.position_at(host_now).saturating_add(if a.playing { latency_ms } else { 0 });
        Some(Target { item_id: a.item_id.clone(), track_id: a.track_id.clone(), position_ms: pos.max(0), playing: a.playing })
    }

    /// Drift correction for a player at `actual_ms` playing the target item.
    pub fn correction(&mut self, now_ns: i64, latency_ms: i64, actual_ms: i64) -> Correction {
        match self.target(now_ns, latency_ms) {
            Some(t) if t.playing => self.drift.decide(t.position_ms, actual_ms, now_ns),
            _ => Correction::None,
        }
    }

    pub fn leave(&mut self) -> Vec<Msg> {
        self.ended(EndReason::Left);
        vec![Msg::Bye]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: i64 = 1_000_000;
    const S: i64 = 1_000 * MS;

    #[test]
    fn resync_spam_is_rate_limited() {
        let mut h = HostSession::new("Host".into(), 0);
        let secret = h.secret.clone();
        h.on_message("g", Msg::Hello { proto: PROTO_VERSION, secret, name: "G".into() }, 0);
        h.queue_op(OpKind::Replace { items: items(3) }).unwrap();
        let snaps = |o: &[Out]| o.iter().filter(|o| matches!(o.msg, Msg::Snapshot(_))).count();
        let mut answered = 0;
        // 100 requests within 100 ms: only the first is answered.
        for i in 0..100 {
            answered += snaps(&h.on_message("g", Msg::Resync { have: u64::MAX }, S + i * MS));
        }
        assert_eq!(answered, 1);
        // A retry after the guest's own gap timeout is answered.
        assert_eq!(snaps(&h.on_message("g", Msg::Resync { have: u64::MAX }, S + GAP_TIMEOUT_NS)), 1);
    }

    #[test]
    fn hello_spam_from_joined_guest_is_rate_limited() {
        let mut h = HostSession::new("Host".into(), 0);
        let secret = h.secret.clone();
        let hello = || Msg::Hello { proto: PROTO_VERSION, secret: secret.clone(), name: "G".into() };
        let welcomes = |o: &[Out]| o.iter().filter(|o| matches!(o.msg, Msg::Welcome { .. })).count();
        let mut answered = 0;
        for i in 0..100 {
            answered += welcomes(&h.on_message("g", hello(), i * MS));
        }
        assert_eq!(answered, 1);
        assert_eq!(welcomes(&h.on_message("g", hello(), RESYNC_MIN_NS)), 1, "a reconnect after the limit is welcomed");
    }

    fn items(n: usize) -> Vec<QueueItem> {
        (0..n).map(|i| QueueItem { item_id: format!("i{i}"), track_id: format!("t{i}") }).collect()
    }

    /// In-memory link: guest clock = host clock - `skew`, one-way delay `lat`.
    struct World {
        host: HostSession,
        guest: GuestSession,
        skew: i64,
        lat: i64,
        events: Vec<GuestEvent>,
    }

    impl World {
        fn new(skew: i64, lat: i64) -> Self {
            let host = HostSession::new("Host".into(), 100 * S);
            let guest = GuestSession::new(host.secret.clone(), "Guest".into(), 100 * S - skew);
            Self { host, guest, skew, lat, events: Vec::new() }
        }
        /// Delivers guest messages sent at host time `t` and the replies.
        fn send_guest(&mut self, msgs: Vec<Msg>, t: i64) {
            for m in msgs {
                let replies = self.host.on_message("g", m, t + self.lat);
                self.deliver(replies, t + self.lat);
            }
        }
        fn deliver(&mut self, outs: Vec<Out>, t: i64) {
            for o in outs {
                let (back, ev) = self.guest.on_message(o.msg, t + self.lat - self.skew);
                self.events.extend(ev);
                self.send_guest(back, t + self.lat);
            }
        }
        fn guest_tick(&mut self, t: i64) {
            let (m, ev) = self.guest.tick(t - self.skew);
            self.events.extend(ev);
            self.send_guest(m, t);
        }
        fn join(&mut self, t: i64) {
            let h = self.guest.hello();
            self.send_guest(vec![h], t);
        }
    }

    #[test]
    fn dropped_snapshot_request_is_retried() {
        use crate::live::queue::Snapshot;
        let mut w = World::new(0, MS);
        w.join(100 * S);
        let resyncs = |m: &[Msg]| m.iter().filter(|m| matches!(m, Msg::Resync { have: u64::MAX })).count();
        // An unusable snapshot makes the guest ask for a new one; the host
        // drops that request (rate limit), so nothing comes back.
        let t = 101 * S;
        let (out, _) = w.guest.on_message(Msg::Snapshot(Snapshot { version: u64::MAX, items: vec![] }), t);
        assert_eq!(resyncs(&out), 1);
        let (out, _) = w.guest.tick(t + GAP_TIMEOUT_NS / 2);
        assert_eq!(resyncs(&out), 0, "no retry before the timeout");
        let (out, _) = w.guest.tick(t + GAP_TIMEOUT_NS + MS);
        assert_eq!(resyncs(&out), 1, "retried after the timeout");
        // A good snapshot stops the retries.
        let good = w.host.queue.snapshot();
        w.guest.on_message(Msg::Snapshot(good), t + GAP_TIMEOUT_NS + 2 * MS);
        let (out, _) = w.guest.tick(t + 3 * GAP_TIMEOUT_NS);
        assert_eq!(resyncs(&out), 0);
    }

    #[test]
    fn join_snapshot_and_clock_then_target_matches_host() {
        let mut w = World::new(3_600 * S, 25 * MS);
        w.host.queue_op(OpKind::Replace { items: items(5) }).unwrap();
        w.host.set_playback(Some("i1".into()), Some("t1".into()), 10_000, true, 100 * S);
        w.join(101 * S);
        assert!(matches!(w.events[0], GuestEvent::Joined { .. }));
        assert_eq!(w.guest.queue_items(), w.host.queue_items());
        let mut t = 101 * S;
        for _ in 0..10 {
            t += 200 * MS;
            w.guest_tick(t);
        }
        let off = w.guest.clock().offset().expect("calibrated");
        assert!((off - 3_600 * S).abs() < MS, "offset {off}");
        let target = w.guest.target(t - w.skew, 0).unwrap();
        let host_pos = w.host.anchor().position_at(t);
        assert!((target.position_ms - host_pos).abs() <= 1, "{} vs {host_pos}", target.position_ms);
        assert_eq!(target.item_id.as_deref(), Some("i1"));
        // Output latency moves the target ahead.
        assert_eq!(w.guest.target(t - w.skew, 200).unwrap().position_ms, target.position_ms + 200);
    }

    #[test]
    fn rejects_wrong_secret_and_version_and_full() {
        let mut h = HostSession::new("H".into(), 0);
        let r = h.on_message("x", Msg::Hello { proto: 1, secret: "nope".into(), name: "x".into() }, 0);
        assert_eq!(r[0].msg, Msg::Reject { reason: RejectReason::BadSecret, proto: PROTO_VERSION });
        let r = h.on_message("x", Msg::Hello { proto: 2, secret: h.secret.clone(), name: "x".into() }, 0);
        assert_eq!(r[0].msg, Msg::Reject { reason: RejectReason::IncompatibleVersion, proto: PROTO_VERSION });
        for i in 0..MAX_GUESTS {
            h.on_message(&format!("p{i}"), Msg::Hello { proto: 1, secret: h.secret.clone(), name: "x".into() }, 0);
        }
        let r = h.on_message("late", Msg::Hello { proto: 1, secret: h.secret.clone(), name: "x".into() }, 0);
        assert_eq!(r[0].msg, Msg::Reject { reason: RejectReason::Full, proto: PROTO_VERSION });
        // Unknown peers cannot ping or resync.
        assert!(h.on_message("stranger", Msg::Ping { id: 0, t0: 0 }, 0).is_empty());
    }

    #[test]
    fn stale_or_duplicate_anchor_ignored() {
        let mut w = World::new(0, MS);
        w.join(0);
        let a1 = w.host.set_playback(Some("i".into()), None, 0, true, S);
        let a2 = w.host.set_playback(Some("j".into()), None, 0, true, 2 * S);
        w.deliver(a2.clone(), 2 * S);
        w.events.clear();
        w.deliver(a1, 2 * S);
        w.deliver(a2, 2 * S);
        assert!(w.events.is_empty());
        assert_eq!(w.guest.anchor().unwrap().item_id.as_deref(), Some("j"));
    }

    #[test]
    fn seek_spam_is_debounced() {
        let mut h = HostSession::new("H".into(), 0);
        assert_eq!(h.set_playback(Some("i".into()), None, 0, true, S).len(), 1);
        assert!(h.set_playback(Some("i".into()), None, 5_000, true, S + 50 * MS).is_empty());
        assert!(h.set_playback(Some("i".into()), None, 9_000, true, S + 100 * MS).is_empty());
        let out = h.tick(S + 300 * MS);
        let Msg::Anchor(a) = &out[0].msg else { panic!("{out:?}") };
        assert_eq!(a.position_ms, 9_000, "only the last seek goes out");
        // A track change is never delayed.
        assert_eq!(h.set_playback(Some("k".into()), None, 0, true, S + 310 * MS).len(), 1);
    }

    #[test]
    fn track_change_without_item_id_is_not_debounced() {
        let mut h = HostSession::new("H".into(), 0);
        assert_eq!(h.set_playback(None, Some("t1".into()), 0, true, S).len(), 1);
        assert_eq!(h.set_playback(None, Some("t2".into()), 0, true, S + MS).len(), 1);
    }

    #[test]
    fn ops_batch_leaving_a_gap_keeps_resync_timer() {
        use super::super::queue::{Op, VersionedOp};
        let mut w = World::new(0, MS);
        w.host.queue_op(OpKind::Replace { items: items(3) }).unwrap();
        w.join(0);
        let v = w.guest.queue_items().len();
        assert_eq!(v, 3);
        let have = 1; // snapshot version after one Replace
        let mk = |ver: u64, id: &str| VersionedOp { version: ver, op: Op { op_id: id.into(), kind: OpKind::Remove { item_id: format!("i{}", ver - 2) } } };
        let (_, ev) = w.guest.on_message(Msg::Ops { ops: vec![mk(have + 1, "a"), mk(have + 3, "c")] }, S);
        assert!(ev.contains(&GuestEvent::QueueChanged), "{ev:?}");
        let (out, _) = w.guest.tick(S + GAP_TIMEOUT_NS + 10 * MS);
        assert!(out.iter().any(|m| matches!(m, Msg::Resync { .. })), "{out:?}");
    }

    #[test]
    fn single_op_partial_advance_reports_change() {
        use super::super::queue::{Op, VersionedOp};
        let mut w = World::new(0, MS);
        w.host.queue_op(OpKind::Replace { items: items(4) }).unwrap();
        w.join(0);
        let mk = |ver: u64| VersionedOp { version: ver, op: Op { op_id: format!("o{ver}"), kind: OpKind::Remove { item_id: format!("i{}", ver - 2) } } };
        let (_, ev) = w.guest.on_message(Msg::Op(mk(4)), S);
        assert!(!ev.contains(&GuestEvent::QueueChanged));
        let (_, ev) = w.guest.on_message(Msg::Op(mk(2)), S);
        assert!(ev.contains(&GuestEvent::QueueChanged), "{ev:?}");
    }

    #[test]
    fn oversized_catch_up_falls_back_to_snapshot() {
        let mut w = World::new(0, MS);
        w.join(0);
        let big = |p: &str| (0..500).map(|i| QueueItem { item_id: format!("{p}{i:031}"), track_id: format!("{i:09}") }).collect();
        w.host.queue_op(OpKind::Replace { items: big("a") }).unwrap();
        w.host.queue_op(OpKind::Replace { items: big("b") }).unwrap();
        let out = w.host.on_message("g", Msg::Resync { have: 0 }, S);
        assert!(matches!(out[0].msg, Msg::Snapshot(_)), "{:?}", out[0].msg);
        assert!(out.iter().all(|o| super::super::proto::encode(&o.msg).is_ok()));
    }

    #[test]
    fn lost_ops_recovered_by_resync() {
        let mut w = World::new(0, MS);
        w.host.queue_op(OpKind::Replace { items: items(3) }).unwrap();
        w.join(0);
        let lost = w.host.queue_op(OpKind::Remove { item_id: "i0".into() }).unwrap();
        drop(lost);
        let o = w.host.queue_op(OpKind::Clear).unwrap();
        w.deliver(o, S);
        assert_ne!(w.guest.queue_items(), w.host.queue_items());
        w.guest_tick(S + GAP_TIMEOUT_NS + 10 * MS);
        assert_eq!(w.guest.queue_items(), w.host.queue_items());
    }

    #[test]
    fn reconnect_catches_up_and_recalibrates() {
        let mut w = World::new(0, MS);
        w.host.queue_op(OpKind::Replace { items: items(3) }).unwrap();
        w.join(0);
        for i in 1..12 {
            w.guest_tick(i * 200 * MS);
        }
        assert!(w.guest.clock().offset().is_some());
        // Connection drops; host keeps going.
        w.host.peer_disconnected("g");
        w.host.queue_op(OpKind::Remove { item_id: "i1".into() }).unwrap();
        w.host.set_playback(Some("i2".into()), Some("t2".into()), 0, true, 5 * S);
        let msgs = w.guest.on_reconnect(6 * S);
        assert!(w.guest.clock().offset().is_none(), "clock re-measured");
        w.send_guest(msgs, 6 * S);
        assert_eq!(w.guest.queue_items(), w.host.queue_items());
        assert_eq!(w.guest.anchor().unwrap().item_id.as_deref(), Some("i2"));
        assert_eq!(*w.guest.phase(), GuestPhase::Joined);
    }

    #[test]
    fn host_restart_new_epoch_replaces_state() {
        let mut w = World::new(0, MS);
        w.host.set_playback(Some("old".into()), None, 0, true, S);
        w.join(S);
        let secret = w.host.secret.clone();
        w.host = HostSession::new("Host".into(), 50 * S);
        w.host.secret = secret;
        w.host.set_playback(Some("new".into()), None, 0, true, 51 * S);
        let msgs = w.guest.on_reconnect(52 * S);
        w.send_guest(msgs, 52 * S);
        assert_eq!(w.guest.anchor().unwrap().item_id.as_deref(), Some("new"));
    }

    #[test]
    fn host_end_and_host_lost() {
        let mut w = World::new(0, MS);
        w.join(0);
        let o = w.host.end();
        w.deliver(o, S);
        assert_eq!(*w.guest.phase(), GuestPhase::Ended(EndReason::HostEnded));
        assert!(w.events.contains(&GuestEvent::Ended(EndReason::HostEnded)));

        let mut w = World::new(0, MS);
        w.join(0);
        w.guest_tick(HOST_TIMEOUT_NS - S);
        assert_eq!(*w.guest.phase(), GuestPhase::Joined);
        // Heartbeats keep it alive.
        let hb = w.host.tick(HOST_TIMEOUT_NS);
        w.deliver(hb, HOST_TIMEOUT_NS);
        w.guest_tick(HOST_TIMEOUT_NS + 5 * S);
        assert_eq!(*w.guest.phase(), GuestPhase::Joined);
        // Silence: ended once, with a clear reason.
        let (_, ev) = w.guest.tick(3 * HOST_TIMEOUT_NS);
        assert_eq!(ev, vec![GuestEvent::Ended(EndReason::HostLost)]);
        let (_, ev) = w.guest.tick(4 * HOST_TIMEOUT_NS);
        assert!(ev.is_empty());
        assert!(w.guest.on_reconnect(4 * HOST_TIMEOUT_NS).is_empty());
    }

    #[test]
    fn guest_commands_need_permission() {
        let mut w = World::new(0, MS);
        w.join(0);
        assert!(!w.guest.guest_control());
        assert_eq!(w.guest.command(GuestCommand::Pause, S), None, "locked by default");
        // A guest sending anyway is ignored and told it is locked.
        let r = w.host.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: false }, S);
        assert_eq!(r, vec![to("g", Msg::Control { guest_control: false })]);
        assert!(w.host.take_commands().is_empty());
        // The host allows it: guests hear about it.
        let o = w.host.set_guest_control(true);
        w.deliver(o, 2 * S);
        assert!(w.events.contains(&GuestEvent::ControlChanged(true)));
        let m = w.guest.command(GuestCommand::Enqueue { track_ids: vec!["123".into()] }, 3 * S).unwrap();
        w.send_guest(vec![m], 3 * S);
        let cmds = w.host.take_commands();
        assert_eq!(cmds, vec![HostCommand { peer: "g".into(), name: "Guest".into(), cmd: GuestCommand::Enqueue { track_ids: vec!["123".into()] }, next: false }]);
        // Forbidding again drops what the app has not taken.
        w.host.on_message("g", Msg::Command { cmd: GuestCommand::Play, next: false }, 4 * S);
        let o = w.host.set_guest_control(false);
        assert!(w.host.take_commands().is_empty());
        w.deliver(o, 5 * S);
        assert!(!w.guest.guest_control());
        assert!(w.host.set_guest_control(false).is_empty(), "no change, no broadcast");
    }

    fn ids(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn play_next_needs_permission() {
        let mut w = World::new(0, MS);
        w.host.set_play_next(true);
        w.join(0);
        assert!(w.guest.host_supports_play_next());
        assert_eq!(w.guest.play_next(ids(&["1"]), S), None, "locked by default");
        let r = w.host.on_message("g", Msg::Command { cmd: GuestCommand::Enqueue { track_ids: ids(&["1"]) }, next: true }, S);
        assert_eq!(r, vec![to("g", Msg::Control { guest_control: false })]);
        assert!(w.host.take_commands().is_empty());
        // Malformed play-next is dropped like a malformed enqueue.
        w.host.set_guest_control(true);
        w.host.on_message("g", Msg::Command { cmd: GuestCommand::Enqueue { track_ids: ids(&["x"]) }, next: true }, 2 * S);
        assert!(w.host.take_commands().is_empty());
    }

    #[test]
    fn play_next_reaches_host_in_order() {
        let mut w = World::new(0, MS);
        w.host.set_play_next(true);
        w.host.set_guest_control(true);
        w.join(0);
        for (i, t) in ["1", "2", "3"].iter().enumerate() {
            let m = w.guest.play_next(ids(&[t]), S + i as i64 * MS).expect("sent");
            assert_eq!(m, Msg::Command { cmd: GuestCommand::Enqueue { track_ids: ids(&[t]) }, next: true });
            w.send_guest(vec![m], S + i as i64 * MS);
        }
        let cmds = w.host.take_commands();
        assert!(cmds.iter().all(|c| c.next));
        assert_eq!(cmds.iter().map(|c| c.cmd.clone()).collect::<Vec<_>>(), ["1", "2", "3"].map(|t| GuestCommand::Enqueue { track_ids: ids(&[t]) }));
    }

    /// Host with queue `q` playing entry `playing`; returns the entry ids.
    fn host_with(q: &[&str], playing: usize) -> (HostSession, Vec<String>) {
        let mut h = HostSession::new("H".into(), 0);
        let (_, e) = h.replace_queue(ids(q)).expect("replace");
        h.set_playback(Some(e[playing].clone()), Some(q[playing].into()), 0, true, 0);
        (h, e)
    }

    /// What a host app does: insert at the offset after `playing`, then sync.
    fn insert_next(h: &mut HostSession, q: &mut Vec<String>, playing: usize, req: &[&str]) -> Vec<String> {
        let at = (playing + 1 + h.play_next_offset(&ids(req))).min(q.len());
        q.splice(at..at, ids(req));
        h.replace_queue(q.clone()).expect("replace").1
    }

    #[test]
    fn replace_keeps_entry_ids() {
        let (mut h, e) = host_with(&["a", "b", "a", "c"], 2);
        let (_, e2) = h.replace_queue(ids(&["a", "b", "x", "a", "c"])).expect("replace");
        assert_eq!((&e2[0], &e2[1], &e2[3], &e2[4]), (&e[0], &e[1], &e[2], &e[3]));
        assert!(!e.contains(&e2[2]), "new entry, new id");
    }

    #[test]
    fn play_next_offsets_keep_request_order() {
        let (mut h, _) = host_with(&["1", "2", "3"], 0);
        let mut q = ids(&["1", "2", "3"]);
        for req in [&["x1", "x2"][..], &["y"], &["z"]] {
            insert_next(&mut h, &mut q, 0, req);
        }
        assert_eq!(q, ids(&["1", "x1", "x2", "y", "z", "2", "3"]));
        // Several requests applied before one sync keep their order too.
        let a = h.play_next_offset(&ids(&["p"]));
        let b = h.play_next_offset(&ids(&["r"]));
        assert_eq!((a, b), (4, 5));
    }

    #[test]
    fn play_next_before_another_copy_of_the_same_track() {
        // [playing, x] + Play Next x -> [playing, x(new), x(old)]; the next
        // request still goes after the inserted x.
        let (mut h, _) = host_with(&["p", "x"], 0);
        let mut q = ids(&["p", "x"]);
        insert_next(&mut h, &mut q, 0, &["x"]);
        insert_next(&mut h, &mut q, 0, &["y"]);
        assert_eq!(q, ids(&["p", "x", "y", "x"]));
    }

    #[test]
    fn multi_track_play_next_before_another_copy() {
        let (mut h, _) = host_with(&["p", "x"], 0);
        let mut q = ids(&["p", "x"]);
        insert_next(&mut h, &mut q, 0, &["x", "y"]);
        insert_next(&mut h, &mut q, 0, &["z"]);
        assert_eq!(q, ids(&["p", "x", "y", "z", "x"]));
    }

    #[test]
    fn play_next_keeps_order_with_nothing_playing() {
        let (mut h, _) = host_with(&["a", "b"], 0);
        h.set_playback(None, None, 0, false, 0);
        let mut q = ids(&["a", "b"]);
        for req in [&["x"][..], &["y"]] {
            let at = h.play_next_offset(&ids(req));
            q.splice(at..at, ids(req));
            h.replace_queue(q.clone()).expect("replace");
        }
        assert_eq!(q, ids(&["x", "y", "a", "b"]));
    }

    #[test]
    fn play_next_tracked_when_the_cap_hides_the_old_copy() {
        let mut full: Vec<&str> = vec!["p", "x", "x"];
        full.resize(super::super::MAX_QUEUE, "f");
        let (mut h, _) = host_with(&full, 0);
        let mut q = ids(&full);
        insert_next(&mut h, &mut q, 0, &["x"]);
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
    }

    #[test]
    fn play_next_past_the_cap_keeps_order() {
        let full: Vec<String> = (0..super::super::MAX_QUEUE).map(|n| format!("t{n}")).collect();
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        let last = full.len() - 1;
        let (mut h, _) = host_with(&refs, last);
        let mut q = full.clone();
        insert_next(&mut h, &mut q, last, &["x"]);
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
    }

    fn entries(q: &[(&str, &str)]) -> Vec<QueueItem> {
        q.iter().map(|(i, t)| QueueItem { item_id: (*i).into(), track_id: (*t).into() }).collect()
    }

    /// Exact path: a host with its own entry ids, playing `playing`.
    fn exact_host(q: &[(&str, &str)], playing: &str) -> HostSession {
        let mut h = HostSession::new("H".into(), 0);
        h.replace_queue_entries(entries(q)).expect("replace");
        let track = q.iter().find(|(i, _)| *i == playing).map(|(_, t)| (*t).to_owned());
        h.set_playback(Some(playing.into()), track, 0, true, 0);
        h
    }

    #[test]
    fn exact_ids_tell_a_hidden_insert_from_a_removed_one_at_the_cap() {
        // Macroscope: 498 fillers, playing p, ordinary x; Play Next x pushes
        // the ordinary x past the shared cap.
        let fill: Vec<String> = (0..498).map(|n| format!("f{n}")).collect();
        let mut q: Vec<(&str, &str)> = fill.iter().map(|f| (f.as_str(), "f")).collect();
        q.extend([("p", "p"), ("x0", "x")]);
        let mut h = exact_host(&q, "p");
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        let mut with_insert = q.clone();
        with_insert.insert(499, ("n1", "x"));
        h.replace_queue_entries(entries(&with_insert)).expect("replace");
        assert_eq!(h.queue.queue().items.len(), super::super::MAX_QUEUE);
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
        h.run.pending.clear();
        // The host removes the inserted copy: the ordinary x is not a run entry.
        h.replace_queue_entries(entries(&q)).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["z"])), 0);
    }

    #[test]
    fn exact_ids_ignore_other_copies_and_keep_order() {
        let q = [("a", "p"), ("b", "x"), ("c", "x")];
        let mut h = exact_host(&q, "a");
        // Qodo 1: an existing copy of the requested track is never the run.
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        h.replace_queue_entries(entries(&q)).expect("replace");
        assert!(h.run.items.is_empty());
        h.replace_queue_entries(entries(&[("a", "p"), ("n1", "x"), ("b", "x"), ("c", "x")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
        h.replace_queue_entries(entries(&[("a", "p"), ("n1", "x"), ("n2", "y"), ("b", "x"), ("c", "x")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["z"])), 2);
        h.run.pending.clear();
        // Removing the inserted x keeps only y in the run.
        h.replace_queue_entries(entries(&[("a", "p"), ("n2", "y"), ("b", "x"), ("c", "x")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["w"])), 1);
    }

    #[test]
    fn exact_ids_restart_on_another_copy_of_the_playing_track() {
        // Qodo 2: playing another copy of the same track is another entry.
        let q = [("a", "p"), ("b", "p")];
        let mut h = exact_host(&q, "a");
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        h.replace_queue_entries(entries(&[("a", "p"), ("n1", "x"), ("b", "p")])).expect("replace");
        h.set_playback(Some("b".into()), Some("p".into()), 0, true, 0);
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
    }

    #[test]
    fn exact_ids_nothing_playing_and_failed_update() {
        // Qodo 3: nothing playing inserts at the front, in order.
        let mut h = HostSession::new("H".into(), 0);
        h.replace_queue_entries(entries(&[("a", "a")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        // A rejected update changes nothing.
        let bad = h.replace_queue_entries(entries(&[("n1", "x"), ("n1", "a")])).err();
        assert_eq!(bad, Some(QueueError::DuplicateItem("n1".into())));
        h.replace_queue_entries(entries(&[("n1", "x"), ("a", "a")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
    }

    #[test]
    fn direct_queue_ops_invalidate_the_app_queue() {
        let mut h = exact_host(&[("a", "p"), ("b", "x")], "a");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
        h.replace_queue_entries(entries(&[("a", "p"), ("n1", "y"), ("b", "x")])).expect("replace");
        h.queue_op(OpKind::Remove { item_id: "n1".into() }).expect("remove");
        assert_eq!(h.play_next_offset(&ids(&["z"])), 0);
    }

    #[test]
    fn exact_partial_updates_keep_later_requests_pending() {
        let mut h = exact_host(&[("a", "p"), ("b", "q")], "a");
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
        h.replace_queue_entries(entries(&[("a", "p"), ("n1", "x"), ("b", "q")])).expect("replace");
        h.replace_queue_entries(entries(&[("a", "p"), ("n1", "x"), ("n2", "y"), ("b", "q")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["z"])), 2);
    }

    #[test]
    fn moved_away_play_next_does_not_rejoin_the_run() {
        let (mut h, _) = host_with(&["p", "a", "b"], 0);
        let mut q = ids(&["p", "a", "b"]);
        insert_next(&mut h, &mut q, 0, &["x"]);
        h.replace_queue(ids(&["p", "a", "b", "x"])).expect("move away");
        h.replace_queue(ids(&["p", "x", "a", "b"])).expect("move back");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
    }

    #[test]
    fn playing_duplicate_keeps_its_identity() {
        let (mut h, e) = host_with(&["a", "a"], 1);
        let (_, kept) = h.replace_queue(ids(&["a"])).expect("replace");
        assert_eq!(kept, vec![e[1].clone()]);
    }

    #[test]
    fn playing_entry_outweighs_run_tie_breaks() {
        let (mut h, e) = host_with(&["a", "b", "c", "d", "a", "b", "c", "d"], 4);
        h.run.items = e[5..].to_vec();
        let (_, kept) = h.replace_queue(ids(&["a", "b", "c", "d"])).expect("replace");
        assert!(kept.contains(&e[4]));
    }

    #[test]
    fn exact_updates_retire_overflow() {
        let mut h = exact_host(&[("a", "p")], "a");
        h.run.hidden = 10;
        h.replace_queue_entries(entries(&[("a", "p")])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
    }

    #[test]
    fn unchanged_capped_refresh_does_not_bind_an_existing_copy() {
        let mut full: Vec<String> = vec!["p".into(), "x".into()];
        full.extend((0..super::super::MAX_QUEUE - 2).map(|n| format!("o{n}")));
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        let (mut h, _) = host_with(&refs, 0);
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        h.replace_queue(full.clone()).expect("unchanged");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
    }

    #[test]
    fn multi_track_play_next_at_the_cap_keeps_order() {
        let mut full: Vec<String> = (0..497).map(|n| format!("f{n}")).collect();
        full.extend(["p", "x", "x"].map(String::from));
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        let (mut h, _) = host_with(&refs, 497);
        let mut q = full.clone();
        insert_next(&mut h, &mut q, 497, &["x", "y"]);
        assert_eq!(h.play_next_offset(&ids(&["z"])), 2);
    }

    #[test]
    fn batch_at_the_cap_shifted_by_earlier_tracks_keeps_order() {
        let mut full: Vec<String> = (0..495).map(|n| format!("f{n}")).collect();
        full.extend(["p", "a", "x", "q", "r"].map(String::from));
        let refs: Vec<&str> = full.iter().map(String::as_str).collect();
        let (mut h, _) = host_with(&refs, 495);
        let mut q = full.clone();
        insert_next(&mut h, &mut q, 495, &["n", "x", "y"]);
        assert_eq!(h.play_next_offset(&ids(&["z"])), 3);
    }

    #[test]
    fn pending_requests_stay_bounded() {
        let (mut h, _) = host_with(&["p"], 0);
        let n = MAX_APP_QUEUE + 10;
        for k in 0..n {
            assert_eq!(h.play_next_offset(&ids(&["x"])), k);
        }
        assert_eq!(h.run.pending.len(), MAX_APP_QUEUE);
        assert_eq!(h.play_next_offset(&ids(&["y"])), n);
    }

    #[test]
    fn inferring_path_still_works_after_exact_updates() {
        let mut h = exact_host(&[("a", "p"), ("b", "x")], "a");
        let (_, e) = h.replace_queue(ids(&["p", "x"])).expect("replace");
        assert_eq!(e, ids(&["a", "b"]));
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
    }

    #[test]
    fn pending_play_next_is_dropped_without_a_playing_entry() {
        let (mut h, _) = host_with(&["a", "b"], 0);
        h.set_playback(None, None, 0, false, 0);
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        h.replace_queue(ids(&["x", "a", "b"])).expect("replace");
        h.replace_queue(ids(&["a", "b"])).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
    }

    #[test]
    fn failed_replace_keeps_the_pending_play_next() {
        let (mut h, _) = host_with(&["p", "b"], 0);
        let mut q = ids(&["p", "b"]);
        assert_eq!(h.play_next_offset(&ids(&["x"])), 0);
        let failed = h.replace_queue(ids(&["p", "x", ""])).err();
        assert!(failed.is_some());
        q.insert(1, "x".into());
        h.replace_queue(q.clone()).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 1);
    }

    #[test]
    fn play_next_ignores_another_copy_of_a_removed_track() {
        // Qodo 1: [playing, inserted X, existing X] -> remove the inserted X.
        let (mut h, _) = host_with(&["p", "x"], 0);
        let mut q = ids(&["p", "x"]);
        insert_next(&mut h, &mut q, 0, &["x"]);
        assert_eq!(q, ids(&["p", "x", "x"]));
        q.remove(1);
        h.replace_queue(q.clone()).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0, "right after the playing entry, not after the old copy");
        // Removing through a non-refresh op is exact.
        let (mut h, _) = host_with(&["p", "x"], 0);
        let mut q = ids(&["p", "x"]);
        let e = insert_next(&mut h, &mut q, 0, &["x"]);
        h.queue_op(OpKind::Remove { item_id: e[1].clone() }).expect("remove");
        assert_eq!(h.play_next_offset(&ids(&["y"])), 0);
    }

    #[test]
    fn play_next_restarts_on_another_copy_of_the_same_track() {
        // Qodo 2: [a, x, a]: run after the first a, then the second a plays.
        let (mut h, _) = host_with(&["a", "x", "a"], 0);
        let mut q = ids(&["a", "x", "a"]);
        let e = insert_next(&mut h, &mut q, 0, &["n"]);
        assert_eq!(q, ids(&["a", "n", "x", "a"]));
        h.set_playback(Some(e[3].clone()), Some("a".into()), 0, true, S);
        assert_eq!(h.play_next_offset(&ids(&["m"])), 0, "fresh run");
        // A plain refresh does not restart the run (ids survive).
        let (mut h, _) = host_with(&["a", "b"], 0);
        let mut q = ids(&["a", "b"]);
        insert_next(&mut h, &mut q, 0, &["n"]);
        h.replace_queue(q.clone()).expect("replace");
        assert_eq!(h.play_next_offset(&ids(&["m"])), 1);
    }

    #[test]
    fn play_next_keeps_order_when_the_playing_entry_is_unknown() {
        // Qodo 3: nothing playing (or playing entry past the queue cap).
        let mut h = HostSession::new("H".into(), 0);
        h.replace_queue(ids(&["a", "b"])).expect("replace");
        let offs: Vec<usize> = (0..3).map(|i| h.play_next_offset(&ids(&[&i.to_string()]))).collect();
        assert_eq!(offs, vec![0, 1, 2]);
    }

    #[test]
    fn play_next_keeps_the_playing_copy_of_a_repeated_track() {
        let (mut h, _) = host_with(&["a", "b", "a", "c"], 2);
        let mut q = ids(&["a", "b", "a", "c"]);
        h.replace_queue(q.clone()).expect("plain refresh first");
        insert_next(&mut h, &mut q, 2, &["x"]);
        insert_next(&mut h, &mut q, 2, &["y"]);
        assert_eq!(q, ids(&["a", "b", "a", "x", "y", "c"]));
    }

    #[test]
    fn joined_guests_learn_play_next_changes() {
        // Qodo 5: the capability changes after the guest joined.
        let mut w = World::new(0, MS);
        w.host.set_guest_control(true);
        w.join(0);
        assert!(!w.guest.host_supports_play_next());
        let o = w.host.set_play_next(true);
        w.deliver(o, S);
        assert!(w.guest.host_supports_play_next());
        assert!(w.host.set_play_next(true).is_empty(), "no change, no broadcast");
        let o = w.host.set_play_next(false);
        w.deliver(o, 2 * S);
        assert!(!w.guest.host_supports_play_next());
        // Older guests ignore the new message.
        assert_eq!(super::super::proto::decode(br#"{"t":"features","features":["play_next"]}"#).expect("decode"), Msg::Features { features: ids(&["play_next"]) });
    }

    #[test]
    fn play_next_falls_back_to_append() {
        // A host that does not advertise the feature (older host, or an app
        // that has not opted in): the guest sends a plain enqueue.
        let mut w = World::new(0, MS);
        w.host.set_guest_control(true);
        w.join(0);
        assert!(!w.guest.host_supports_play_next());
        let m = w.guest.play_next(ids(&["1"]), S).expect("sent");
        assert_eq!(m, Msg::Command { cmd: GuestCommand::Enqueue { track_ids: ids(&["1"]) }, next: false });
        // And a host app that has not opted in never sees `next`.
        w.host.on_message("g", Msg::Command { cmd: GuestCommand::Enqueue { track_ids: ids(&["1"]) }, next: true }, 2 * S);
        w.host.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: true }, 3 * S);
        assert!(w.host.take_commands().iter().all(|c| !c.next));
    }

    #[test]
    fn late_joiner_learns_permission_from_welcome() {
        let mut w = World::new(0, MS);
        w.host.set_guest_control(true);
        w.join(0);
        assert!(w.guest.guest_control());
        assert!(w.events.contains(&GuestEvent::ControlChanged(true)));
    }

    #[test]
    fn malformed_commands_rejected() {
        let mut h = HostSession::new("H".into(), 0);
        let secret = h.secret.clone();
        h.on_message("g", Msg::Hello { proto: PROTO_VERSION, secret, name: "G".into() }, 0);
        h.set_guest_control(true);
        let bad = [
            GuestCommand::Unknown,
            GuestCommand::Enqueue { track_ids: vec![] },
            GuestCommand::Enqueue { track_ids: vec!["12a".into()] },
            GuestCommand::Enqueue { track_ids: vec!["\"\n".into()] },
            GuestCommand::Enqueue { track_ids: vec!["1".repeat(MAX_TRACK_ID_LEN + 1)] },
            GuestCommand::Enqueue { track_ids: vec!["1".into(); MAX_ENQUEUE + 1] },
        ];
        for (i, c) in bad.into_iter().enumerate() {
            h.on_message("g", Msg::Command { cmd: c, next: false }, i as i64 * CMD_WINDOW_NS);
        }
        assert!(h.take_commands().is_empty());
        // Strangers cannot send commands at all.
        h.on_message("x", Msg::Command { cmd: GuestCommand::Play, next: false }, 100 * S);
        assert!(h.take_commands().is_empty());
        h.on_message("g", Msg::Command { cmd: GuestCommand::Enqueue { track_ids: vec!["1".into(); MAX_ENQUEUE] }, next: false }, 100 * S);
        assert_eq!(h.take_commands().len(), 1);
    }

    #[test]
    fn rejoining_does_not_refill_the_budget() {
        let mut h = HostSession::new("H".into(), 0);
        let secret = h.secret.clone();
        let hello = |s: &str| Msg::Hello { proto: PROTO_VERSION, secret: s.into(), name: "G".into() };
        h.on_message("g", hello(&secret), 0);
        h.set_guest_control(true);
        for i in 0..CMD_BURST as i64 {
            h.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: false }, S + i * MS);
        }
        h.on_message("g", Msg::Bye, S + 10 * MS);
        h.on_message("g", hello(&secret), S + RESYNC_MIN_NS + 20 * MS);
        h.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: false }, S + RESYNC_MIN_NS + 30 * MS);
        h.peer_disconnected("g");
        h.on_message("g", hello(&secret), S + 2 * RESYNC_MIN_NS + 40 * MS);
        h.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: false }, S + 2 * RESYNC_MIN_NS + 50 * MS);
        assert_eq!(h.take_commands().len(), CMD_BURST);
    }

    #[test]
    fn guest_limits_its_own_commands() {
        let mut w = World::new(0, MS);
        w.host.set_guest_control(true);
        w.join(0);
        let sent = (0..100).filter(|i| w.guest.command(GuestCommand::Next, S + i * MS).is_some()).count();
        assert_eq!(sent, CMD_BURST);
        assert!(w.guest.command(GuestCommand::Next, S + CMD_WINDOW_NS).is_some());
    }

    #[test]
    fn command_flood_is_rate_limited() {
        let mut h = HostSession::new("H".into(), 0);
        let secret = h.secret.clone();
        h.on_message("g", Msg::Hello { proto: PROTO_VERSION, secret, name: "G".into() }, 0);
        h.set_guest_control(true);
        for i in 0..1_000 {
            h.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: false }, S + i * MS);
        }
        assert_eq!(h.take_commands().len(), CMD_BURST);
        h.on_message("g", Msg::Command { cmd: GuestCommand::Next, next: false }, S + CMD_WINDOW_NS);
        assert_eq!(h.take_commands().len(), 1, "budget refills");
        // Even when the app never drains, pending commands stay bounded.
        for i in 0..1_000 {
            h.on_message("g", Msg::Command { cmd: GuestCommand::Play, next: false }, 10 * S + i * CMD_WINDOW_NS);
        }
        assert_eq!(h.take_commands().len(), MAX_PENDING_CMDS);
    }

    #[test]
    fn paused_target_is_fixed_and_no_correction() {
        let mut w = World::new(0, MS);
        w.join(0);
        for i in 1..12 {
            w.guest_tick(i * 200 * MS);
        }
        let o = w.host.set_playback(Some("i".into()), None, 42_000, false, 3 * S);
        w.deliver(o, 3 * S);
        assert_eq!(w.guest.target(60 * S, 300).unwrap().position_ms, 42_000);
        assert_eq!(w.guest.correction(60 * S, 0, 0), Correction::None);
    }

    #[test]
    fn drift_correction_uses_host_time() {
        let mut w = World::new(7 * S, 10 * MS);
        w.join(0);
        for i in 1..12 {
            w.guest_tick(i * 200 * MS);
        }
        let o = w.host.set_playback(Some("i".into()), None, 0, true, 3 * S);
        w.deliver(o, 3 * S);
        let local = 13 * S - 7 * S; // host time 13 s => host at 10_000 ms
        assert_eq!(w.guest.correction(local, 0, 10_000), Correction::None);
        assert_eq!(w.guest.correction(local, 0, 9_900), Correction::Rate(1.02));
        assert_eq!(w.guest.correction(local + 10 * MS, 0, 5_000), Correction::Seek(10_010));
    }

    #[test]
    fn worst_case_welcome_and_snapshot_fit_frame() {
        use crate::live::queue::{QueueItem, MAX_ID_LEN};
        use crate::live::MAX_QUEUE;
        let mut h = HostSession::new("\u{0}".repeat(500), 0);
        let items: Vec<QueueItem> = (0..MAX_QUEUE)
            .map(|i| QueueItem { item_id: format!("{i:0>MAX_ID_LEN$}"), track_id: "9".repeat(MAX_ID_LEN) })
            .collect();
        h.queue_op(OpKind::Replace { items }).unwrap();
        h.set_playback(Some("x".repeat(MAX_ID_LEN)), Some("y".repeat(MAX_ID_LEN)), i64::MAX, true, 0);
        let w = h.on_message("p", Msg::Hello { proto: PROTO_VERSION, secret: h.secret.clone(), name: "g".into() }, 0);
        assert!(matches!(&w[0].msg, Msg::Welcome { .. }));
        crate::live::proto::encode(&w[0].msg).unwrap();
        for o in &h.on_message("p", Msg::Resync { have: 0 }, 0) {
            crate::live::proto::encode(&o.msg).unwrap();
        }
        // Oversized or escaping ids are rejected on ingest.
        let bad = vec![QueueItem { item_id: "a".repeat(MAX_ID_LEN + 1), track_id: "t".into() }];
        assert_eq!(h.queue_op(OpKind::Replace { items: bad }), Err(QueueError::InvalidItem));
        let bad = vec![QueueItem { item_id: "a\"b".into(), track_id: "t".into() }];
        assert_eq!(h.queue_op(OpKind::Replace { items: bad }), Err(QueueError::InvalidItem));
    }
}

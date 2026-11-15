//! Continue playback on another device, through TIDAL's own per-account
//! playback state (no SDAT server, no pairing, any network).
//!
//! Every install is a player of the account. Only the active player writes
//! the queue, the current item, the position and the status; the others
//! read them. Starting to play makes a device the active player, which
//! tells the previous one to pause.

mod follow;
mod mirror;
mod wire;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::error::{CoreError, CoreResult as Result};
use crate::model::Track;
use follow::Follower;
use mirror::{Mirror, Out, Write, MAX_FUTURE};

/// Reports closer together than this are written as one.
const THROTTLE: Duration = Duration::from_millis(500);
/// Events come in bursts; one read after the last of them.
const SETTLE: Duration = Duration::from_millis(200);
/// TIDAL drops an attachment about 90 s after the last refresh.
const ATTACH_EVERY: Duration = Duration::from_secs(30);
/// State reads while an event stream is down.
const POLL_EVERY: Duration = Duration::from_secs(15);
const CALL_TIMEOUT: Duration = Duration::from_secs(30);
/// First wait before a failed write or arrival is tried again, and the longest.
const RETRY: Duration = Duration::from_secs(5);
const MAX_RETRY: Duration = Duration::from_mins(1);
/// A position written this recently by another device, found at the first
/// read with this device active, was a push to it.
const FRESH_PUSH_MS: i64 = 60_000;
/// How far from the resumed position a report may be and still be the resumed playback.
const ACK_MS: i64 = 5_000;
/// How long after taking over a queue the app has to load it.
const TAKEOVER: Duration = Duration::from_secs(10);

/// Another player of the account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    #[default]
    Off,
    One,
    All,
}

/// What another device plays, or played last.
#[derive(Debug, Clone)]
pub struct Elsewhere {
    pub device: Device,
    pub track: Track,
    /// Position at `updated_at_ms`.
    pub position_ms: i64,
    pub playing: bool,
    /// Unix ms of the last position write.
    pub updated_at_ms: i64,
}

/// The local player, as the app sees it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalPlayback {
    /// Track id; `None` when nothing is loaded.
    pub current: Option<String>,
    /// Track ids after the current one, in play order (play-next first).
    pub future: Vec<String>,
    pub position_ms: i64,
    pub playing: bool,
    pub repeat: Repeat,
}

/// Playback taken over from another device.
#[derive(Debug, Clone)]
pub struct Resume {
    pub from: Device,
    pub current: Track,
    pub future: Vec<Track>,
    pub position_ms: i64,
    pub repeat: Repeat,
}

#[derive(Debug, Clone)]
pub enum HandoffEvent {
    /// The other players of the account.
    Devices(Vec<Device>),
    /// What another device plays or last played. `None` hides the banner.
    Elsewhere(Option<Elsewhere>),
    /// Another device sent its playback here: load and play it.
    Arrived(Resume),
    /// Another device became the active player: pause here.
    MovedTo(Device),
}

/// Called on a background thread.
pub trait HandoffListener: Send + Sync {
    fn on_event(&self, ev: HandoffEvent);
}

/// A random installation key (UUID v4) for the app to keep in its settings.
pub fn new_installation_key() -> String {
    let b = rand::random::<u128>() & !(0xf000 << 64) & !(0xc << 60) | (0x4000 << 64) | (0x8 << 60);
    let h = format!("{b:032x}");
    format!("{}-{}-{}-{}-{}", &h[..8], &h[8..12], &h[12..16], &h[16..20], &h[20..])
}

enum Cmd {
    Report(LocalPlayback),
    Changed,
    /// The caller gives up at the deadline: nothing changes hands after it.
    SendTo(String, Instant, Sender<Result<()>>),
    ContinueHere(Instant, Sender<Result<Resume>>),
    /// With the access token at the time of the stop, for the detach.
    Stop(Option<String>),
}

/// Workers told to stop that have not detached yet.
static STOPPING: (Mutex<usize>, Condvar) = (Mutex::new(0), Condvar::new());

fn stopping_done() {
    *STOPPING.0.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
    STOPPING.1.notify_all();
}

/// Waits up to `timeout` until every stopped [`Handoff`] has detached
/// (before quitting); `false` on timeout.
pub fn wait_detached(timeout: Duration) -> bool {
    let n = STOPPING.0.lock().unwrap_or_else(PoisonError::into_inner);
    let (n, _) = STOPPING.1.wait_timeout_while(n, timeout, |n| *n > 0).unwrap_or_else(PoisonError::into_inner);
    *n == 0
}

/// This device as a player of the account. Dropping it detaches.
pub struct Handoff {
    tx: Sender<Cmd>,
    devices: Arc<Mutex<Vec<Device>>>,
    stopped: AtomicBool,
}

impl std::fmt::Debug for Handoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Handoff").finish_non_exhaustive()
    }
}

impl Handoff {
    /// Registers this install (idempotent), attaches it and follows the
    /// account's state. Blocking. `installation_key` comes from
    /// [`new_installation_key`], kept by the app.
    pub fn start(installation_key: &str, device_name: &str, listener: Arc<dyn HandoffListener>) -> Result<Self> {
        // A previous instance's late detach would undo this attach.
        if !wait_detached(Duration::from_secs(5)) {
            return Err(CoreError::Network("an earlier handoff is still detaching".into()));
        }
        let account = crate::auth::tokens().user_id();
        let me = wire::register(installation_key, device_name)?;
        wire::attach(&me)?;
        // Signed into another account meanwhile: this installation belongs to the old one.
        if crate::auth::tokens().user_id() != account {
            return Err(CoreError::NotSignedIn);
        }
        let (tx, rx) = mpsc::channel();
        let devices = Arc::new(Mutex::new(Vec::new()));
        let mut w = Worker {
            mirror: Mirror::new(me),
            account,
            listener,
            devices: Arc::clone(&devices),
            tx: tx.clone(),
            streams: Vec::new(),
            last_local: None,
            pending: None,
            backoff: RETRY,
            dirty: Some(Instant::now()),
            awaiting: None,
            held: None,
            arrival: None,
            sent: None,
            names: HashMap::new(),
        };
        std::thread::Builder::new().name("handoff".into()).spawn(move || w.run(&rx)).map_err(|e| CoreError::Other(e.to_string()))?;
        Ok(Self { tx, devices, stopped: AtomicBool::new(false) })
    }

    /// Every local change (track, queue, play, pause, seek). Cheap: only
    /// differences are written, and only while this device is active.
    pub fn report(&self, local: LocalPlayback) {
        let _ = self.tx.send(Cmd::Report(local));
    }

    pub fn devices(&self) -> Vec<Device> {
        self.devices.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Push the playback last reported to `device_id`. Pause here after it returns.
    pub fn send_to(&self, device_id: &str) -> Result<()> {
        self.call(|deadline, tx| Cmd::SendTo(device_id.to_string(), deadline, tx))
    }

    /// Pull: makes this device active and returns what to play.
    pub fn continue_here(&self) -> Result<Resume> {
        self.call(Cmd::ContinueHere)
    }

    /// Detach (backgrounded, quit, sign-out). Drop does the same. Returns
    /// at once; call it before removing the credentials.
    pub fn stop(&self) {
        if self.stopped.swap(true, Ordering::Relaxed) {
            return;
        }
        *STOPPING.0.lock().unwrap_or_else(PoisonError::into_inner) += 1;
        if self.tx.send(Cmd::Stop(crate::api::cached_user_token())).is_err() {
            stopping_done();
        }
    }

    /// Waits for the worker's answer, which always comes: every request has
    /// its own timeout, and past the deadline it refuses to change hands.
    fn call<T>(&self, cmd: impl FnOnce(Instant, Sender<Result<T>>) -> Cmd) -> Result<T> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(cmd(Instant::now() + CALL_TIMEOUT, tx)).map_err(|_stopped| CoreError::Other("handoff stopped".into()))?;
        rx.recv().map_err(|_stopped| CoreError::Other("handoff stopped".into()))?
    }
}

impl Drop for Handoff {
    fn drop(&mut self) {
        self.stop();
    }
}

/// The caller gave up waiting: change nothing more.
fn past(deadline: Instant) -> Result<()> {
    if Instant::now() > deadline {
        return Err(CoreError::Network("handoff: timed out".into()));
    }
    Ok(())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// Who sent playback here, while it waits to be read.
#[derive(Clone)]
struct Arrival(Option<Device>);

struct Worker {
    mirror: Mirror,
    /// The signed-in user at start: after an account switch nothing more is
    /// read or written with the new account's credentials.
    account: String,
    listener: Arc<dyn HandoffListener>,
    devices: Arc<Mutex<Vec<Device>>>,
    tx: Sender<Cmd>,
    streams: Vec<Follower>,
    last_local: Option<LocalPlayback>,
    /// When the last report is due to be written.
    pending: Option<Instant>,
    /// Wait before writing again after a failed write.
    backoff: Duration,
    /// When TIDAL's state is read next.
    dirty: Option<Instant>,
    /// After taking over a queue: what it holds, until the app reports it
    /// or the deadline passes. Other reports meanwhile are the old local
    /// playback, and would overwrite the queue.
    awaiting: Option<(LocalPlayback, Instant)>,
    /// The last report held back by `awaiting`, written once it ends.
    held: Option<LocalPlayback>,
    /// The track sent away by the last successful send, until the app pauses.
    sent: Option<String>,
    /// This device was made active but its queue could not be read yet.
    arrival: Option<Arrival>,
    /// Installation names, for devices that are no longer attached.
    names: HashMap<String, String>,
}

impl Worker {
    fn run(&mut self, rx: &Receiver<Cmd>) {
        let mut next_attach = Instant::now() + ATTACH_EVERY;
        let mut next_poll = Instant::now() + POLL_EVERY;
        loop {
            let wake = [self.pending, self.awaiting.as_ref().map(|a| a.1), self.dirty, Some(next_attach), Some(next_poll)]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(next_poll);
            match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
                Ok(Cmd::Report(l)) => self.reported(l),
                Ok(Cmd::Changed) => {
                    self.dirty.get_or_insert_with(|| Instant::now() + SETTLE);
                }
                Ok(Cmd::SendTo(id, deadline, reply)) => {
                    // The user acts on what plays now, held back or not.
                    if let Some(l) = self.held.take() {
                        self.awaiting = None;
                        self.last_local = Some(l);
                    }
                    self.pending = None;
                    let _ = reply.send(self.send_to(&id, deadline));
                }
                Ok(Cmd::ContinueHere(deadline, reply)) => {
                    // A successful continue drops the pending report; a failed one keeps it.
                    let _ = reply.send(self.continue_here(deadline));
                }
                Ok(Cmd::Stop(token)) => {
                    if self.pending.is_some() {
                        self.write_local(false);
                    }
                    self.streams.clear();
                    if let Err(e) = wire::detach(self.mirror.me(), token.as_deref()) {
                        log::info!("handoff: detach failed error={e}");
                    }
                    stopping_done();
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            if self.awaiting.as_ref().is_some_and(|a| a.1 <= now) {
                self.awaiting = None;
                if let Some(l) = self.held.take() {
                    self.reported(l);
                }
            }
            if self.pending.is_some_and(|p| p <= now) {
                self.pending = None;
                self.write_local(false);
            }
            if self.dirty.is_some_and(|d| d <= now) {
                self.dirty = None;
                if let Err(e) = self.refresh() {
                    log::info!("handoff: state read failed, retrying error={e}");
                    self.dirty.get_or_insert_with(|| Instant::now() + RETRY);
                }
            }
            if next_attach <= now {
                next_attach = now + ATTACH_EVERY;
                if self.same_account()
                    && let Err(e) = wire::attach(self.mirror.me())
                {
                    log::info!("handoff: attach failed error={e}");
                }
            }
            if next_poll <= now {
                next_poll = now + POLL_EVERY;
                if (self.streams.is_empty() || self.streams.iter().any(Follower::is_down))
                    && let Err(e) = self.refresh()
                {
                    log::info!("handoff: state read failed, retrying error={e}");
                    self.dirty.get_or_insert_with(|| Instant::now() + RETRY);
                }
            }
        }
    }

    fn reported(&mut self, l: LocalPlayback) {
        // After a send: another track playing here is a new start, which claims back.
        if let Some(sent) = &self.sent {
            if !l.playing {
                self.sent = None;
            } else if l.current.as_ref() != Some(sent) {
                self.sent = None;
                self.mirror.reset_start();
            }
        }
        if let Some((resumed, since)) = &self.awaiting {
            // The resumed playback, at about the resumed position (playing on from it).
            let elapsed = i64::try_from(TAKEOVER.saturating_sub(since.saturating_duration_since(Instant::now())).as_millis()).unwrap_or(0);
            let near = l.position_ms >= resumed.position_ms - ACK_MS && l.position_ms <= resumed.position_ms + elapsed + ACK_MS;
            if l.current != resumed.current || l.future != resumed.future || !near {
                self.held = Some(l);
                return;
            }
            self.awaiting = None;
            self.held = None;
        }
        // A pause and a play within one throttle window must both be seen:
        // starting to play is what claims the active player.
        if self.pending.is_some() && self.last_local.as_ref().is_some_and(|p| p.playing != l.playing) {
            self.pending = None;
            self.write_local(false);
        }
        self.last_local = Some(l);
        self.pending.get_or_insert_with(|| Instant::now() + THROTTLE);
    }

    fn emit(&self, ev: HandoffEvent) {
        if let HandoffEvent::Devices(d) = &ev {
            d.clone_into(&mut self.devices.lock().unwrap_or_else(PoisonError::into_inner));
        }
        self.listener.on_event(ev);
    }

    fn follow(&mut self, urls: &[Option<String>]) {
        let urls: Vec<String> = urls.iter().flatten().cloned().collect();
        self.streams.retain(|s| urls.contains(&s.url));
        for u in urls {
            if !self.streams.iter().any(|s| s.url == u) {
                self.streams.push(Follower::spawn(u, self.tx.clone()));
            }
        }
    }

    fn same_account(&self) -> bool {
        crate::auth::tokens().user_id() == self.account
    }

    /// Reads TIDAL's state and tells the app what changed.
    fn refresh(&mut self) -> Result<()> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        let state = wire::state()?;
        let head = state.queue_id.as_deref().map(wire::head_with_track).transpose()?;
        self.follow(&[state.stream.clone(), head.as_ref().and_then(|h| h.stream.clone())]);
        for p in &state.players {
            self.names.insert(p.id.clone(), p.name.clone());
        }
        let by = head.as_ref().and_then(|h| h.updated_by.clone()).map(|id| {
            let name = self.names.entry(id.clone()).or_insert_with(|| wire::installation_name(&id).unwrap_or_default()).clone();
            Device { id, name }
        });
        // Sent here between this start's attach and its first read: that read
        // has no earlier state to tell the change from. A recent position
        // written by another device is a push; an old one is a stale selection.
        let me = self.mirror.me();
        let fresh_push = !self.mirror.has_fetched()
            && state.active.as_deref() == Some(me)
            && head.as_ref().is_some_and(|h| {
                h.updated_by.as_deref().is_some_and(|u| u != me) && h.position_at_ms.is_some_and(|at| now_ms() - at < FRESH_PUSH_MS)
            });
        if fresh_push {
            self.arrival = Some(Arrival(by.clone()));
        }
        for o in self.mirror.fetched(state, head, by.clone()) {
            match o {
                Out::Event(ev) => self.emit(*ev),
                Out::Arrive => self.arrival = Some(Arrival(by.clone())),
            }
        }
        if !self.mirror.is_active() {
            self.arrival = None;
        }
        if let Some(Arrival(from)) = self.arrival.clone() {
            // The reads above may span another claim: only a device still active plays.
            // A failed read keeps the arrival; the caller reads again shortly.
            if wire::state()?.active.as_deref() != Some(self.mirror.me()) {
                self.arrival = None;
                return Ok(());
            }
            match self.resume(None, from) {
                Ok(r) => {
                    self.arrival = None;
                    self.emit(HandoffEvent::Arrived(r));
                }
                Err(e) => {
                    log::warn!("handoff: arrived queue unreadable, retrying error={e}");
                    self.dirty.get_or_insert_with(|| Instant::now() + RETRY);
                }
            }
        }
        Ok(())
    }

    /// Makes this device the active player; an error if TIDAL picked another.
    fn claim(&mut self) -> Result<()> {
        let active = wire::set_active(self.mirror.me())?;
        if active.as_deref() != Some(self.mirror.me()) {
            return Err(CoreError::Other("TIDAL did not make this device active".into()));
        }
        self.mirror.claimed(active);
        Ok(())
    }

    /// Reads the queue for playing it here, from the device that wrote it.
    /// `claim` makes this device active once the queue proved readable.
    fn resume(&mut self, claim: Option<Instant>, from: Option<Device>) -> Result<Resume> {
        let mut queue = self.mirror.queue_id().ok_or_else(|| CoreError::Other("nothing to continue".into()))?.to_string();
        let before = self.mirror.state().active.clone().filter(|a| a != self.mirror.me());
        let mut claimed = false;
        // Read the selected queue until a snapshot agrees with itself and is
        // still the selection, before the claim and once more after it (the
        // source may switch queues in between).
        let mut rounds = 0;
        let (head, mut entries, future, current, track) = loop {
            rounds += 1;
            let read = Self::snapshot(&queue).and_then(|r| Ok((r, wire::state()?)));
            let ((head, entries, future), now) = match read {
                Ok(r) => r,
                Err(e) => return Err(self.undo_claim(claimed, before.as_deref(), e)),
            };
            // Another device claimed after this one: it owns playback now, and is left alone.
            if claimed && now.active.as_deref() != Some(self.mirror.me()) {
                self.mirror.claimed(now.active);
                return Err(CoreError::Other("another device took over playback".into()));
            }
            let selected = now.queue_id;
            if selected.as_deref() != Some(queue.as_str()) {
                let Some(next) = selected.filter(|_| rounds < 4) else {
                    return Err(self.undo_claim(claimed, before.as_deref(), CoreError::Network("the queue changed while it was read".into())));
                };
                self.mirror.queue_created(next.clone());
                queue = next;
                continue;
            }
            let (Some(current), Some(track)) = (head.current.clone(), head.track.clone()) else {
                return Err(self.undo_claim(claimed, before.as_deref(), CoreError::Other("nothing to continue".into())));
            };
            match claim {
                Some(deadline) if !claimed => {
                    past(deadline)?;
                    self.claim()?;
                    claimed = true;
                }
                _ => break (head, entries, future, current, track),
            }
        };
        let duration = i64::try_from(track.duration().as_millis()).unwrap_or(i64::MAX);
        // A pull takes over a player that may still be playing; a push wrote its exact position.
        let playing = claim.is_some() && self.mirror.state().status == wire::Status::Playing;
        let now = now_ms();
        let position_ms = mirror::position_now(head.position_ms.unwrap_or(0), head.position_at_ms.unwrap_or(now), now, playing, duration);
        let resumed = LocalPlayback {
            current: Some(track.id().to_string()),
            future: future.iter().map(|t| t.id().to_string()).collect(),
            position_ms,
            ..LocalPlayback::default()
        };
        entries.insert(0, current);
        self.mirror.adopt(entries, &head, position_ms, now);
        // A report from before this would overwrite the queue just taken over.
        self.pending = None;
        self.last_local = None;
        self.held = None;
        self.awaiting = Some((resumed, Instant::now() + TAKEOVER));
        let from = from
            .or_else(|| head.updated_by.as_deref().and_then(|id| self.mirror.device(id)))
            .unwrap_or_else(|| Device { id: head.updated_by.clone().unwrap_or_default(), name: String::new() });
        Ok(Resume { from, current: track, future, position_ms, repeat: head.repeat })
    }

    /// Head, entries and tracks of `queue` from reads that agree on its revision.
    fn snapshot(queue: &str) -> Result<(wire::Head, Vec<wire::Entry>, Vec<Track>)> {
        for _ in 0..3 {
            let head = wire::head_with_track(queue)?;
            let (entries, future) = wire::future_tracks(queue, MAX_FUTURE)?;
            if wire::head(queue)?.revision == head.revision {
                return Ok((head, entries, future));
            }
        }
        Err(CoreError::Network("the queue kept changing while it was read".into()))
    }

    /// A pull that claimed and then failed hands playback back to where it was.
    fn undo_claim(&mut self, claimed: bool, before: Option<&str>, e: CoreError) -> CoreError {
        // Only while this device still holds it: a newer claim by a third device stays.
        let still_mine = || wire::state().is_ok_and(|s| s.active.as_deref() == Some(self.mirror.me()));
        if claimed
            && let Some(b) = before
            && still_mine()
        {
            match wire::set_active(b) {
                Ok(active) => self.mirror.claimed(active),
                Err(e) => log::warn!("handoff: could not hand playback back error={e}"),
            }
        }
        e
    }

    fn continue_here(&mut self, deadline: Instant) -> Result<Resume> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        // Never continue from a remembered queue: it may be gone.
        self.refresh()?;
        let from = self.mirror.state().active.as_deref().filter(|a| *a != self.mirror.me()).and_then(|a| self.mirror.device(a));
        self.arrival = None;
        self.resume(Some(deadline), from)
    }

    fn send_to(&mut self, id: &str, deadline: Instant) -> Result<()> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        // Only before the first write: once started, the send runs to an answer.
        past(deadline)?;
        if self.last_local.as_ref().is_none_or(|l| l.current.is_none()) {
            return Err(CoreError::Other("nothing playing".into()));
        }
        if self.mirror.state().active.as_deref() != Some(self.mirror.me()) {
            self.claim()?;
        }
        if !self.write_local(true) {
            return Err(CoreError::Other("could not write the queue".into()));
        }
        // Paused before the target owns the state, so this write cannot land after its own.
        wire::set_status(false)?;
        self.mirror.status_written(false);
        let handed = wire::set_active(id);
        if !matches!(&handed, Ok(Some(a)) if a == id) {
            // An unattached target leaves no active player, and a failed call
            // leaves it unknown: take it back and undo the pause.
            if self.claim().is_ok() && self.last_local.as_ref().is_some_and(|l| l.playing) && wire::set_status(true).is_ok() {
                self.mirror.status_written(true);
            }
            handed?;
            return Err(CoreError::Other("the device is not available".into()));
        }
        self.mirror.released_to(id);
        self.sent = self.last_local.as_ref().and_then(|l| l.current.clone());
        Ok(())
    }

    /// Writes the last report; `exact` also writes its position. `false` on
    /// failure, and then it is tried again later.
    fn write_local(&mut self, exact: bool) -> bool {
        let Some(local) = self.last_local.clone() else { return true };
        if !self.same_account() {
            return false;
        }
        if exact {
            self.mirror.force_position();
        }
        let writes = self.mirror.plan(&local, now_ms());
        match self.apply(writes, &local) {
            Ok(()) => {
                self.backoff = RETRY;
                true
            }
            Err(e) => {
                log::warn!("handoff: write failed, retrying in {}s error={e}", self.backoff.as_secs());
                self.mirror.write_failed();
                self.pending.get_or_insert_with(|| Instant::now() + self.backoff);
                self.backoff = (self.backoff * 2).min(MAX_RETRY);
                false
            }
        }
    }

    fn apply(&mut self, writes: Vec<Write>, local: &LocalPlayback) -> Result<()> {
        let me = self.mirror.me().to_string();
        for w in writes {
            // A sign-in to another account during the batch ends it.
            if !self.same_account() {
                return Err(CoreError::NotSignedIn);
            }
            match w {
                Write::Claim => self.claim()?,
                Write::NewQueue => self.mirror.queue_created(wire::create_queue()?),
                Write::Queue(tracks) => {
                    let q = self.queue()?;
                    wire::rewrite(&q, &tracks, local.repeat)?;
                    let head = wire::head(&q)?;
                    let mut entries = wire::future_entries(&q, MAX_FUTURE)?;
                    entries.splice(0..0, head.current);
                    self.mirror.queue_written(tracks, entries, head.revision, local.repeat);
                }
                Write::Repeat(r) => {
                    let rev = wire::set_repeat(&self.queue()?, r)?;
                    self.mirror.repeat_written(r, rev);
                }
                Write::Current { index, position_ms } => {
                    let e = self.mirror.entry(index).ok_or_else(|| CoreError::Other("queue entry missing".into()))?;
                    let rev = wire::set_current(&self.queue()?, &e, position_ms, &me)?;
                    self.mirror.current_written(index, position_ms, local.playing, now_ms(), rev);
                }
                Write::Status { playing } => {
                    wire::set_status(playing)?;
                    self.mirror.status_written(playing);
                }
            }
        }
        Ok(())
    }

    fn queue(&self) -> Result<String> {
        self.mirror.queue_id().map(str::to_string).ok_or_else(|| CoreError::Other("no queue".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installation_keys_are_uuid_v4() {
        let k = new_installation_key();
        assert_eq!(k.split('-').map(str::len).collect::<Vec<_>>(), [8, 4, 4, 4, 12]);
        assert_eq!(&k[14..15], "4");
        assert!(matches!(&k[19..20], "8" | "9" | "a" | "b"));
        assert_ne!(k, new_installation_key());
    }

    struct Log(Mutex<Vec<String>>);
    impl HandoffListener for Log {
        fn on_event(&self, ev: HandoffEvent) {
            let line = match ev {
                HandoffEvent::Devices(d) => format!("devices {:?}", d.iter().map(|d| &d.name).collect::<Vec<_>>()),
                HandoffEvent::Elsewhere(e) => format!("elsewhere {:?}", e.map(|e| (e.device.name, e.track.title().to_string(), e.playing))),
                HandoffEvent::Arrived(r) => format!("arrived {} +{} at {}", r.current.title(), r.future.len(), r.position_ms),
                HandoffEvent::MovedTo(d) => format!("moved to {}", d.name),
            };
            eprintln!("{line}");
            self.0.lock().unwrap_or_else(PoisonError::into_inner).push(line);
        }
    }

    /// Two installs of the signed-in account hand playback back and forth.
    /// Needs `SDAT_LIVE_REFRESH_TOKEN`; changes the account's playback state.
    #[test]
    #[ignore = "live: writes the TIDAL account's playback state"]
    fn live_handoff_between_two_installs() {
        struct Env;
        impl crate::auth::TokenStore for Env {
            fn has_refresh_token(&self) -> bool {
                std::env::var("SDAT_LIVE_REFRESH_TOKEN").is_ok()
            }
            fn get_refresh_token(&self) -> String {
                std::env::var("SDAT_LIVE_REFRESH_TOKEN").unwrap_or_default()
            }
            fn user_id(&self) -> String {
                String::new()
            }
        }
        crate::auth::install(Box::new(Env));
        let wait = |log: &Log, what: &str| {
            let until = Instant::now() + Duration::from_secs(20);
            while Instant::now() < until {
                if log.0.lock().unwrap_or_else(PoisonError::into_inner).iter().any(|l| l.starts_with(what) || what.contains('*') && what.split('*').all(|p| l.contains(p))) {
                    return;
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            panic!("no '{what}' in {:?}", log.0.lock().unwrap_or_else(PoisonError::into_inner));
        };
        let (la, lb) = (Arc::new(Log(Mutex::new(vec![]))), Arc::new(Log(Mutex::new(vec![]))));
        let a = Handoff::start(&std::env::var("SDAT_LIVE_KEY_A").unwrap_or_else(|_| new_installation_key()), "SDAT test A", Arc::clone(&la) as Arc<dyn HandoffListener>).expect("start a");
        let b = Handoff::start(&std::env::var("SDAT_LIVE_KEY_B").unwrap_or_else(|_| new_installation_key()), "SDAT test B", Arc::clone(&lb) as Arc<dyn HandoffListener>).expect("start b");
        let ids: Vec<String> = ["35707532", "60923645", "40475829", "504370449", "162358163"].map(String::from).to_vec();
        a.report(LocalPlayback { current: Some(ids[0].clone()), future: ids[1..].to_vec(), position_ms: 30_000, playing: true, repeat: Repeat::Off });
        // Playing, so not what a previous run left behind.
        wait(&lb, "elsewhere Some((\"SDAT test A\"*true))");
        let r = b.continue_here().expect("continue here");
        assert_eq!(r.current.id(), ids[0]);
        assert_eq!(r.future.iter().map(|t| t.id().to_string()).collect::<Vec<_>>(), ids[1..]);
        assert!(r.position_ms >= 30_000, "position {}", r.position_ms);
        wait(&la, "moved to SDAT test B");
        a.report(LocalPlayback { current: Some(ids[0].clone()), future: ids[1..].to_vec(), position_ms: 31_000, playing: false, repeat: Repeat::Off });
        // B still reports its old playback, then the resumed one, then moves on.
        b.report(LocalPlayback { current: Some("1".into()), future: vec![], position_ms: 0, playing: false, repeat: Repeat::Off });
        b.report(LocalPlayback { current: Some(ids[0].clone()), future: ids[1..].to_vec(), position_ms: r.position_ms, playing: true, repeat: Repeat::Off });
        std::thread::sleep(Duration::from_secs(1));
        b.report(LocalPlayback { current: Some(ids[1].clone()), future: ids[2..].to_vec(), position_ms: 12_345, playing: true, repeat: Repeat::Off });
        std::thread::sleep(Duration::from_secs(2));
        let a_id = b.devices().into_iter().find(|d| d.name == "SDAT test A").expect("a listed").id;
        b.send_to(&a_id).expect("send to a");
        wait(&la, "arrived");
        let got = la.0.lock().unwrap_or_else(PoisonError::into_inner).iter().find(|l| l.starts_with("arrived")).cloned().unwrap_or_default();
        assert!(got.ends_with("+3 at 12345"), "{got}");
        a.stop();
        b.stop();
        assert!(wait_detached(Duration::from_secs(10)), "not detached");
    }
}

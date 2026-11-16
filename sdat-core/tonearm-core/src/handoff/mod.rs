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
const WORKER_THREAD: &str = "handoff";
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
/// A failed read of what follows a resumed track is tried again this often.
const FUTURE_RETRY: Duration = Duration::from_secs(2);
const FUTURE_RETRIES: u32 = 2;
/// A resumed track with less left than this could end before what follows it
/// arrives: that is read before the resume is delivered.
const NEAR_END_MS: i64 = 20_000;

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

/// Playback taken over from another device: the current track only, so it
/// starts sooner. What follows it comes after, as [`HandoffEvent::Upcoming`].
#[derive(Debug, Clone)]
pub struct Resume {
    pub from: Device,
    pub current: Track,
    pub position_ms: i64,
    pub repeat: Repeat,
}

/// The tracks after a resumed one.
#[derive(Debug, Clone)]
pub struct Upcoming {
    /// Track id of the resumed track they follow.
    pub current: String,
    pub tracks: Vec<Track>,
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
    /// What follows the track of the last [`HandoffEvent::Arrived`] or
    /// [`Handoff::continue_here`], read after it. Append it while that track
    /// is still the current one. Not sent when nothing follows.
    Upcoming(Upcoming),
}

/// Called on the handoff's own thread: hand events on, don't block on them.
/// [`Handoff::send_to`] and [`Handoff::continue_here`] fail when called from it.
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
    /// With the time it was reported, which a playing position counts from.
    Report(LocalPlayback, Instant),
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

/// Live instances per installation id: the attachment is shared, so only the
/// last instance to stop detaches it.
static LIVE: Mutex<Vec<(String, usize)>> = Mutex::new(Vec::new());

/// Held around every attach and detach, so a stopping instance's detach and a
/// new instance's attach of the same installation can't interleave.
static ATTACHING: Mutex<()> = Mutex::new(());

/// One more live instance of installation `id`.
fn attached(id: &str) {
    let mut l = LIVE.lock().unwrap_or_else(PoisonError::into_inner);
    match l.iter_mut().find(|(i, _)| i == id) {
        Some((_, n)) => *n += 1,
        None => l.push((id.to_string(), 1)),
    }
}

/// One live instance of `id` fewer; `true` if it was the last.
fn released(id: &str) -> bool {
    let mut l = LIVE.lock().unwrap_or_else(PoisonError::into_inner);
    let Some(k) = l.iter().position(|(i, _)| i == id) else { return true };
    l[k].1 -= 1;
    let last = l[k].1 == 0;
    if last {
        l.remove(k);
    }
    last
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
    /// The user this instance was started for: the detach token must be theirs.
    account: String,
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
        // Registration and attach go out for the account signed in now, and no other.
        wire::bind_to(crate::auth::tokens().user_id());
        let started = Self::start_bound(installation_key, device_name, listener);
        wire::unbind();
        started
    }

    fn start_bound(installation_key: &str, device_name: &str, listener: Arc<dyn HandoffListener>) -> Result<Self> {
        // A previous instance's late detach would undo this attach.
        if !wait_detached(Duration::from_secs(5)) {
            return Err(CoreError::Network("an earlier handoff is still detaching".into()));
        }
        let account = crate::auth::tokens().user_id();
        // To detach with this account if another one signs in during the start.
        let token = crate::api::cached_user_token();
        let me = wire::register(installation_key, device_name)?;
        {
            let _serial = ATTACHING.lock().unwrap_or_else(PoisonError::into_inner);
            attached(&me);
            if let Err(e) = wire::attach(&me) {
                released(&me);
                return Err(e);
            }
        }
        // Signed into another account meanwhile: this installation belongs to the old one.
        if crate::auth::tokens().user_id() != account {
            if released(&me)
                && let Some(t) = token.as_deref()
            {
                drop(wire::detach(&me, Some(t)));
            }
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
            last_local_at: Instant::now(),
            pending: None,
            backoff: RETRY,
            dirty: Some(Instant::now()),
            awaiting: None,
            future_due: None,
            held: None,
            before_takeover: None,
            arrival: None,
            sent: None,
            quarantined: None,
            quarantine_track: None,
            rollback: None,
            rollback_queue: None,
            rollback_playing: false,
            names: HashMap::new(),
        };
        let me = w.mirror.me().to_string();
        if let Err(e) = std::thread::Builder::new().name(WORKER_THREAD.into()).spawn(move || w.run(&rx)) {
            // No worker will ever detach it.
            if released(&me) {
                drop(wire::detach(&me, None));
            }
            return Err(CoreError::Other(e.to_string()));
        }
        Ok(Self { tx, devices, stopped: AtomicBool::new(false), account: crate::auth::tokens().user_id() })
    }

    /// Every local change (track, queue, play, pause, seek). Cheap: only
    /// differences are written, and only while this device is active.
    pub fn report(&self, local: LocalPlayback) {
        let _ = self.tx.send(Cmd::Report(local, Instant::now()));
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
        // Another account already signed in: its token mustn't detach this one; TIDAL drops it later.
        let token = crate::api::cached_user_token().filter(|_| crate::auth::tokens().user_id() == self.account);
        if self.tx.send(Cmd::Stop(token)).is_err() {
            stopping_done();
        }
    }

    /// Waits for the worker's answer, which always comes: every request has
    /// its own timeout, and past the deadline it refuses to change hands.
    fn call<T>(&self, cmd: impl FnOnce(Instant, Sender<Result<T>>) -> Cmd) -> Result<T> {
        // From the listener, on the worker thread: waiting for the worker would never end.
        if std::thread::current().name() == Some(WORKER_THREAD) {
            return Err(CoreError::Other("handoff: called from its own listener".into()));
        }
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

/// Playback just taken over, until the app reports it or `until` passes.
struct Takeover {
    resumed: LocalPlayback,
    /// When the resume was read: a playing report has moved on from its position since.
    read_at: Instant,
    until: Instant,
}

impl Takeover {
    /// `l` is the resumed playback, at about the resumed position (playing on from it).
    fn reported_by(&self, l: &LocalPlayback, now: Instant) -> bool {
        let elapsed = i64::try_from(now.saturating_duration_since(self.read_at).as_millis()).unwrap_or(i64::MAX);
        let near = l.position_ms >= self.resumed.position_ms.saturating_sub(ACK_MS)
            && l.position_ms <= self.resumed.position_ms.saturating_add(elapsed).saturating_add(ACK_MS);
        l.current == self.resumed.current && l.future == self.resumed.future && near
    }
}

/// A resume delivered without what follows its track: the queue it was read
/// from and the track, read next at `at` unless `tail` already holds it.
struct Resumed {
    queue: String,
    track: String,
    retries: u32,
    at: Instant,
    tail: Option<Tail>,
}

/// The entries and tracks after a queue's current one, and its revision.
type Tail = (Vec<wire::Entry>, Vec<Track>, String);

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
    /// When `last_local` was reported.
    last_local_at: Instant,
    /// When the last report is due to be written.
    pending: Option<Instant>,
    /// Wait before writing again after a failed write.
    backoff: Duration,
    /// When TIDAL's state is read next.
    dirty: Option<Instant>,
    /// After taking over a queue: other reports meanwhile are the old local
    /// playback, and would overwrite the queue.
    awaiting: Option<Takeover>,
    /// What follows the last resumed track, read before the next command.
    future_due: Option<Resumed>,
    /// The last report held back by `awaiting`, written once it ends.
    held: Option<(LocalPlayback, Instant)>,
    /// The track that played here when a takeover began.
    before_takeover: Option<String>,
    /// A failed pull or send to undo once the account can be read: the player it took over from.
    rollback: Option<String>,
    /// With `rollback` after a send: that player's queue, selected again first.
    rollback_queue: Option<String>,
    /// With `rollback` after a send: that player was playing.
    rollback_playing: bool,
    /// The report from before a send whose outcome is unknown, until a state read tells.
    quarantined: Option<(LocalPlayback, Instant)>,
    /// The track that send was about, to tell a new local start from the old report.
    quarantine_track: Option<String>,
    /// The track sent away by the last successful send, until the app pauses.
    sent: Option<String>,
    /// This device was made active but its queue could not be read yet.
    arrival: Option<Arrival>,
    /// Installation names, for devices that are no longer attached.
    names: HashMap<String, String>,
}

impl Worker {
    fn run(&mut self, rx: &Receiver<Cmd>) {
        wire::bind_to(self.account.clone());
        let mut next_attach = Instant::now() + ATTACH_EVERY;
        let mut next_poll = Instant::now() + POLL_EVERY;
        loop {
            if self.future_due.as_ref().is_some_and(|r| r.at <= Instant::now())
                && let Some(r) = self.future_due.take()
            {
                self.read_future(r);
            }
            let wake = [self.pending, self.awaiting.as_ref().map(|t| t.until), self.future_due.as_ref().map(|r| r.at), self.dirty, Some(next_attach), Some(next_poll)]
                .into_iter()
                .flatten()
                .min()
                .unwrap_or(next_poll);
            match rx.recv_timeout(wake.saturating_duration_since(Instant::now())) {
                Ok(Cmd::Report(l, at)) => self.reported(l, at),
                Ok(Cmd::Changed) => {
                    self.dirty.get_or_insert_with(|| Instant::now() + SETTLE);
                }
                Ok(Cmd::SendTo(id, deadline, reply)) => {
                    // The pending report stays scheduled: a send that fails early must not drop it.
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
                    // Another live instance of this installation still uses the attachment.
                    let serial = ATTACHING.lock().unwrap_or_else(PoisonError::into_inner);
                    if released(self.mirror.me())
                        && let Err(e) = wire::detach(self.mirror.me(), token.as_deref())
                    {
                        log::info!("handoff: detach failed error={e}");
                    }
                    drop(serial);
                    stopping_done();
                    return;
                }
                Err(RecvTimeoutError::Disconnected) => return,
                Err(RecvTimeoutError::Timeout) => {}
            }
            let now = Instant::now();
            if self.awaiting.as_ref().is_some_and(|t| t.until <= now) {
                self.awaiting = None;
                let before = self.before_takeover.take();
                // Only a track started here since then counts: the old playback is dropped.
                if let Some((l, at)) = self.held.take().filter(|(l, _)| l.current.is_some() && l.current != before) {
                    self.reported(l, at);
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

    /// `at`: when `l` was reported, which a playing position counts from.
    fn reported(&mut self, l: LocalPlayback, at: Instant) {
        // While an unknown send outcome is unresolved, a newer report replaces the
        // held-back one and waits with it: nothing claims before ownership is read.
        if self.quarantined.is_some() {
            self.quarantined = Some((l, at));
            return;
        }
        // After a send: another track playing here is a new start, which claims back.
        if let Some(sent) = &self.sent {
            if !l.playing {
                self.sent = None;
            } else if l.current.as_ref() != Some(sent) {
                self.sent = None;
                self.mirror.reset_start();
            }
        }
        if let Some(t) = &self.awaiting {
            if !t.reported_by(&l, Instant::now()) {
                self.held = Some((l, at));
                return;
            }
            self.awaiting = None;
            self.held = None;
            self.before_takeover = None;
        }
        // A pause and a play within one throttle window must both be seen:
        // starting to play is what claims the active player.
        if self.pending.is_some() && self.last_local.as_ref().is_some_and(|p| p.playing != l.playing) {
            self.pending = None;
            self.write_local(false);
        }
        self.last_local = Some(l);
        self.last_local_at = at;
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
        self.streams.retain(|s| urls.contains(&s.url) && !s.is_dead());
        for u in urls {
            if !self.streams.iter().any(|s| s.url == u) {
                self.streams.push(Follower::spawn(u, self.tx.clone()));
            }
        }
    }

    /// A write made only while the account is still the one this worker started with.
    fn checked<T>(&self, write: impl FnOnce() -> Result<T>) -> Result<T> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        write()
    }

    fn same_account(&self) -> bool {
        crate::auth::tokens().user_id() == self.account
    }

    /// Reads TIDAL's state and tells the app what changed. Returns the state
    /// and then its selected queue's head, as read.
    fn refresh(&mut self) -> Result<(wire::State, Option<wire::Head>)> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        let mut state = wire::state()?;
        if let Some(b) = self.rollback.take() {
            // A failed pull still holding playback that nothing plays here: hand it back.
            let idle = self.last_local.as_ref().is_none_or(|l| !l.playing);
            if idle && (state.active.is_none() || state.active.as_deref() == Some(self.mirror.me())) {
                if let Err(e) = self.hand_back(&b) {
                    self.rollback = Some(b);
                    return Err(e);
                }
                state = wire::state()?;
            } else if state.active.as_deref() == Some(b.as_str()) {
                // Handed back already; only its playing status may still be missing.
                self.rollback_queue = None;
                if let Err(e) = self.restore_playing() {
                    self.rollback = Some(b);
                    return Err(e);
                }
            } else {
                self.rollback_queue = None;
                self.rollback_playing = false;
            }
        }
        // Told before the queue read, so this device pauses one call sooner. Not while
        // a send's outcome is unsettled: a track started here since then keeps playing.
        if self.quarantined.is_none()
            && let Some(ev) = self.mirror.moved(&state)
        {
            self.emit(ev);
        }
        let head = state.queue_id.as_deref().map(wire::head_with_track).transpose()?;
        // Settled only by a complete read (the head too): a failed read keeps the report held.
        if let Some((l, at)) = self.quarantined.take() {
            let sent = self.quarantine_track.take();
            if state.active.as_deref() == Some(self.mirror.me()) {
                // Still the active player after an unknown send outcome: its playback is written
                // again, with the time it was reported so a playing position moves on from there.
                self.mirror.claimed(state.active.clone());
                self.last_local = Some(l);
                self.last_local_at = at;
                self.mirror.force_position();
                self.pending.get_or_insert_with(Instant::now);
            } else if state.active.is_none() {
                // Nobody took it: the send failed, and playback stays here. Playing claims
                // again (and writes its status); a paused report writes nothing.
                if l.playing {
                    self.mirror.reset_start();
                }
                self.last_local = Some(l);
                self.last_local_at = at;
                self.pending.get_or_insert_with(Instant::now);
            } else if !l.playing {
                // Paused here since the send: nothing to hold back, and a later play is a start.
            } else if !(l.current.is_some() && l.current != sent) {
                // The send went through: the old playback here stops once the move
                // arrives, and its reports until then claim nothing.
                self.sent = sent;
                self.mirror.still_playing();
            } else {
                // The send went through, but the user started another track here since:
                // that start claims playback, and isn't paused by the move.
                if let Some(a) = state.active.as_deref() {
                    self.mirror.released_to(a);
                }
                self.mirror.reset_start();
                self.last_local = Some(l);
                self.last_local_at = at;
                self.pending.get_or_insert_with(Instant::now);
            }
        }
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
                h.updated_by.as_deref().is_some_and(|u| u != me) && h.position_at_ms.is_some_and(|at| (0..FRESH_PUSH_MS).contains(&(now_ms() - at)))
            });
        if fresh_push {
            self.arrival = Some(Arrival(by.clone()));
        }
        let read = (state.clone(), head.clone());
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
            let now = wire::state()?;
            if now.active.as_deref() != Some(self.mirror.me()) {
                self.arrival = None;
                return Ok(read);
            }
            // The head read above and this state are the resume's first round.
            let selected = read.0.queue_id.clone().zip(read.1.clone());
            match self.resume(None, from, selected, now) {
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
        Ok(read)
    }

    /// Makes this device the active player; an error if TIDAL picked another.
    fn claim(&mut self) -> Result<()> {
        let me = self.mirror.me().to_string();
        let active = self.checked(|| wire::set_active(&me))?;
        if active.as_deref() != Some(self.mirror.me()) {
            return Err(CoreError::Other("TIDAL did not make this device active".into()));
        }
        self.mirror.claimed(active);
        Ok(())
    }

    /// Reads the current track for playing it here, from the device that wrote
    /// it; what follows it is read after the resume is delivered. `selected` is
    /// the selected queue and its head, read before `state`: the first round.
    /// `claim` makes this device active once the queue proved readable.
    fn resume(&mut self, claim: Option<Instant>, from: Option<Device>, selected: Option<(String, wire::Head)>, state: wire::State) -> Result<Resume> {
        let (mut queue, head) = selected.ok_or_else(|| CoreError::Other("nothing to continue".into()))?;
        let mut read = Some((head, state));
        let mut before: Option<String> = None;
        let mut status;
        let mut claimed = false;
        // Read the selected queue's head, then the state, until it is still the
        // selection, before the claim and once more after it (the source may
        // switch queues in between).
        let mut rounds = 0;
        let (head, current, track) = loop {
            rounds += 1;
            let read = read.take().map_or_else(|| wire::head_with_track(&queue).and_then(|h| Ok((h, wire::state()?))), Ok);
            let (head, now) = match read {
                Ok(r) => r,
                Err(e) => return Err(self.undo_claim(claimed, before.as_deref(), e)),
            };
            // Another device claimed after this one (or, for a push, after the sender
            // handed over): it owns playback now, and is left alone.
            if (claimed || claim.is_none()) && now.active.as_deref() != Some(self.mirror.me()) {
                // The next read reports the move, so a player still going here pauses.
                self.dirty.get_or_insert_with(Instant::now);
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
            status = now.status;
            match claim {
                Some(deadline) if !claimed => {
                    past(deadline)?;
                    // The owner this claim displaces, as read right before it.
                    before = now.active.filter(|a| a != self.mirror.me());
                    // A failed claim may still have reached TIDAL: undo it if it did.
                    if let Err(e) = self.claim() {
                        return Err(self.undo_claim(true, before.as_deref(), e));
                    }
                    claimed = true;
                }
                _ => break (head, current, track),
            }
        };
        // The caller gave up during the reads after the claim: give it back.
        if let Some(deadline) = claim
            && Instant::now() > deadline
        {
            return Err(self.undo_claim(claimed, before.as_deref(), CoreError::Network("handoff: timed out".into())));
        }
        let duration = i64::try_from(track.duration().as_millis()).unwrap_or(i64::MAX);
        // A pull takes over a player that may still be playing; a push wrote its exact position.
        let playing = claim.is_some() && status == wire::Status::Playing;
        let now = now_ms();
        let position_ms = mirror::position_now(head.position_ms.unwrap_or(0), head.position_at_ms.unwrap_or(now), now, playing, duration);
        // Nothing follows it until that is read: the app plays the current track alone first.
        let resumed = LocalPlayback { current: Some(track.id().to_string()), position_ms, ..LocalPlayback::default() };
        self.mirror.adopt(current, &head, position_ms, now);
        // A report from before this would overwrite the queue just taken over.
        self.pending = None;
        // What played here before: if it still is when the wait ends, it is old playback, not a choice.
        self.before_takeover = self.last_local.take().and_then(|l| l.current);
        self.held = None;
        let read_at = Instant::now();
        self.awaiting = Some(Takeover { resumed, read_at, until: read_at + TAKEOVER });
        let tail = (duration.saturating_sub(position_ms) < NEAR_END_MS)
            .then(|| Self::read_tail(&queue).inspect_err(|e| log::info!("handoff: reading what follows a track near its end failed error={e}")).ok())
            .flatten();
        self.future_due = Some(Resumed { queue, track: track.id().to_string(), retries: 0, at: read_at, tail });
        let from = from
            .or_else(|| head.updated_by.as_deref().and_then(|id| self.mirror.device(id)))
            .unwrap_or_else(|| Device { id: head.updated_by.clone().unwrap_or_default(), name: String::new() });
        Ok(Resume { from, current: track, position_ms, repeat: head.repeat })
    }

    /// Reads what follows a resumed track and tells the app. A failed read is
    /// tried again a little later; after the last try the track plays alone
    /// and TIDAL's queue is left as it is.
    fn read_future(&mut self, mut r: Resumed) {
        match r.tail.take().map_or_else(|| Self::read_tail(&r.queue), Ok) {
            Ok((_, tracks, _)) if tracks.is_empty() => {}
            Ok((entries, tracks, revision)) => {
                // The queue was rewritten since, here or elsewhere: what follows is no longer the resume's.
                if !self.mirror.holds_adoption(&revision) {
                    log::info!("handoff: the resumed queue changed before what follows was read");
                    return;
                }
                // Taken into the mirror only while the app's report of the track alone is held back: one
                // already taken would plan a rewrite to that track. Without it, the app's append rewrites them.
                if let Some(t) = self.awaiting.as_mut().filter(|t| t.resumed.current.as_ref() == Some(&r.track)) {
                    self.mirror.adopted_future(&revision, entries);
                    t.resumed.future = tracks.iter().map(|t| t.id().to_string()).collect();
                    // The app needs the time to append them.
                    t.until = Instant::now() + TAKEOVER;
                }
                self.emit(HandoffEvent::Upcoming(Upcoming { current: r.track, tracks }));
            }
            Err(e) if r.retries < FUTURE_RETRIES => {
                log::info!("handoff: reading what follows the resumed track failed, retrying error={e}");
                r.retries += 1;
                r.at = Instant::now() + FUTURE_RETRY;
                self.future_due = Some(r);
            }
            Err(e) => log::warn!("handoff: gave up reading what follows the resumed track error={e}"),
        }
    }

    /// What follows the current entry of `queue`, from reads that agree on its revision.
    fn read_tail(queue: &str) -> Result<Tail> {
        let revision = wire::head(queue)?.revision;
        let (entries, tracks) = wire::future_tracks(queue, MAX_FUTURE)?;
        if wire::head(queue)?.revision != revision {
            return Err(CoreError::Network("the queue changed while what follows was read".into()));
        }
        Ok((entries, tracks, revision))
    }

    /// Selects the other device's queue again (after a failed send), then makes
    /// it active. Nothing is handed back while its queue isn't restored.
    fn hand_back(&mut self, to: &str) -> Result<()> {
        if let Some(q) = self.rollback_queue.clone() {
            self.checked(|| wire::select_queue(&q))?;
            self.mirror.queue_created(q);
            self.rollback_queue = None;
        }
        let active = self.checked(|| wire::set_active(to))?;
        let back = active.as_deref() == Some(to);
        self.mirror.claimed(active);
        // Its attachment lapsed meanwhile: tried again on a later read.
        if !back {
            return Err(CoreError::Other("the other device is not available yet".into()));
        }
        self.restore_playing()
    }

    /// The send paused the account; the player it took over was playing.
    /// Kept until written, so a failed write is retried.
    fn restore_playing(&mut self) -> Result<()> {
        if self.rollback_playing {
            self.checked(|| wire::set_status(true))?;
            self.rollback_playing = false;
            self.mirror.status_written(true);
        }
        Ok(())
    }

    /// A send that took playback from another device and then failed: that
    /// device gets its queue selected again and playback back.
    fn undo_send(&mut self, claimed: bool, before: Option<&str>, their_queue: Option<&str>, e: CoreError) -> CoreError {
        if !claimed {
            // This device kept playback: its own playing status (paused by the send) is written again.
            if self.restore_playing().is_err() {
                self.rollback_playing = false;
                self.pending.get_or_insert_with(|| Instant::now() + RETRY);
            }
            return e;
        }
        self.rollback_queue = their_queue.map(str::to_string);
        self.undo_claim(claimed, before, e)
    }

    /// A pull that claimed and then failed hands playback back to where it was.
    fn undo_claim(&mut self, claimed: bool, before: Option<&str>, e: CoreError) -> CoreError {
        let (true, Some(b)) = (claimed, before) else {
            self.rollback_queue = None;
            self.rollback_playing = false;
            return e;
        };
        // Only while this device still holds it: a newer claim by a third device
        // keeps playback and its queue. Unknown (a failed read): the next state read decides.
        match self.checked(wire::state) {
            // Still this device, nobody (an attachment lapsed), or the claim never
            // landed: hand back, which also restores the queue and playing status.
            Ok(s) if s.active.is_none() || s.active.as_deref() == Some(self.mirror.me()) || s.active.as_deref() == Some(b) => {
                if self.hand_back(b).is_err() {
                    self.rollback = Some(b.to_string());
                }
            }
            // A third device: it keeps playback and its queue.
            Ok(_) => {
                self.rollback_queue = None;
                self.rollback_playing = false;
            }
            Err(_) => self.rollback = Some(b.to_string()),
        }
        if self.rollback.is_some() {
            // Playback from before the pull is not a reason to keep it; a report after this one is.
            self.last_local = None;
            self.pending = None;
            self.dirty.get_or_insert_with(|| Instant::now() + RETRY);
        }
        e
    }

    fn continue_here(&mut self, deadline: Instant) -> Result<Resume> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        // Never continue from a remembered queue: it may be gone. A failed read is retried
        // (a pending arrival with it). What it read is the resume's first round.
        let (state, head) = match self.refresh() {
            Ok(read) => read,
            Err(e) => {
                self.dirty.get_or_insert_with(|| Instant::now() + RETRY);
                return Err(e);
            }
        };
        let from = self.mirror.state().active.as_deref().filter(|a| *a != self.mirror.me()).and_then(|a| self.mirror.device(a));
        // A pending arrival is what this pull takes over; kept if the pull fails, so it is retried.
        let arrival = self.arrival.take();
        let r = self.resume(Some(deadline), from, state.queue_id.clone().zip(head), state);
        if r.is_err() && arrival.is_some() && self.mirror.is_active() {
            self.arrival = arrival;
            self.dirty.get_or_insert_with(|| Instant::now() + RETRY);
        }
        r
    }

    fn send_to(&mut self, id: &str, deadline: Instant) -> Result<()> {
        if !self.same_account() {
            return Err(CoreError::NotSignedIn);
        }
        // Only before the first write: once started, the send runs to an answer.
        past(deadline)?;
        // Who owns playback and which queue is selected, as of now, not as last seen.
        self.refresh()?;
        // That read may have used up the time the caller waits: change nothing after it.
        past(deadline)?;
        // Playback just taken over is not loaded here yet: what reports would send is the old one.
        if self.awaiting.is_some() {
            return Err(CoreError::Other("still taking over playback".into()));
        }
        if self.last_local.as_ref().is_none_or(|l| l.current.is_none()) {
            return Err(CoreError::Other("nothing playing".into()));
        }
        // The player this claim displaces, handed back if the send fails before the hand-over.
        let before = self.mirror.state().active.clone().filter(|a| a != self.mirror.me());
        let claimed = self.mirror.state().active.as_deref() != Some(self.mirror.me());
        // Its queue stays as it was: this device writes into a queue of its own.
        let their_queue = self.mirror.queue_id().map(str::to_string).filter(|_| before.is_some());
        // And the playing status, restored if the send fails: that player's, or this
        // device's own when it was already the active player.
        self.rollback_playing = if before.is_some() {
            self.mirror.state().status == wire::Status::Playing
        } else {
            self.last_local.as_ref().is_some_and(|l| l.playing)
        };
        // A failed claim may still have reached TIDAL: undo it if it did.
        if claimed && let Err(e) = self.claim() {
            return Err(self.undo_send(claimed, before.as_deref(), None, e));
        }
        if their_queue.is_some() {
            match self.checked(wire::create_queue) {
                Ok(q) => self.mirror.queue_created(q),
                Err(e) => return Err(self.undo_send(claimed, before.as_deref(), their_queue.as_deref(), e)),
            }
        }
        if !self.write_local(true) {
            let e = CoreError::Other("could not write the queue".into());
            return Err(self.undo_send(claimed, before.as_deref(), their_queue.as_deref(), e));
        }
        // Paused before the target owns the state, so this write cannot land after its own.
        if let Err(e) = self.checked(|| wire::set_status(false)) {
            return Err(self.undo_send(claimed, before.as_deref(), their_queue.as_deref(), e));
        }
        self.mirror.status_written(false);
        // A failed call leaves the outcome unknown: read who is active now.
        let active = match self.checked(|| wire::set_active(id)) {
            Ok(a) => Ok(a),
            Err(e) => self.checked(wire::state).map(|s| s.active).map_err(|_unread| e),
        };
        let me = self.mirror.me().to_string();
        match active {
            Ok(Some(a)) if a == id => {}
            // An unattached target leaves no active player: hand playback back to
            // the device this send took it from, if any.
            Ok(a) if before.is_some() && (a.is_none() || a.as_deref() == Some(me.as_str())) => {
                let e = CoreError::Other("the device is not available".into());
                return Err(self.undo_send(claimed, before.as_deref(), their_queue.as_deref(), e));
            }
            // Otherwise take it back here and undo the pause.
            Ok(a) if a.is_none() || a.as_deref() == Some(me.as_str()) => {
                let playing = self.last_local.as_ref().is_some_and(|l| l.playing);
                let restored = self.claim().is_ok() && (!playing || wire::set_status(true).is_ok());
                if restored {
                    if playing {
                        self.mirror.status_written(true);
                    }
                } else {
                    self.ownership_unknown();
                }
                return Err(CoreError::Other("the device is not available".into()));
            }
            // A third device claimed meanwhile: it keeps playback, and the next
            // read reports the move so this device pauses.
            Ok(_) => {
                // Nothing more is written from here as the active player, and the
                // report from before the send must not claim playback back.
                self.ownership_unknown();
                self.pending = None;
                // Reports of this playback until the move pauses it claim nothing.
                self.sent = self.last_local.take().and_then(|l| l.current);
                self.mirror.still_playing();
                self.rollback_playing = false;
                self.rollback_queue = None;
                return Err(CoreError::Other("another device took over playback".into()));
            }
            // The target may have it or not: the next read tells, and a move pauses here.
            // The report from before the send waits for that read: written again if this
            // device turns out to be still active, dropped otherwise.
            Err(e) => {
                self.ownership_unknown();
                self.pending = None;
                self.rollback_playing = false;
                self.rollback_queue = None;
                self.quarantine_track = self.last_local.as_ref().and_then(|l| l.current.clone());
                self.quarantined = self.last_local.take().map(|l| (l, self.last_local_at));
                return Err(e);
            }
        }
        self.mirror.released_to(id);
        self.rollback_playing = false;
        self.rollback_queue = None;
        self.sent = self.last_local.as_ref().and_then(|l| l.current.clone());
        Ok(())
    }

    /// Who is active is unknown: nothing is written as the active player until
    /// a read settles it, and playing here claims first.
    fn ownership_unknown(&mut self) {
        self.mirror.ownership_unknown();
        self.dirty.get_or_insert_with(Instant::now);
    }

    /// Writes the last report; `sending` (right before a hand-over) also writes
    /// its position. `false` on failure, and then it is tried again later.
    fn write_local(&mut self, sending: bool) -> bool {
        let Some(local) = self.last_local.clone() else { return true };
        if !self.same_account() {
            return false;
        }
        // A failed pull still holds playback it is about to hand back: nothing
        // from here is written over the source's queue until that is settled.
        // The same while playback sent here is still being read: the old local
        // playback must not overwrite the queue that arrived.
        if self.rollback.is_some() || self.arrival.is_some() {
            self.pending.get_or_insert_with(|| Instant::now() + RETRY);
            return false;
        }
        if sending {
            self.mirror.force_position();
        }
        // Written later than reported (throttle, retries): where it plays now.
        let mut local = local;
        if local.playing {
            local.position_ms += i64::try_from(self.last_local_at.elapsed().as_millis()).unwrap_or(0);
        }
        let writes = self.mirror.plan(&local, now_ms());
        match self.apply(writes, &local, Instant::now(), sending) {
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

    /// `planned`: when `local`'s position was true; a playing position moves on
    /// by the time the writes before it took. `sending`: the queue is handed
    /// over right after, so only a rewritten queue's current entry is read back.
    fn apply(&mut self, writes: Vec<Write>, local: &LocalPlayback, planned: Instant, sending: bool) -> Result<()> {
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
                    let mut entries: Vec<wire::Entry> = head.current.into_iter().collect();
                    if !sending {
                        entries.extend(wire::future_entries(&q, MAX_FUTURE)?);
                    }
                    self.mirror.queue_written(tracks, entries, head.revision, local.repeat);
                }
                Write::Repeat(r) => {
                    let rev = wire::set_repeat(&self.queue()?, r)?;
                    self.mirror.repeat_written(r, rev);
                }
                Write::Current { index, position_ms } => {
                    let e = self.mirror.entry(index).ok_or_else(|| CoreError::Other("queue entry missing".into()))?;
                    let position_ms = if local.playing { position_ms + i64::try_from(planned.elapsed().as_millis()).unwrap_or(0) } else { position_ms };
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

    fn playback(current: &str, future: &[&str], position_ms: i64) -> LocalPlayback {
        LocalPlayback { current: Some(current.into()), future: future.iter().map(|t| (*t).to_string()).collect(), position_ms, playing: true, repeat: Repeat::Off }
    }

    #[test]
    fn a_takeover_counts_the_position_from_the_resume_read() {
        let read_at = Instant::now();
        let now = read_at + Duration::from_secs(12);
        // The wait restarted when what follows arrived, 10 s after the read.
        let t = Takeover { resumed: playback("1", &["2", "3"], 10_000), read_at, until: read_at + Duration::from_secs(10) + TAKEOVER };
        assert!(t.reported_by(&playback("1", &["2", "3"], 22_000), now));
        assert!(!t.reported_by(&playback("1", &["2", "3"], 40_000), now));
        assert!(!t.reported_by(&playback("1", &["2", "3"], 2_000), now));
    }

    #[test]
    fn a_takeover_waits_for_what_follows() {
        let read_at = Instant::now();
        let mut t = Takeover { resumed: playback("1", &[], 10_000), read_at, until: read_at + TAKEOVER };
        // Before the future is read, the current track alone is the resumed playback.
        assert!(t.reported_by(&playback("1", &[], 10_000), read_at));
        t.resumed.future = vec!["2".into()];
        assert!(!t.reported_by(&playback("1", &[], 10_000), read_at));
        assert!(t.reported_by(&playback("1", &["2"], 10_000), read_at));
        assert!(!t.reported_by(&playback("9", &["2"], 10_000), read_at));
    }

    struct Log(Mutex<Vec<String>>);
    impl HandoffListener for Log {
        fn on_event(&self, ev: HandoffEvent) {
            let line = match ev {
                HandoffEvent::Devices(d) => format!("devices {:?}", d.iter().map(|d| &d.name).collect::<Vec<_>>()),
                HandoffEvent::Elsewhere(e) => format!("elsewhere {:?}", e.map(|e| (e.device.name, e.track.title().to_string(), e.playing))),
                HandoffEvent::Arrived(r) => format!("arrived {} at {}", r.current.title(), r.position_ms),
                HandoffEvent::MovedTo(d) => format!("moved to {}", d.name),
                HandoffEvent::Upcoming(u) => format!("upcoming after {}: {}", u.current, u.tracks.iter().map(Track::id).collect::<Vec<_>>().join(",")),
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
        assert!(r.position_ms >= 30_000, "position {}", r.position_ms);
        wait(&lb, &format!("upcoming after {}: {}", ids[0], ids[1..].join(",")));
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
        wait(&la, &format!("upcoming after {}: {}", ids[1], ids[2..].join(",")));
        let got = la.0.lock().unwrap_or_else(PoisonError::into_inner).iter().find(|l| l.starts_with("arrived")).cloned().unwrap_or_default();
        // Reported playing at 12 345 ms about 2 s before the send: written where it plays then.
        let at: i64 = got.rsplit(' ').next().and_then(|n| n.parse().ok()).unwrap_or(0);
        assert!((12_345..16_000).contains(&at), "{got}");
        a.stop();
        b.stop();
        assert!(wait_detached(Duration::from_secs(10)), "not detached");
    }
}

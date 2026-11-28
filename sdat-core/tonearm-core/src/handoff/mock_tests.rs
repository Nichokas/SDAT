use std::io::{BufRead, BufReader, Read, Write as _};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;

use serde_json::{Value, json};

use super::*;
use crate::auth::{self, TokenStore};
use crate::config::{self, Config};

struct Local(String);

impl Config for Local {
    fn api_base_url(&self) -> String {
        self.0.clone()
    }
    fn auth_base_url(&self) -> String {
        self.0.clone()
    }
    fn openapi_base_url(&self) -> String {
        self.0.clone()
    }
}

struct SignedIn;

impl TokenStore for SignedIn {
    fn has_refresh_token(&self) -> bool {
        true
    }
    fn get_refresh_token(&self) -> String {
        "mock-refresh".into()
    }
    fn user_id(&self) -> String {
        "mock-user".into()
    }
}

type Response = (&'static str, Value);
const OK: &str = "200 OK";
const MISSING: &str = "404 Not Found";
const UNAVAILABLE: &str = "503 Service Unavailable";

struct Account {
    active: Option<&'static str>,
    writer: &'static str,
    queue_id: String,
    current: String,
    item: String,
    status: &'static str,
    repeat: &'static str,
    revision: u32,
    position_at_ms: i64,
    missing: bool,
    fail_after_claim: bool,
    fail_rollback_state: bool,
    seek_before_claim: bool,
    fail_send_with_queue: Option<&'static str>,
    fail_queue_restore: bool,
    unavailable: Option<&'static str>,
    rewrites: usize,
    claims: Vec<String>,
    positions: Vec<String>,
}

impl Account {
    fn new() -> Self {
        Self {
            active: Some("source"),
            writer: "source",
            queue_id: "queue".into(),
            current: "1".into(),
            item: "i1".into(),
            status: "PAUSED",
            repeat: "ONE",
            revision: 1,
            position_at_ms: now_ms() - 60_000,
            missing: false,
            fail_after_claim: false,
            fail_rollback_state: false,
            seek_before_claim: false,
            fail_send_with_queue: None,
            fail_queue_restore: false,
            unavailable: None,
            rewrites: 0,
            claims: Vec::new(),
            positions: Vec::new(),
        }
    }

    fn push(&mut self) {
        self.active = Some("receiver");
        self.writer = "source";
        self.revision += 1;
        self.position_at_ms = now_ms();
    }

    fn answer(&mut self, method: &str, path: &str, body: &Value) -> Response {
        match (method, path) {
            ("POST", "/v1/oauth2/token") => (OK, json!({"access_token":"mock-token","expires_in":3600})),
            ("GET", "/v1/sessions") => (OK, json!({"countryCode":"ES"})),
            ("GET", "/v2/userPlaybackStates/me") if self.active == Some("receiver") && self.fail_rollback_state => {
                self.fail_rollback_state = false;
                (UNAVAILABLE, json!({}))
            }
            ("GET", "/v2/userPlaybackStates/me") => (
                OK,
                json!({"data":{
                    "id":"mock-user", "type":"userPlaybackStates", "attributes":{"playbackStatus":self.status},
                    "relationships":{
                        "activePlayer":{"data":self.active.map(|id| json!({"id":id,"type":"installations"}))},
                        "availablePlayers":{"data":[{"id":"source","type":"installations"},{"id":"receiver","type":"installations"}]},
                        "playQueue":{"data":{"id":self.queue_id,"type":"playQueues"}}
                    }
                }}),
            ),
            ("GET", "/v2/playQueues/queue") if self.active == Some("receiver") && self.fail_after_claim => {
                self.fail_after_claim = false;
                (UNAVAILABLE, json!({}))
            }
            ("GET", "/v2/playQueues/queue") => (
                OK,
                json!({"data":{
                "id":"queue","type":"playQueues","attributes":{"repeat":self.repeat,"revision":self.revision.to_string()},
                "relationships":{"current":{"data":{
                    "id":self.current,"type":"tracks","meta":{
                        "itemId":self.item,
                        "position":"PT10S",
                        "positionUpdatedAt":chrono::DateTime::from_timestamp_millis(self.position_at_ms).expect("position time").to_rfc3339(),
                        "updatedByInstallationId":self.writer
                    }
                }}}
            }, "included":if self.missing { vec![] } else { vec![track(&self.current)] }}),
            ),
            ("GET", "/v2/playQueues/queue/relationships/future") => {
                // Track 1 appears again later as a *different* queue occurrence.
                let data = if self.repeat == "ONE" {
                    vec![entry("1", "i1")]
                } else {
                    vec![entry("2", "i2"), entry("1", "i3")]
                };
                (OK, json!({"data":data,"included":[track("1"),track("2")]}))
            }
            ("GET", "/v2/tracks/1") => (MISSING, json!({})),
            ("POST", "/v2/playQueues") => {
                self.queue_id = "send-queue".into();
                (OK, json!({"data":{"id":self.queue_id}}))
            }
            ("PATCH", "/v2/userPlaybackStates/me/relationships/playQueue") => {
                let id = body["data"]["id"].as_str().expect("queue id");
                if id == "queue" && self.fail_queue_restore {
                    self.fail_queue_restore = false;
                    return (UNAVAILABLE, json!({}));
                }
                self.queue_id = id.into();
                (OK, json!({}))
            }
            ("PATCH", "/v2/playQueues/send-queue") => {
                self.active = Some("source");
                self.queue_id = self.fail_send_with_queue.take().expect("queue after failed send").into();
                (UNAVAILABLE, json!({}))
            }
            ("PATCH", "/v2/userPlaybackStates/me/relationships/activePlayer") => {
                let to = body["data"]["id"].as_str().expect("target");
                self.claims.push(to.into());
                if to == "receiver" && self.seek_before_claim {
                    self.revision += 1;
                    self.position_at_ms = now_ms();
                    self.seek_before_claim = false;
                }
                self.active = if Some(to) == self.unavailable {
                    None
                } else if to == "receiver" {
                    Some("receiver")
                } else {
                    Some("source")
                };
                (OK, json!({"data":self.active.map(|id| json!({"id":id}))}))
            }
            ("PATCH", "/v2/userPlaybackStates/me") => {
                self.status = if body["data"]["attributes"]["playbackStatus"] == "PLAYING" {
                    "PLAYING"
                } else {
                    "PAUSED"
                };
                (OK, json!({}))
            }
            ("PATCH", "/v2/playQueues/queue") => {
                let repeat = body["data"]["attributes"]["repeat"].as_str();
                self.repeat = match repeat {
                    Some("ONE") => "ONE",
                    Some("BATCH") => "BATCH",
                    _ => "NONE",
                };
                self.changed()
            }
            ("POST", "/v2/playQueues/queue/relationships/future") => {
                self.rewrites += 1;
                self.current = body["data"][0]["id"].as_str().expect("new current").into();
                self.item = format!("i{}", self.current);
                self.missing = false;
                self.changed()
            }
            ("PATCH", "/v2/playQueues/queue/relationships/current") => {
                self.current = body["data"]["id"].as_str().expect("track").into();
                self.item = body["data"]["meta"]["itemId"].as_str().expect("item id").into();
                self.positions.push(self.item.clone());
                self.writer = "receiver";
                self.changed()
            }
            _ => panic!("unexpected mock request: {method} {path}"),
        }
    }

    fn changed(&mut self) -> Response {
        self.revision += 1;
        (OK, json!({"data":{"attributes":{"revision":self.revision.to_string()}}}))
    }
}

fn entry(id: &str, item: &str) -> Value {
    json!({"id":id,"type":"tracks","meta":{"itemId":item}})
}

fn track(id: &str) -> Value {
    json!({"id":id,"type":"tracks","attributes":{"title":"Song","duration":"PT3M"}})
}

fn serve(mut stream: TcpStream, account: &Mutex<Account>) {
    let mut request = BufReader::new(&mut stream);
    let mut line = String::new();
    request.read_line(&mut line).expect("request line");
    let mut length = 0;
    loop {
        let mut header = String::new();
        request.read_line(&mut header).expect("request header");
        if header.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':')
            && name.eq_ignore_ascii_case("content-length")
        {
            length = value.trim().parse().expect("content length");
        }
    }
    let mut body = vec![0; length];
    request.read_exact(&mut body).expect("request body");
    let mut line = line.split_whitespace();
    let method = line.next().expect("method");
    let path = line.next().expect("path").split('?').next().expect("path without query");
    let body = serde_json::from_slice(&body).unwrap_or(Value::Null);
    let (status, response) = account.lock().expect("account").answer(method, path, &body);
    let response = response.to_string();
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{response}",
        response.len()
    )
    .expect("mock response");
}

impl HandoffListener for Sender<HandoffEvent> {
    fn on_event(&self, event: HandoffEvent) {
        self.send(event).expect("event receiver");
    }
}

fn setup() -> (&'static Mutex<Account>, Worker, Receiver<HandoffEvent>) {
    static ACCOUNT: OnceLock<Mutex<Account>> = OnceLock::new();
    let account = ACCOUNT.get_or_init(|| {
        let listener = TcpListener::bind("127.0.0.1:0").expect("mock TIDAL");
        let address = listener.local_addr().expect("address");
        assert!(config::install(Box::new(Local(format!("http://{address}")))));
        assert!(auth::install(Box::new(SignedIn)));
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                serve(stream.expect("connection"), ACCOUNT.get().expect("account"));
            }
        });
        Mutex::new(Account::new())
    });
    *account.lock().expect("account") = Account::new();
    let (events, rx) = mpsc::channel();
    let worker = Worker {
        mirror: Mirror::new("receiver".into()),
        account: "mock-user".into(),
        listener: Arc::new(events),
        devices: Arc::default(),
        tx: mpsc::channel().0,
        streams: Vec::new(),
        last_local: None,
        last_local_at: Instant::now(),
        pending: None,
        backoff: RETRY,
        dirty: None,
        awaiting: None,
        future_due: None,
        held: None,
        before_takeover: None,
        rollback: None,
        quarantined: None,
        quarantine_track: None,
        sent: None,
        arrival: None,
        names: HashMap::new(),
    };
    (account, worker, rx)
}

fn deadline() -> Instant {
    Instant::now() + CALL_TIMEOUT
}

#[test]
fn handoff_against_mock_service() {
    const CHILD: &str = "SDAT_HANDOFF_MOCK_TEST_CHILD";
    // Configuration and the token store can only be installed once per process.
    if std::env::var_os(CHILD).is_none() {
        let run = std::process::Command::new(std::env::current_exe().expect("test executable"))
            .args(["--exact", concat!(module_path!(), "::handoff_against_mock_service").split_once("::").expect("test module").1])
            .env(CHILD, "1")
            .output()
            .expect("isolated handoff test");
        assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stdout));
        assert!(String::from_utf8_lossy(&run.stdout).contains("1 passed"), "{}", String::from_utf8_lossy(&run.stdout));
        return;
    }
    missing_pushed_track_releases_local_writes();
    pulling_from_a_paused_device_cannot_restore_a_previous_sends_playing_status();
    failed_send_restores_only_its_own_replacement_queue();
    a_rollback_does_not_undo_a_new_push_on_the_same_queue_from_the_same_source();
    continue_here_delivers_a_discovered_push_only_in_its_reply();
    a_failed_pull_hands_back_when_the_same_queue_is_still_selected();
    a_failed_pull_can_finish_on_the_receiver_after_a_concurrent_source_seek();
    repeat_one_does_not_append_the_current_occurrence();
}

fn missing_pushed_track_releases_local_writes() {
    let (account, mut worker, events) = setup();
    worker.refresh().expect("initial state");
    {
        let mut account = account.lock().expect("account");
        account.missing = true;
        account.push();
    }
    worker.refresh().expect("pushed state");
    assert!(!events.try_iter().any(|event| matches!(event, HandoffEvent::Arrived(_))));
    worker.reported(
        LocalPlayback {
            current: Some("2".into()),
            playing: true,
            ..LocalPlayback::default()
        },
        Instant::now(),
    );
    assert!(worker.write_local(false), "stale arrival blocked the new local playback");
    {
        let account = account.lock().expect("account");
        assert_eq!(account.current, "2");
        assert_eq!(account.status, "PLAYING");
    }
    worker.send_to("source", deadline()).expect("send after replacing the missing track");
    assert_eq!(account.lock().expect("account").active, Some("source"));
}

fn pulling_from_a_paused_device_cannot_restore_a_previous_sends_playing_status() {
    let (account, mut worker, _events) = setup();
    {
        let mut account = account.lock().expect("account");
        account.active = Some("receiver");
        account.writer = "receiver";
        account.unavailable = Some("missing");
    }
    worker.refresh().expect("initial state");
    worker.reported(
        LocalPlayback {
            current: Some("1".into()),
            playing: true,
            ..LocalPlayback::default()
        },
        Instant::now(),
    );
    assert!(worker.send_to("missing", deadline()).is_err());
    assert_eq!(account.lock().expect("account").status, "PLAYING");
    {
        let mut account = account.lock().expect("account");
        account.active = Some("source");
        account.writer = "source";
        account.status = "PAUSED";
        account.fail_after_claim = true;
    }
    assert!(worker.continue_here(deadline()).is_err());
    let account = account.lock().expect("account");
    assert_eq!(account.active, Some("source"));
    assert_eq!(account.status, "PAUSED");
}

fn failed_send_restores_only_its_own_replacement_queue() {
    for (selected, fail_restore, expected) in [
        ("send-queue", false, "queue"),
        ("send-queue", true, "queue"),
        ("newer-queue", false, "newer-queue"),
    ] {
        let (account, mut worker, _events) = setup();
        {
            let mut account = account.lock().expect("account");
            account.fail_send_with_queue = Some(selected);
            account.fail_queue_restore = fail_restore;
        }
        worker.reported(
            LocalPlayback { current: Some("2".into()), playing: true, ..LocalPlayback::default() },
            Instant::now(),
        );
        assert!(worker.send_to("target", deadline()).is_err());
        if fail_restore {
            assert_eq!(account.lock().expect("account").queue_id, "send-queue");
            worker.refresh().expect("retry queue restoration");
        }
        let account = account.lock().expect("account");
        assert_eq!(account.queue_id, expected);
        assert_eq!(account.active, Some("source"));
    }
}

fn a_rollback_does_not_undo_a_new_push_on_the_same_queue_from_the_same_source() {
    for fail_rollback_state in [false, true] {
        let (account, mut worker, events) = setup();
        worker.refresh().expect("initial state");
        {
            let mut account = account.lock().expect("account");
            account.fail_after_claim = true;
            account.fail_rollback_state = fail_rollback_state;
            account.unavailable = Some("source");
        }
        assert!(worker.continue_here(deadline()).is_err());
        let expected_owner = fail_rollback_state.then_some("receiver");
        assert_eq!(account.lock().expect("account").active, expected_owner);
        {
            let mut account = account.lock().expect("account");
            account.unavailable = None;
            let revision = account.revision;
            account.push();
            account.revision = revision;
        }
        worker.refresh().expect("new push");
        assert!(events.try_iter().any(|event| matches!(event, HandoffEvent::Arrived(_))));
        let account = account.lock().expect("account");
        assert_eq!(account.active, Some("receiver"));
        let expected_claims = if fail_rollback_state { vec!["receiver"] } else { vec!["receiver", "source"] };
        assert_eq!(account.claims, expected_claims);
    }
}

fn continue_here_delivers_a_discovered_push_only_in_its_reply() {
    let (account, mut worker, events) = setup();
    worker.refresh().expect("initial state");
    account.lock().expect("account").push();
    let resume = worker.continue_here(deadline()).expect("take the pushed queue");
    assert_eq!(resume.current.id(), "1");
    assert_eq!(resume.position_ms, 10_000);
    assert!(!events.try_iter().any(|event| matches!(event, HandoffEvent::Arrived(_))));
    assert!(account.lock().expect("account").claims.is_empty());
}

fn a_failed_pull_hands_back_when_the_same_queue_is_still_selected() {
    let (account, mut worker, events) = setup();
    {
        let mut account = account.lock().expect("account");
        account.fail_after_claim = true;
        account.unavailable = Some("source");
    }
    assert!(worker.continue_here(deadline()).is_err());
    assert_eq!(account.lock().expect("account").active, None);
    account.lock().expect("account").unavailable = None;
    worker.refresh().expect("retry hand back");
    assert!(!events.try_iter().any(|event| matches!(event, HandoffEvent::Arrived(_))));
    let account = account.lock().expect("account");
    assert_eq!(account.active, Some("source"));
    assert_eq!(account.claims, ["receiver", "source", "source"]);
}

fn a_failed_pull_can_finish_on_the_receiver_after_a_concurrent_source_seek() {
    for fail_rollback_state in [false, true] {
        let (account, mut worker, events) = setup();
        {
            let mut account = account.lock().expect("account");
            account.seek_before_claim = true;
            account.fail_after_claim = true;
            account.fail_rollback_state = fail_rollback_state;
        }
        assert!(worker.continue_here(deadline()).is_err());
        worker.refresh().expect("recover the changed queue");
        worker.refresh().expect("refresh after recovering");
        let arrivals = events.try_iter().filter(|event| matches!(event, HandoffEvent::Arrived(resume) if resume.current.id() == "1")).count();
        assert_eq!(arrivals, 1);
        let account = account.lock().expect("account");
        assert_eq!(account.active, Some("receiver"));
        assert_eq!(account.claims, ["receiver"]);
    }
}

fn repeat_one_does_not_append_the_current_occurrence() {
    let (account, mut worker, events) = setup();
    let resume = worker.continue_here(deadline()).expect("pull resume");
    assert_eq!(resume.repeat, Repeat::One);
    let future = worker.future_due.take().expect("scheduled tail");
    worker.read_future(future);
    assert!(events.try_iter().any(|event| matches!(event, HandoffEvent::Upcoming(upcoming) if upcoming.tracks.is_empty())));
    assert_eq!(account.lock().expect("account").repeat, "ONE");
}

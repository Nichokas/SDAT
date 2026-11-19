//! The device-login poll survives a run of failed sends, as when Android holds back
//! network for an app that sits behind the browser while the user approves the code.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

struct Local(String);
impl tonearm_core::config::Config for Local {
    fn auth_base_url(&self) -> String {
        self.0.clone()
    }
}

#[test]
fn poll_retries_failed_sends_and_respects_slow_down_until_approved() {
    let server = TcpListener::bind("127.0.0.1:0").unwrap();
    tonearm_core::config::install(Box::new(Local(format!("http://{}", server.local_addr().unwrap()))));
    let responses = std::thread::spawn(move || {
        let mut received = Vec::new();
        for (i, stream) in server.incoming().enumerate() {
            let mut s = stream.unwrap();
            // More dropped connections in a row than the old limit of five.
            if i < 7 {
                continue;
            }
            let mut buf = [0u8; 4096];
            let _ = s.read(&mut buf);
            received.push(Instant::now());
            let (status, body) = match i {
                7 => ("400 Bad Request", r#"{"error":"slow_down"}"#),
                8 => ("400 Bad Request", r#"{"error":"authorization_pending"}"#),
                _ => ("200 OK", r#"{"access_token":"a","clientName":"c","expires_in":1,"refresh_token":"r","scope":"s","token_type":"Bearer","user":{},"user_id":42}"#),
            };
            let _ = write!(s, "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            if i == 9 { break; }
        }
        received
    });
    let r = tonearm_core::api::poll_device_linking_status("code", 1, 300, &|| false).unwrap();
    assert_eq!((r.refresh_token.as_str(), r.user_id), ("r", 42));
    let sent = responses.join().unwrap();
    assert_eq!(sent.len(), 3);
    assert!(sent.windows(2).all(|pair| pair[1].duration_since(pair[0]) >= Duration::from_secs(6)));
}


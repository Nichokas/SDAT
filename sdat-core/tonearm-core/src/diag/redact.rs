//! Strips credentials from text before it is stored, written or sent anywhere.

use std::borrow::Cow;

const REDACTED: &str = "<redacted>";
const KEYS: [&str; 5] = ["access_token", "refresh_token", "client_secret", "password", "code"];

fn is_delimiter(b: u8) -> bool {
    b.is_ascii_whitespace() || matches!(b, b'"' | b'\'' | b'\\' | b'&' | b',' | b';' | b')' | b']' | b'}' | b'<' | b'>')
}

fn skip(s: &[u8], mut i: usize, f: impl Fn(u8) -> bool) -> usize {
    while i < s.len() && f(s[i]) {
        i += 1;
    }
    i
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

/// A quoted value whose opening quote is at `i` (possibly escaped as `\"` inside a debug-printed
/// string): its range without the quotes.
fn quoted(s: &[u8], i: usize) -> Option<(usize, usize)> {
    let escaped = s[i] == b'\\';
    let open = if escaped { i + 1 } else { i };
    if s.get(open) != Some(&b'"') {
        return None;
    }
    let start = open + 1;
    let mut end = start;
    while end < s.len() {
        match s[end] {
            b'"' => break,
            b'\\' if escaped => break,
            b'\\' => end += 2,
            _ => end += 1,
        }
    }
    let end = end.min(s.len());
    (start < end).then_some((start, end))
}

/// `key`, an optional closing quote, then `=` or `:` and the value. Only `key=value` takes a bare value,
/// so prose such as "error code: 401" and JSON numbers such as `"code":401` stay readable.
fn key_value(s: &[u8], after_key: usize) -> Option<(usize, usize)> {
    let mut i = after_key;
    let quoted_key = matches!(s.get(i), Some(b'"' | b'\\'));
    if quoted_key {
        i = skip(s, i, |b| b == b'\\');
        i += usize::from(s.get(i) == Some(&b'"'));
    }
    i = skip(s, i, is_blank);
    let sep = *s.get(i)?;
    if sep != b'=' && sep != b':' {
        return None;
    }
    i = skip(s, i + 1, is_blank);
    if matches!(s.get(i), Some(b'"' | b'\\')) {
        return quoted(s, i);
    }
    if sep == b':' || quoted_key {
        return None;
    }
    let end = skip(s, i, |b| !is_delimiter(b));
    (end > i).then_some((i, end))
}

/// `Authorization: <scheme> <token>`: the whole value, to the end of the line unless quoted.
fn header_value(s: &[u8], after_key: usize) -> Option<(usize, usize)> {
    let mut i = skip(s, after_key, |b| matches!(b, b'"' | b'\\'));
    i = skip(s, i, is_blank);
    if !matches!(s.get(i), Some(b':' | b'=')) {
        return None;
    }
    i = skip(s, i + 1, is_blank);
    if matches!(s.get(i), Some(b'"' | b'\\')) {
        return quoted(s, i);
    }
    let end = skip(s, i, |b| b != b'\n' && b != b'\r');
    (end > i).then_some((i, end))
}

fn secret_at(s: &[u8], lower: &[u8], i: usize) -> Option<(usize, usize)> {
    let rest = &lower[i..];
    if rest.starts_with(b"bearer ") {
        let start = i + "bearer ".len();
        let end = skip(s, start, |b| !is_delimiter(b));
        return (end > start).then_some((start, end));
    }
    if rest.starts_with(b"authorization") {
        return header_value(s, i + "authorization".len());
    }
    let key = KEYS.iter().find(|k| rest.starts_with(k.as_bytes()))?;
    if *key == "code" && i > 0 && (s[i - 1].is_ascii_alphanumeric() || s[i - 1] == b'_') {
        return None;
    }
    key_value(s, i + key.len())
}

/// Replaces the value of every credential in `text` with `<redacted>`: `access_token`, `refresh_token`,
/// `client_secret`, `code` and `password` in query strings, form bodies, JSON and debug output, plus
/// `Bearer` tokens and `Authorization` headers.
pub(super) fn redact(text: &str) -> Cow<'_, str> {
    let lower = text.to_ascii_lowercase();
    let (s, l) = (text.as_bytes(), lower.as_bytes());
    let mut out = String::new();
    let (mut copied, mut i) = (0, 0);
    while i < s.len() {
        if let Some((start, end)) = secret_at(s, l, i) {
            out.push_str(&text[copied..start]);
            out.push_str(REDACTED);
            copied = end;
            i = end;
        } else {
            i += 1;
        }
    }
    if copied == 0 {
        return Cow::Borrowed(text);
    }
    out.push_str(&text[copied..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn query_strings_and_form_bodies() {
        assert_eq!(redact("GET /cb?code=abc123&state=x"), "GET /cb?code=<redacted>&state=x");
        assert_eq!(
            redact("grant_type=refresh_token&refresh_token=eyJ.a-b_c&client_id=1"),
            "grant_type=refresh_token&refresh_token=<redacted>&client_id=1"
        );
        assert_eq!(redact("client_secret=s3cr3t password=hunter2"), "client_secret=<redacted> password=<redacted>");
        assert_eq!(redact("ACCESS_TOKEN=Tok"), "ACCESS_TOKEN=<redacted>");
    }

    #[test]
    fn json_bodies() {
        assert_eq!(
            redact(r#"{"access_token":"eyJ.x.y","expires_in":3600,"refresh_token": "r\"t"}"#),
            r#"{"access_token":"<redacted>","expires_in":3600,"refresh_token": "<redacted>"}"#
        );
        assert_eq!(redact(r#"{"password":"p w"}"#), r#"{"password":"<redacted>"}"#);
    }

    #[test]
    fn json_inside_a_debug_printed_string() {
        assert_eq!(
            redact(r#"body: "{\"access_token\":\"abc\",\"ok\":true}""#),
            r#"body: "{\"access_token\":\"<redacted>\",\"ok\":true}""#
        );
    }

    #[test]
    fn bearer_tokens_and_authorization_headers() {
        assert_eq!(redact("sent Bearer eyJ.abc.def to the server"), "sent Bearer <redacted> to the server");
        assert_eq!(redact("authorization: Token abc123\nnext"), "authorization: <redacted>\nnext");
        assert_eq!(redact("Authorization=Bearer abc"), "Authorization=<redacted>");
        assert_eq!(redact(r#"{"Authorization":"Bearer abc"}"#), r#"{"Authorization":"<redacted>"}"#);
    }

    #[test]
    fn readable_text_is_left_alone() {
        for s in [
            "error code: 401 from the server",
            r#"{"code":401,"message":"nope"}"#,
            "status_code=500 zipcode=12345 encoded",
            "no secrets here, only a passcode",
            "bearer_auth was called",
            "token=",
        ] {
            assert_eq!(redact(s), s);
        }
    }

    #[test]
    fn redacting_twice_changes_nothing() {
        let once = redact("code=abc&password=x Bearer tok").into_owned();
        assert_eq!(redact(&once), once);
    }

    #[test]
    fn non_ascii_text_survives() {
        assert_eq!(redact("Müll code=äö&x=ü"), "Müll code=<redacted>&x=ü");
    }
}

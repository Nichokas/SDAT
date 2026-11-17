//! Strips credentials from text before it is stored, written or sent anywhere.

use std::borrow::Cow;

const REDACTED: &str = "<redacted>";
/// `Authorization` schemes: a value on the line after `Authorization:` is taken only when it starts with one.
const SCHEMES: [&str; 4] = ["basic ", "bearer ", "digest ", "token "];
/// Each `_` is optional when matching, so the camelCase spellings (`refreshToken`, `deviceCode`) match too.
const KEYS: [&str; 12] = [
    "access_token",
    "refresh_token",
    "client_secret",
    "session_key",
    "device_code",
    "user_code",
    "password",
    "api_key",
    "api_sig",
    "token",
    "code",
    "sk",
];
/// Short keys that also end longer words (`zipcode`, `disk`, `x_tidal_token`): matched only at a word start.
const WORD_KEYS: [&str; 3] = ["token", "code", "sk"];

/// The end of a bare value starting at `i`: whitespace or `&`. Any other character may be part of a
/// password, so quotes and punctuation are redacted with it, even when they closed the text around it.
fn bare_end(s: &[u8], i: usize) -> usize {
    skip(s, i, |b| !b.is_ascii_whitespace() && b != b'&')
}

fn is_blank(b: u8) -> bool {
    b == b' ' || b == b'\t'
}

/// Whitespace, and the escaped `\n`, `\r` and `\t` of a debug-printed string.
fn skip_json_space(s: &[u8], mut i: usize) -> usize {
    loop {
        match (s.get(i), s.get(i + 1)) {
            (Some(b), _) if b.is_ascii_whitespace() => i += 1,
            (Some(b'\\'), Some(b'n' | b'r' | b't')) => i += 2,
            _ => return i,
        }
    }
}

fn skip(s: &[u8], mut i: usize, f: impl Fn(u8) -> bool) -> usize {
    while i < s.len() && f(s[i]) {
        i += 1;
    }
    i
}

/// A quoted value whose opening quote (`"` or `'`) is at `i`: its range without the quotes. Inside a debug-printed
/// string the quote is `\"` and every character of the value is escaped once more, so one level of
/// escaping is undone before looking for the value's own closing quote. A closing quote that something
/// other than whitespace or punctuation follows may be inside the password: the rest is redacted as a bare value.
fn quoted(s: &[u8], i: usize) -> Option<(usize, usize)> {
    let escaped = s[i] == b'\\';
    let open = if escaped { i + 1 } else { i };
    let quote = *s.get(open)?;
    if quote != b'"' && (escaped || quote != b'\'') {
        return None;
    }
    let start = open + 1;
    let (mut end, mut in_escape) = (start, false);
    while end < s.len() {
        let (c, width) = match (escaped, s[end], s.get(end + 1)) {
            (true, b'\\', Some(&next)) => (next, 2),
            (_, c, _) => (c, 1),
        };
        if c == quote && !in_escape {
            break;
        }
        in_escape = c == b'\\' && !in_escape;
        end += width;
    }
    let end = end.min(s.len());
    let after = end + if escaped { 2 } else { 1 };
    let closes = s.get(after).is_none_or(|&b| b.is_ascii_whitespace() || b",;:)]}&>\\".contains(&b));
    let end = if closes { end } else { bare_end(s, after) };
    (start < end).then_some((start, end))
}

/// `key`, an optional closing quote, then `=` or `:` and the value. A bare `code` is taken only from
/// `code=value`, so prose such as "error code: 401" and JSON numbers such as `"code":401` stay readable.
fn key_value(s: &[u8], after_key: usize, is_code: bool) -> Option<(usize, usize)> {
    let mut i = after_key;
    let quoted_key = matches!(s.get(i), Some(b'"' | b'\'' | b'\\'));
    if quoted_key {
        i = skip(s, i, |b| b == b'\\');
        i += usize::from(matches!(s.get(i), Some(b'"' | b'\'')));
    }
    i = skip(s, i, |b| b.is_ascii_whitespace());
    let sep = *s.get(i)?;
    if sep != b'=' && sep != b':' {
        return None;
    }
    // JSON may put a value on the next line, also escaped as `\n` in a debug-printed string; after `password=`
    // a new line is the next message, not the value.
    i = if quoted_key { skip_json_space(s, i + 1) } else { skip(s, i + 1, is_blank) };
    if matches!(s.get(i), Some(b'"' | b'\'' | b'\\'))
        && let Some((start, end)) = quoted(s, i)
    {
        // After an unquoted key (a query string, a form body) the quotes may be part of the password itself.
        return Some((start, if quoted_key { end } else { end.max(bare_end(s, i)) }));
    }
    if is_code && (sep == b':' || quoted_key) {
        return None;
    }
    let mut end = bare_end(s, i);
    if quoted_key {
        // A JSON number, `true` or `null` ends where the object or array goes on.
        end = i + s[i..end].iter().position(|b| matches!(b, b',' | b'}' | b']')).unwrap_or(end - i);
    }
    // A JSON flag such as `"hasAccessToken":true` says whether there is a secret, not what it is; a password
    // would be the string `"true"`. After `password=` the same word may be the password itself.
    if quoted_key && matches!(&s[i..end], b"true" | b"false" | b"null") {
        return None;
    }
    (end > i).then_some((i, end))
}

/// `Authorization: <scheme> <token>`: the whole value, to the end of the line unless quoted.
fn header_value(s: &[u8], after_key: usize) -> Option<(usize, usize)> {
    let mut i = skip(s, after_key, |b| matches!(b, b'"' | b'\'' | b'\\'));
    i = skip(s, i, |b| b.is_ascii_whitespace());
    if !matches!(s.get(i), Some(b':' | b'=')) {
        return None;
    }
    let value = skip(s, i + 1, |b| b.is_ascii_whitespace());
    if matches!(s.get(value), Some(b'"' | b'\'' | b'\\'))
        && let Some(range) = quoted(s, value)
    {
        return Some(range);
    }
    let has_scheme =
        SCHEMES.iter().any(|scheme| s.get(value..value + scheme.len()).is_some_and(|p| p.eq_ignore_ascii_case(scheme.as_bytes())));
    i = if has_scheme { value } else { skip(s, i + 1, is_blank) };
    let end = skip(s, i, |b| b != b'\n' && b != b'\r');
    (end > i).then_some((i, end))
}

fn secret_at(s: &[u8], lower: &[u8], i: usize) -> Option<(usize, usize)> {
    let rest = &lower[i..];
    if rest.starts_with(b"bearer ") {
        let start = i + "bearer ".len();
        let end = bare_end(s, start);
        return (end > start).then_some((start, end));
    }
    if rest.starts_with(b"authorization") {
        return header_value(s, i + "authorization".len());
    }
    let (key, len) = KEYS.iter().find_map(|k| key_len(rest, k).map(|len| (*k, len)))?;
    if WORD_KEYS.contains(&key) && i > 0 && (lower[i - 1].is_ascii_alphanumeric() || lower[i - 1] == b'_') {
        return None;
    }
    key_value(s, i + len, key == "code")
}

/// How many bytes of `text` spell `key`, whose `_` may be left out.
fn key_len(text: &[u8], key: &str) -> Option<usize> {
    let mut i = 0;
    for &k in key.as_bytes() {
        match text.get(i) {
            Some(&b) if b == k => i += 1,
            _ if k == b'_' => {}
            _ => return None,
        }
    }
    Some(i)
}

/// Replaces the value of every credential in `text` with `<redacted>`: `access_token`, `refresh_token`,
/// `client_secret`, `code` and `password` in query strings, form bodies, JSON and debug output, plus
/// `Bearer` tokens and `Authorization` headers.
pub(super) fn redact(text: &str) -> Cow<'_, str> {
    // `-` and `.` read as `_`, so `client-secret` and `api.key` match too; byte offsets stay the same.
    let lower: Vec<u8> = text.bytes().map(|b| if matches!(b, b'-' | b'.') { b'_' } else { b.to_ascii_lowercase() }).collect();
    let (s, l) = (text.as_bytes(), lower.as_slice());
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
        assert_eq!(redact(r#"{"password":123456,"x":1}"#), r#"{"password":<redacted>,"x":1}"#);
        assert_eq!(redact(r#"[{"sk": 42}]"#), r#"[{"sk": <redacted>}]"#);
        assert_eq!(redact("password: hunter2 next"), "password: <redacted> next");
        assert_eq!(redact("{\"password\":\n  123456}"), "{\"password\":\n  <redacted>}");
        assert_eq!(redact("Authorization:\nToken secret\nnext"), "Authorization:\n<redacted>\nnext");
        assert_eq!(redact("password=hunter2"), "password=<redacted>");
        assert_eq!(redact("password=\nstatus: ok"), "password=\nstatus: ok");
        assert_eq!(redact("password=\n\"next message\""), "password=\n\"next message\"");
        assert_eq!(
            redact(r#"body: "{\"access_token\":\n  \"secret\"}""#),
            r#"body: "{\"access_token\":\n  \"<redacted>\"}""#
        );
        assert_eq!(redact("Authorization:\nnext operation succeeded"), "Authorization:\nnext operation succeeded");
        assert_eq!(redact("Bearer secret"), "Bearer <redacted>");
    }

    #[test]
    fn device_login_codes() {
        assert_eq!(redact("device_code=abc&user_code=XYZ"), "device_code=<redacted>&user_code=<redacted>");
        assert_eq!(redact(r#"{"deviceCode":"abc","userCode":"XYZ"}"#), r#"{"deviceCode":"<redacted>","userCode":"<redacted>"}"#);
        assert_eq!(redact(r#"{"device_code":"abc"}"#), r#"{"device_code":"<redacted>"}"#);
        assert_eq!(
            redact("LoginTokens(accessToken=a1, refreshToken=r1, userId=1)"),
            "LoginTokens(accessToken=<redacted> refreshToken=<redacted> userId=1)"
        );
        assert_eq!(redact(r#"{"clientSecret":"c","sessionKey":"k"}"#), r#"{"clientSecret":"<redacted>","sessionKey":"<redacted>"}"#);
        assert_eq!(redact(r#"{"hasAccessToken":true,"accessToken":"s"}"#), r#"{"hasAccessToken":true,"accessToken":"<redacted>"}"#);
        assert_eq!(redact("password=true&x=1"), "password=<redacted>&x=1");
        assert_eq!(redact("épassword=x é"), "épassword=<redacted> é");
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
    fn json_values_on_the_next_line() {
        assert_eq!(redact("{\"access_token\":\n  \"tok\"}"), "{\"access_token\":\n  \"<redacted>\"}");
        assert_eq!(redact("{\"password\"\r\n: \"pw\"}"), "{\"password\"\r\n: \"<redacted>\"}");
    }

    #[test]
    fn escaped_characters_inside_a_value_stay_redacted() {
        assert_eq!(redact(r#"body: "{\"password\":\"abc\\ndef\"}""#), r#"body: "{\"password\":\"<redacted>\"}""#);
        assert_eq!(redact(r#"body: "{\"password\":\"a\\\"b\"}""#), r#"body: "{\"password\":\"<redacted>\"}""#);
        assert_eq!(redact(r#"{"password":"a\\b\"c"}"#), r#"{"password":"<redacted>"}"#);
    }

    #[test]
    fn bare_values_keep_their_punctuation() {
        assert_eq!(redact("password=abc,secret;x) next"), "password=<redacted> next");
        assert_eq!(redact("password=abc\\hunter2<x> next"), "password=<redacted> next");
        assert_eq!(redact("password=\\hunter2 next"), "password=<redacted> next");
        assert_eq!(redact("password=abc'def&x=1"), "password=<redacted>&x=1");
        assert_eq!(redact(r#"password=a"b'c next"#), "password=<redacted> next");
        // A closing quote right after the value is redacted too: it could as well be part of the password.
        assert_eq!(redact("msg='password=abc', x"), "msg='password=<redacted> x");
        assert_eq!(redact(r#"password=abc",def next"#), "password=<redacted> next");
        assert_eq!(redact("password='abc'def&x=1"), "password='<redacted>&x=1");
        assert_eq!(redact("password='abc',secret&x=1"), "password='<redacted>&x=1");
        assert_eq!(redact(r#"password="p w" next"#), r#"password="<redacted>" next"#);
        assert_eq!(redact(r#"{"password":"abc"}"#), r#"{"password":"<redacted>"}"#);
        assert_eq!(redact("Authorization: \\xyz\nnext"), "Authorization: <redacted>\nnext");
        assert_eq!(redact(r#"msg: "password=abc\" ok""#), r#"msg: "password=<redacted> ok""#);
    }

    #[test]
    fn multibyte_text_around_secrets_never_splits_a_character() {
        let pieces = ["", "ä", "€", "😀", "\\", "\"", "'", "=", ":", " ", "\n", "&"];
        for key in ["password", "token", "Bearer ", "Authorization", "\"code\"", "\\\"sk\\\""] {
            for a in pieces {
                for b in pieces {
                    for c in pieces {
                        let text = format!("{a}{key}{b}={c}x{a}€{b}\"{c}ä{a}");
                        let _ = redact(&text);
                        let text = format!("{a}{key}{b}:{c}\\\"{a}😀{b}\\\"{c}");
                        let _ = redact(&text);
                    }
                }
            }
        }
    }

    #[test]
    fn single_quoted_values() {
        assert_eq!(redact("password='hunter2' user='me'"), "password='<redacted> user='me'");
        assert_eq!(redact("{'access_token': 'a\\'b'}"), "{'access_token': '<redacted>'}");
    }

    #[test]
    fn last_fm_credentials_and_key_spellings() {
        assert_eq!(
            redact("error sending request for url (https://ws.audioscrobbler.com/2.0/?method=user.getInfo&sk=s3ss&api_sig=f00&api_key=k3y)"),
            "error sending request for url (https://ws.audioscrobbler.com/2.0/?method=user.getInfo&sk=<redacted>&api_sig=<redacted>&api_key=<redacted>"
        );
        assert_eq!(redact("auth.getSession token=t0k"), "auth.getSession token=<redacted>");
        assert_eq!(redact("client-secret=a CLIENT.SECRET = b"), "client-secret=<redacted> CLIENT.SECRET = <redacted>");
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
            "disk=full task=3 x-tidal-token-count=2",
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

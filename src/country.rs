//! User-selectable TIDAL country/region (`countryCode` on every API request).
//!
//! Stored as a plain text file under the XDG config dir rather than `GSettings`,
//! so an older installed schema (e.g. from the Go package) can't make reads abort.

use std::path::PathBuf;

/// (code, English name) for countries where TIDAL is available.
pub const COUNTRIES: &[(&str, &str)] = &[
    ("AD", "Andorra"), ("AE", "United Arab Emirates"), ("AR", "Argentina"), ("AT", "Austria"),
    ("AU", "Australia"), ("BA", "Bosnia and Herzegovina"), ("BE", "Belgium"), ("BG", "Bulgaria"),
    ("BR", "Brazil"), ("CA", "Canada"), ("CH", "Switzerland"), ("CL", "Chile"), ("CO", "Colombia"),
    ("CY", "Cyprus"), ("CZ", "Czechia"), ("DE", "Germany"), ("DK", "Denmark"), ("DO", "Dominican Republic"),
    ("EE", "Estonia"), ("ES", "Spain"), ("FI", "Finland"), ("FR", "France"), ("GB", "United Kingdom"),
    ("GR", "Greece"), ("HK", "Hong Kong"), ("HR", "Croatia"), ("HU", "Hungary"), ("IE", "Ireland"),
    ("IL", "Israel"), ("IS", "Iceland"), ("IT", "Italy"), ("JM", "Jamaica"), ("LI", "Liechtenstein"),
    ("LT", "Lithuania"), ("LU", "Luxembourg"), ("LV", "Latvia"), ("MC", "Monaco"), ("ME", "Montenegro"),
    ("MK", "North Macedonia"), ("MT", "Malta"), ("MX", "Mexico"), ("MY", "Malaysia"), ("NG", "Nigeria"),
    ("NL", "Netherlands"), ("NO", "Norway"), ("NZ", "New Zealand"), ("PE", "Peru"), ("PL", "Poland"),
    ("PR", "Puerto Rico"), ("PT", "Portugal"), ("RO", "Romania"), ("RS", "Serbia"), ("SE", "Sweden"),
    ("SG", "Singapore"), ("SI", "Slovenia"), ("SK", "Slovakia"), ("TH", "Thailand"), ("UG", "Uganda"),
    ("US", "United States"), ("ZA", "South Africa"),
];

fn path() -> PathBuf {
    glib::user_config_dir().join("tonearm").join("country-code")
}

/// Normalises a stored value: a known two-letter code, or `None` for automatic.
pub fn parse(s: &str) -> Option<String> {
    let c = s.trim().to_ascii_uppercase();
    COUNTRIES.iter().any(|(k, _)| *k == c).then_some(c)
}

/// The user's choice, or `None` for automatic (account/session country).
pub fn preferred() -> Option<String> {
    std::fs::read_to_string(path()).ok().and_then(|s| parse(&s))
}

/// Saves the choice (`None` = automatic).
pub fn set_preferred(code: Option<&str>) {
    let p = path();
    let r = match code {
        Some(c) => p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|()| std::fs::write(&p, c)),
        None => match std::fs::remove_file(&p) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            r => r,
        },
    };
    if let Err(e) = r {
        log::warn!("saving country preference failed error={e}");
    }
}

/// Index into the combo model: 0 = Automatic, then `COUNTRIES` in order.
pub fn index_of(code: Option<&str>) -> u32 {
    code.and_then(|c| COUNTRIES.iter().position(|(k, _)| *k == c)).map_or(0, |i| u32::try_from(i + 1).unwrap_or(0))
}

pub fn code_at(index: u32) -> Option<&'static str> {
    (index as usize).checked_sub(1).and_then(|i| COUNTRIES.get(i)).map(|(k, _)| *k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_known_codes_only() {
        assert_eq!(parse(" es\n").as_deref(), Some("ES"));
        assert_eq!(parse("XX"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn index_round_trip() {
        assert_eq!(index_of(None), 0);
        assert_eq!(code_at(0), None);
        let i = index_of(Some("ES"));
        assert!(i > 0);
        assert_eq!(code_at(i), Some("ES"));
        assert_eq!(code_at(9999), None);
    }

    #[test]
    fn list_is_sorted_and_unique() {
        assert!(COUNTRIES.windows(2).all(|w| w[0].0 < w[1].0));
    }
}

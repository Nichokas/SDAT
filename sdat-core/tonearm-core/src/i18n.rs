//! Translations. Like the Go version (gotext + embed), the `.po` files from
//! `sdat-core/locales` are embedded into the binary and looked up at
//! runtime, so no `.mo` installation step is needed.

use std::collections::HashMap;
use std::sync::OnceLock;

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

#[derive(Default)]
struct Catalog {
    singular: HashMap<String, String>,
    plural: HashMap<String, Vec<String>>,
    plural_rule: PluralRule,
}

#[derive(Default, Clone, Copy)]
enum PluralRule {
    #[default]
    NotOne,
    GreaterOne,
    Zero,
    Czech,
    Russian,
    Polish,
}

impl PluralRule {
    fn from_header(h: &str) -> Self {
        if h.contains("nplurals=1") {
            PluralRule::Zero
        } else if h.contains("n > 1") || h.contains("n>1") {
            PluralRule::GreaterOne
        } else if h.contains("(n>=2 && n<=4)") {
            PluralRule::Czech
        } else if h.contains("n%10==1 && n%100!=11") {
            PluralRule::Russian
        } else if h.contains("n==1 ? 0 : n%10>=2") {
            PluralRule::Polish
        } else {
            PluralRule::NotOne
        }
    }

    fn index(self, n: i64) -> usize {
        let n = n.abs();
        match self {
            PluralRule::NotOne => usize::from(n != 1),
            PluralRule::GreaterOne => usize::from(n > 1),
            PluralRule::Zero => 0,
            PluralRule::Czech => {
                if n == 1 {
                    0
                } else if (2..=4).contains(&n) {
                    1
                } else {
                    2
                }
            }
            PluralRule::Russian => {
                if n % 10 == 1 && n % 100 != 11 {
                    0
                } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
                    1
                } else {
                    2
                }
            }
            PluralRule::Polish => {
                if n == 1 {
                    0
                } else if (2..=4).contains(&(n % 10)) && !(12..=14).contains(&(n % 100)) {
                    1
                } else {
                    2
                }
            }
        }
    }
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix('"').unwrap_or(s);
    let s = s.strip_suffix('"').unwrap_or(s);
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(o) => {
                    out.push('\\');
                    out.push(o);
                }
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn parse_po(src: &str) -> Catalog {
    let mut cat = Catalog::default();
    #[derive(PartialEq)]
    #[expect(clippy::items_after_statements, reason = "parser state kept next to its only use")]
    enum F {
        None,
        Ctx,
        Id,
        IdPlural,
        Str(usize),
    }
    let mut field = F::None;
    let (mut id, mut id_plural, mut strs): (String, String, Vec<String>) = Default::default();
    let mut has_entry = false;
    let flush = |id: &mut String, id_plural: &mut String, strs: &mut Vec<String>, cat: &mut Catalog| {
        if id.is_empty() {
            if let Some(h) = strs.first() {
                for line in h.lines() {
                    if line.starts_with("Plural-Forms:") {
                        cat.plural_rule = PluralRule::from_header(line);
                    }
                }
            }
        } else if !id_plural.is_empty() {
            if strs.iter().any(|s| !s.is_empty()) {
                cat.plural.insert(id.clone(), strs.clone());
            }
        } else if let Some(s) = strs.first()
            && !s.is_empty() {
                cat.singular.insert(id.clone(), s.clone());
            }
        id.clear();
        id_plural.clear();
        strs.clear();
    };
    for line in src.lines() {
        let line = line.trim();
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        if line.starts_with("msgctxt ") {
            if has_entry {
                flush(&mut id, &mut id_plural, &mut strs, &mut cat);
            }
            has_entry = false;
            field = F::Ctx;
        } else if let Some(rest) = line.strip_prefix("msgid_plural ") {
            id_plural = unquote(rest);
            field = F::IdPlural;
        } else if let Some(rest) = line.strip_prefix("msgid ") {
            if has_entry {
                flush(&mut id, &mut id_plural, &mut strs, &mut cat);
            }
            has_entry = true;
            id = unquote(rest);
            field = F::Id;
        } else if let Some(rest) = line.strip_prefix("msgstr[") {
            let (idx, rest) = rest.split_once(']').unwrap_or(("0", rest));
            let idx: usize = idx.parse().unwrap_or(0);
            while strs.len() <= idx {
                strs.push(String::new());
            }
            strs[idx] = unquote(rest);
            field = F::Str(idx);
        } else if let Some(rest) = line.strip_prefix("msgstr ") {
            strs = vec![unquote(rest)];
            field = F::Str(0);
        } else if line.starts_with('"') {
            let s = unquote(line);
            match field {
                F::Id => id.push_str(&s),
                F::IdPlural => id_plural.push_str(&s),
                F::Str(i) => strs[i].push_str(&s),
                _ => {}
            }
        }
    }
    if has_entry {
        flush(&mut id, &mut id_plural, &mut strs, &mut cat);
    }
    cat
}

static LOCALE_OVERRIDE: OnceLock<String> = OnceLock::new();

/// Forces the UI locale (e.g. `es_ES`) on platforms without POSIX locale
/// variables (iOS). Call before the first translation; returns `false` if a
/// locale was already set.
pub fn set_locale(locale: &str) -> bool {
    LOCALE_OVERRIDE.set(locale.replace('-', "_")).is_ok()
}

fn detect_locale() -> String {
    if let Some(l) = LOCALE_OVERRIDE.get() {
        return l.clone();
    }
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            let v = v.split(':').next().unwrap_or("").split('.').next().unwrap_or("").to_string();
            if !v.is_empty() && v != "C" && v != "POSIX" {
                return v;
            }
        }
    }
    "en_US".into()
}

/// Returns the user's locale formatted like go-locale (e.g. `es_ES`).
pub fn user_locale() -> String {
    detect_locale()
}

fn catalog() -> &'static Catalog {
    static CAT: OnceLock<Catalog> = OnceLock::new();
    CAT.get_or_init(|| {
        let loc = detect_locale();
        let lang = loc.split('_').next().unwrap_or("").to_string();
        // gotext lookup order: full locale, then language only; plus a few aliases.
        let mut candidates = vec![loc, lang.clone()];
        if lang == "zh" {
            candidates.push("zh_Hans".into());
        }
        if lang == "pt" {
            candidates.push("pt_BR".into());
        }
        for c in candidates {
            if let Some((_, src)) = LOCALES.iter().find(|(name, _)| *name == c) {
                return parse_po(src);
            }
        }
        Catalog::default()
    })
}

/// Translates `msgid`.
pub fn tr(msgid: &str) -> String {
    catalog().singular.get(msgid).cloned().unwrap_or_else(|| msgid.to_string())
}

/// Translates and formats printf-style (`%s`, `%d`, `%v`, `%%`) like gotext.Get(msgid, args...).
pub fn trf(msgid: &str, args: &[&dyn std::fmt::Display]) -> String {
    sprintf(&tr(msgid), args)
}

/// Plural translation like gotext.GetN.
pub fn trn(msgid: &str, plural: &str, n: i64, args: &[&dyn std::fmt::Display]) -> String {
    let cat = catalog();
    let s = match cat.plural.get(msgid) {
        Some(forms) => {
            let idx = cat.plural_rule.index(n).min(forms.len().saturating_sub(1));
            forms[idx].clone()
        }
        None => {
            if n == 1 {
                msgid.to_string()
            } else {
                plural.to_string()
            }
        }
    };
    sprintf(&s, args)
}

pub fn sprintf(fmt: &str, args: &[&dyn std::fmt::Display]) -> String {
    let mut out = String::new();
    let mut it = fmt.chars().peekable();
    let mut ai = 0;
    while let Some(c) = it.next() {
        if c == '%' {
            // skip flags/width/precision (not supported, ignored)
            while let Some(&n) = it.peek() {
                if n.is_ascii_digit() || n == '.' || n == '-' || n == '+' {
                    it.next();
                } else {
                    break;
                }
            }
            match it.next() {
                Some('%') | None => out.push('%'),
                Some(_) => {
                    if let Some(a) = args.get(ai) {
                        out.push_str(&a.to_string());
                    }
                    ai += 1;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn po_parse_and_format() {
        let po = "msgid \"\"\nmsgstr \"\"\n\"Plural-Forms: nplurals=2; plural=n != 1;\\n\"\n\nmsgid \"%d Track\"\nmsgid_plural \"%d Tracks\"\nmsgstr[0] \"%d Pista\"\nmsgstr[1] \"%d Pistas\"\n\nmsgid \"Hello\"\nmsgstr \"Ho\"\n\"la\"\n";
        let c = parse_po(po);
        assert_eq!(c.singular.get("Hello").unwrap(), "Hola");
        assert_eq!(c.plural.get("%d Track").unwrap()[1], "%d Pistas");
        assert_eq!(sprintf("%d Pistas de %s", &[&3, &"x"]), "3 Pistas de x");
        assert_eq!(PluralRule::Russian.index(22), 1);
        assert_eq!(PluralRule::Russian.index(11), 2);
    }
}

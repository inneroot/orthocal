//! Go `internal/i18n` port.
//!
//! The .flt asset files only use single-line `key = value` messages with
//! `{$variable}` placeholders, so a small key-value parser is used instead of
//! a full Fluent implementation.

use std::collections::HashMap;

pub const COMMON_EN: &str = include_str!("../assets/common_en.flt");
pub const COMMEMORATIONS_EN: &str = include_str!("../assets/commemorations_en.flt");
pub const FASTING_RULES_EN: &str = include_str!("../assets/fasting_rules_en.flt");
pub const READINGS_EN: &str = include_str!("../assets/readings_en.flt");

pub const COMMON_IT: &str = include_str!("../assets/common_it.flt");
pub const COMMEMORATIONS_IT: &str = include_str!("../assets/commemorations_it.flt");
pub const FASTING_RULES_IT: &str = include_str!("../assets/fasting_rules_it.flt");
pub const READINGS_IT: &str = include_str!("../assets/readings_it.flt");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    It,
}

/// Maps a `--lang` code to a supported language, mirroring the deterministic
/// behavior of the Go original's golang.org/x/text/language matcher:
/// exactly "it" (any case) selects Italian; everything else (including
/// "it-IT", unknown codes, and the empty string) falls back to English.
///
/// NOTE: in the Go original, unknown codes (e.g. "de") fall back to a
/// non-deterministic default (Go map ordering over the supported tags);
/// this port falls back to English deterministically.
pub fn to_lang(s: &str) -> Lang {
    match s.to_ascii_lowercase().as_str() {
        "it" => Lang::It,
        _ => Lang::En,
    }
}

/// A translation function for the given language.
pub struct T {
    lang: Lang,
    en: HashMap<String, String>,
    it: HashMap<String, String>,
}

impl T {
    pub fn new(lang: Lang) -> T {
        let mut en = HashMap::new();
        for src in [COMMON_EN, COMMEMORATIONS_EN, FASTING_RULES_EN, READINGS_EN] {
            for (k, v) in parse_flt(src) {
                en.entry(k).or_insert(v);
            }
        }
        let mut it = HashMap::new();
        for src in [COMMON_IT, COMMEMORATIONS_IT, FASTING_RULES_IT, READINGS_IT] {
            for (k, v) in parse_flt(src) {
                it.entry(k).or_insert(v);
            }
        }
        T { lang, en, it }
    }

    /// Translates a message key, substituting `{$var}` placeholders.
    /// If the key is not found, the key itself is returned (as in the Go
    /// original, which falls back to the key on format errors/missing keys).
    pub fn translate(&self, key: &str, vars: &[(&str, &str)]) -> String {
        let template = if self.lang == Lang::It {
            self.it.get(key).or_else(|| self.en.get(key))
        } else {
            self.en.get(key)
        };
        match template {
            Some(t) if t.contains('{') => render(t, vars),
            Some(t) => t.clone(),
            None => key.to_string(),
        }
    }
}

/// Parses a .flt file into (key, value) pairs.
fn parse_flt(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let line = line.trim_end_matches('\r').trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(eq) = line.find('=') {
            let key = line[..eq].trim().to_string();
            let value = line[eq + 1..].trim().to_string();
            out.push((key, value));
        }
    }
    out
}

/// Substitutes `{$name}` placeholders in a template with the given variables.
fn render(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(pos) = rest.find('{') {
        out.push_str(&rest[..pos]);
        let after = &rest[pos + 1..];
        if let Some(body) = after.strip_prefix('$') {
            if let Some(end) = body.find('}') {
                let ident = &body[..end];
                match vars.iter().find(|(name, _)| *name == ident) {
                    Some((_, value)) => out.push_str(value),
                    None => out.push_str(&format!("{{${}}}", ident)),
                }
                rest = &body[end + 1..];
                continue;
            }
        }
        out.push('{');
        rest = after;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_lang() {
        assert_eq!(Lang::En, to_lang("en"));
        assert_eq!(Lang::En, to_lang("EN"));
        assert_eq!(Lang::It, to_lang("it"));
        assert_eq!(Lang::It, to_lang("IT"));
        // x/text/language quirk preserved: subtagged Italian codes do not match
        assert_eq!(Lang::En, to_lang("en-US"));
        assert_eq!(Lang::En, to_lang("it-IT"));
        assert_eq!(Lang::En, to_lang("de"));
        assert_eq!(Lang::En, to_lang(""));
    }

    #[test]
    fn test_translate_missing_key_returns_key() {
        let t = T::new(Lang::En);
        assert_eq!("no-such-key", t.translate("no-such-key", &[]));
    }

    #[test]
    fn test_translate_template() {
        let t = T::new(Lang::En);
        let got = t.translate(
            "date-julian",
            &[
                ("weekday", "Monday"),
                ("month-name", "March"),
                ("day", "1"),
                ("year", "2025"),
            ],
        );
        assert_eq!("Monday, March 1, 2025 (Julian)", got);
    }

    #[test]
    fn test_italian_overrides_english() {
        let t = T::new(Lang::It);
        assert_eq!("Lunedì", t.translate("weekday-monday", &[]));
        let en = T::new(Lang::En);
        assert_eq!("Monday", en.translate("weekday-monday", &[]));
    }
}

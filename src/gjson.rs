//! JSON output matching Go's `encoding/json.Marshal` for the
//! `[]LiturgicalDay` structure (compact form, struct field declaration order,
//! `null` for nil slices/pointers, and Go's default HTML escaping of
//! `<`, `>`, `&`, U+2028 and U+2029).

use crate::cal::JulianDate;
use crate::litcal::{self, LiturgicalDay};

/// Escapes a string like Go's `encoding/json` encoder (HTML escaping on).
pub fn go_json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\x08' => out.push_str("\\b"),
            '\x0c' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            '<' => out.push_str("\\u003c"),
            '>' => out.push_str("\\u003e"),
            '&' => out.push_str("\\u0026"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn date_json(d: &JulianDate) -> String {
    format!(
        "{{\"Year\":{},\"Month\":{},\"Day\":{}}}",
        d.year, d.month, d.day
    )
}

fn opt_i64(v: Option<i64>) -> String {
    match v {
        Some(v) => v.to_string(),
        None => "null".to_string(),
    }
}

fn opt_date(d: Option<JulianDate>) -> String {
    match d {
        Some(d) => date_json(&d),
        None => "null".to_string(),
    }
}

pub fn days_json(days: &[LiturgicalDay]) -> String {
    let mut out = String::from("[");
    for (i, d) in days.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('{');
        out.push_str(&format!("\"JulianDate\":{},", date_json(&d.julian_date)));
        out.push_str(&format!(
            "\"GregorianDate\":{{\"Year\":{},\"Month\":{},\"Day\":{}}},",
            d.gregorian_date.year, d.gregorian_date.month, d.gregorian_date.day
        ));

        // Commemorations (nil in Go when empty)
        out.push_str("\"Commemorations\":");
        if d.commemorations.is_empty() {
            out.push_str("null");
        } else {
            out.push('[');
            for (j, pc) in d.commemorations.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                out.push_str("{\"Commemoration\":{");
                out.push_str(&format!("\"Id\":{},", pc.commemoration.id));
                out.push_str(&format!(
                    "\"Name\":{},",
                    go_json_escape(&pc.commemoration.name)
                ));
                out.push_str(&format!("\"Level\":{},", pc.commemoration.level));
                out.push_str(&format!(
                    "\"ForefeastStart\":{},",
                    opt_date(pc.commemoration.forefeast_start)
                ));
                out.push_str(&format!(
                    "\"Start\":{},",
                    date_json(&pc.commemoration.start)
                ));
                out.push_str(&format!("\"End\":{},", date_json(&pc.commemoration.end)));
                out.push_str(&format!(
                    "\"AfterfeastEnd\":{},",
                    opt_date(pc.commemoration.afterfeast_end)
                ));
                out.push_str(&format!(
                    "\"Source\":{}}},",
                    go_json_escape(&pc.commemoration.source)
                ));
                out.push_str(&format!(
                    "\"Projection\":\"{}\"",
                    litcal::projection_str(&pc.projection)
                ));
                out.push('}');
            }
            out.push(']');
        }

        // Fasting
        out.push_str(",\"Fasting\":{\"Rule\":");
        match &d.fasting.rule {
            None => out.push_str("null"),
            Some(r) => {
                out.push_str(&format!("{{\"Id\":{},", r.id));
                out.push_str(&format!(
                    "\"CommemorationId\":{},",
                    opt_i64(r.commemoration_id)
                ));
                out.push_str(&format!("\"Name\":{},", go_json_escape(&r.name)));
                out.push_str(&format!("\"Level\":{},", r.level));
                out.push_str(&format!("\"Start\":{},", date_json(&r.start)));
                out.push_str(&format!("\"End\":{},", date_json(&r.end)));
                out.push_str(&format!("\"Weekdays\":{},", r.weekdays));
                out.push_str(&format!("\"Strictness\":{}}}", r.strictness));
            }
        }
        out.push_str(&format!(",\"Strictness\":{}}},", d.fasting.strictness));

        // Readings (nil in Go when empty)
        out.push_str("\"Readings\":");
        if d.readings.is_empty() {
            out.push_str("null");
        } else {
            out.push('[');
            for (j, pr) in d.readings.iter().enumerate() {
                if j > 0 {
                    out.push(',');
                }
                out.push_str("{\"Components\":[");
                for (k, r) in pr.components.iter().enumerate() {
                    if k > 0 {
                        out.push(',');
                    }
                    out.push_str(&format!("{{\"Id\":{},", r.id));
                    out.push_str(&format!(
                        "\"CommemorationId\":{},",
                        opt_i64(r.commemoration_id)
                    ));
                    out.push_str(&format!("\"Level\":{},", r.level));
                    out.push_str(&format!("\"Start\":{},", date_json(&r.start)));
                    out.push_str(&format!("\"End\":{},", date_json(&r.end)));
                    out.push_str(&format!("\"Weekdays\":{},", r.weekdays));
                    out.push_str(&format!("\"Book\":{},", go_json_escape(&r.book)));
                    out.push_str(&format!(
                        "\"Range\":{{\"Start\":{{\"Chapter\":{},\"Verse\":{}}},\"End\":{{\"Chapter\":{},\"Verse\":{}}}}},",
                        r.range.start.chapter, r.range.start.verse, r.range.end.chapter, r.range.end.verse
                    ));
                    out.push_str(&format!("\"Source\":{}}}", go_json_escape(&r.source)));
                }
                out.push_str(&format!(
                    "],\"Citation\":{}}}",
                    go_json_escape(&pr.citation)
                ));
            }
            out.push(']');
        }
        out.push('}');
    }
    out.push(']');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_go_json_escape() {
        assert_eq!(go_json_escape("plain"), "\"plain\"");
        assert_eq!(
            go_json_escape("a & b < c > d"),
            "\"a \\u0026 b \\u003c c \\u003e d\""
        );
        assert_eq!(
            go_json_escape("quote\"back\\slash"),
            "\"quote\\\"back\\\\slash\""
        );
        assert_eq!(go_json_escape("line\nbreak"), "\"line\\nbreak\"");
    }
}

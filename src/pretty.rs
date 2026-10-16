//! Pretty (non-JSON) output, matching the Go CLI's `calendarcmd.prettyPrint`.
//!
//! Lines are prefixed with "| " (each write to the Go prefix writer is
//! prefixed, not each newline). ANSI colors replicate `github.com/fatih/color`
//! and are enabled only when Go's `color.NoColor` would be false: stdout is a
//! terminal, NO_COLOR is not set, and TERM is not "dumb".

use crate::i18n::T;
use crate::i18ndom;
use crate::litcal::{LiturgicalDay, Projection};

fn no_color() -> bool {
    use std::io::IsTerminal;
    std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty())
        || std::env::var("TERM").as_deref() == Ok("dumb")
        || !std::io::stdout().is_terminal()
}

fn colorize(enabled: bool, codes: &str, text: &str) -> String {
    if enabled {
        format!("\u{1b}[{}m{}\u{1b}[0m", codes, text)
    } else {
        text.to_string()
    }
}

/// SGR codes matching `color.New(FgRed, Bold)` etc.: parameters joined by ';'
/// in the order they were passed, reset with \x1b[0m.
const RED_BOLD: &str = "31;1";
const BLUE: &str = "34";
const YELLOW_BOLD: &str = "33;1";
const YELLOW: &str = "33";
const MAGENTA: &str = "35";
const GREEN: &str = "32";
const WHITE: &str = "37";
const CYAN_BOLD: &str = "36;1";

fn commemoration_codes(pc: &crate::litcal::ProjectedCommemoration) -> &'static str {
    if pc.projection == Projection::StartEnd && pc.commemoration.level == 910 {
        YELLOW_BOLD
    } else if pc.commemoration.level >= 910 {
        YELLOW
    } else if pc.commemoration.level >= 600 {
        MAGENTA
    } else if pc.commemoration.level >= 500 {
        GREEN
    } else {
        WHITE
    }
}

pub fn pretty_print(days: &[LiturgicalDay], t: &T) -> String {
    let color = !no_color();
    let mut out = String::new();
    for (i, day) in days.iter().enumerate() {
        out.push_str("| ");
        out.push_str(&colorize(
            color,
            RED_BOLD,
            &i18ndom::julian_date(t, &day.julian_date),
        ));
        out.push('\n');
        out.push_str("| ");
        out.push_str(&colorize(
            color,
            RED_BOLD,
            &i18ndom::gregorian_date(t, &day.gregorian_date),
        ));
        out.push('\n');
        out.push_str("| \n");

        // Fasting line: "| " goes through the prefix writer, the rest is
        // written directly to stdout (no prefix).
        out.push_str("| ");
        if let Some(rule) = &day.fasting.rule {
            out.push_str(&colorize(color, BLUE, &format!("{}: ", rule.name)));
        }
        out.push_str(&colorize(
            color,
            BLUE,
            &i18ndom::fast_strictness(t, day.fasting.strictness),
        ));
        out.push('\n');
        out.push_str("| \n");

        for (j, pc) in day.commemorations.iter().enumerate() {
            let mut title = i18ndom::title_commemoration(t, pc);
            // Go: strings.ToUpper on the Pascha feast title
            if pc.commemoration.level == 910 && pc.projection == Projection::StartEnd {
                title = title.to_uppercase();
            }
            out.push_str("| ");
            out.push_str(&colorize(color, commemoration_codes(pc), &title));
            out.push('\n');
            if j < day.commemorations.len() - 1 {
                out.push_str("| \n");
            }
        }

        if !day.readings.is_empty() {
            out.push_str("| \n");
            out.push_str("| ");
            out.push_str(&colorize(color, RED_BOLD, "-------"));
            out.push('\n');
        }
        for r in &day.readings {
            out.push_str("| \n");
            out.push_str("| ");
            out.push_str(&colorize(color, CYAN_BOLD, &r.citation));
            out.push('\n');
        }

        if i < days.len() - 1 {
            out.push_str("\n\n\n");
        }
    }
    out
}

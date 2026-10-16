//! orthocal: Rust port of the Go orthocal CLI (src/cmd/cli).
//!
//! Prints the Orthodox liturgical calendar (Julian calendar by default) for a
//! given day, month, or year.

mod cal;
mod conv;
mod csvp;
mod gjson;
mod i18n;
mod i18ndom;
mod litcal;
mod mmath;
mod paschalion;
mod pdqsort;
mod pretty;

/// Embedded version file (assets/.version).
const VERSION: &str = include_str!("../assets/.version");

/// Man-page SYNOPSIS derived help text (see cmd/cli/man in the Go repo).
const USAGE: &str =
    "Usage: orthocal [OPTIONS] [[[DAY] MONTH] YEAR]\nSee 'man orthocal' for OPTIONS.\n";

struct ParsedFlags {
    g: bool,
    json: bool,
    no_readings: bool,
    v: bool,
    h: bool,
    lang: String,
    reform: String,
    positional: Vec<String>,
}

fn main() {
    let code = run();
    if code != 0 {
        std::process::exit(code);
    }
}

fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let parsed = match parse_flags(&args) {
        Ok(p) => p,
        Err(msg) => {
            // Go's flag package (ExitOnError): error to stderr, usage (custom
            // Usage writes to stdout), exit code 2.
            eprintln!("{}", msg);
            print!("{}", USAGE);
            return 2;
        }
    };

    if parsed.h {
        // usageCmd: help text on stdout, EX_USAGE = 64
        print!("{}", USAGE);
        return 64;
    }

    if parsed.v {
        println!("{}", VERSION);
        return 0;
    }

    let lang = i18n::to_lang(&parsed.lang);
    let t = i18n::T::new(lang);

    let reform_date = match conv::to_reform(&parsed.reform, parsed.g) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            return 1;
        }
    };

    let (date_start, date_end) = match conv::to_opt_date_range(&parsed.positional, &reform_date) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", e);
            return 1;
        }
    };

    calendar_run(&t, date_start, date_end, parsed.json, parsed.no_readings)
}

fn calendar_run(
    t: &i18n::T,
    date_start: cal::JulianDate,
    date_end: cal::JulianDate,
    json: bool,
    no_readings: bool,
) -> i32 {
    let data = litcal::Data::load();
    let days = match litcal::load(&date_start, &date_end, no_readings, t, &data) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("{}", e);
            return 1;
        }
    };
    if json {
        print!("{}", gjson::days_json(&days));
    } else {
        print!("{}", pretty::pretty_print(&days, t));
    }
    0
}

/// Parses command line arguments following Go's `flag` package semantics for
/// the flags defined by the Go CLI: flags may appear before the first
/// positional argument; both `-f` and `--f` forms work; boolean flags accept
/// an optional `=value`; parsing stops at the first non-flag argument.
fn parse_flags(args: &[String]) -> Result<ParsedFlags, String> {
    let mut p = ParsedFlags {
        g: false,
        json: false,
        no_readings: false,
        v: false,
        h: false,
        lang: "en".to_string(),
        reform: String::new(),
        positional: Vec::new(),
    };
    let mut i = 0;
    while i < args.len() {
        let arg = &args[i];
        if !arg.starts_with('-') || arg.len() == 1 {
            // First non-flag argument: everything remaining is positional.
            p.positional.extend_from_slice(&args[i..]);
            break;
        }
        let body = arg.trim_start_matches('-');
        if body.is_empty() {
            // "--" ends flag parsing
            p.positional.extend_from_slice(&args[i + 1..]);
            break;
        }
        let (name, inline_value) = match body.find('=') {
            Some(pos) => (&body[..pos], Some(body[pos + 1..].to_string())),
            None => (body, None),
        };
        let display = format!("-{}", name);
        match name {
            "g" => {
                p.g = parse_bool(inline_value, &display)?;
            }
            "json" => {
                p.json = parse_bool(inline_value, &display)?;
            }
            "no-readings" => {
                p.no_readings = parse_bool(inline_value, &display)?;
            }
            "v" => {
                p.v = parse_bool(inline_value, &display)?;
            }
            "h" => {
                p.h = parse_bool(inline_value, &display)?;
            }
            "lang" => {
                p.lang = string_value(inline_value, args, &mut i, &display)?;
            }
            "reform" => {
                p.reform = string_value(inline_value, args, &mut i, &display)?;
            }
            _ => {
                return Err(format!("flag provided but not defined: {}", display));
            }
        }
        i += 1;
    }
    Ok(p)
}

/// Parses a boolean flag value like Go's `strconv.ParseBool`.
fn parse_bool(inline_value: Option<String>, display: &str) -> Result<bool, String> {
    let v = match inline_value {
        Some(v) => v,
        None => return Ok(true),
    };
    match v.as_str() {
        "1" | "t" | "T" | "true" | "TRUE" | "True" => Ok(true),
        "0" | "f" | "F" | "false" | "FALSE" | "False" => Ok(false),
        _ => Err(format!(
            "invalid boolean value \"{}\" for {}: parse error",
            v, display
        )),
    }
}

fn string_value(
    inline_value: Option<String>,
    args: &[String],
    i: &mut usize,
    display: &str,
) -> Result<String, String> {
    match inline_value {
        Some(v) => Ok(v),
        None => {
            *i += 1;
            if *i < args.len() {
                Ok(args[*i].clone())
            } else {
                Err(format!("flag needs an argument: {}", display))
            }
        }
    }
}

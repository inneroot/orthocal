//! Go `internal/csv` port: minimal CSV row parsing for the embedded asset
//! files (no quoting is used in these files).

use crate::conv::to_int as conv_to_int;

/// Splits a CSV asset into rows of columns.
/// Lines starting with '#' and empty lines are skipped, as is the first
/// non-skipped line (the header).
pub fn rows(csv: &str) -> Vec<Vec<String>> {
    let mut out = Vec::new();
    let mut header = true;
    for mut row in csv.split('\n') {
        row = row.trim_end_matches('\r');
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        if header {
            header = false;
            continue;
        }
        out.push(row.split(',').map(|c| c.to_string()).collect());
    }
    out
}

/// Like Go's `csv.ToInt`: panics on invalid input (trusted asset data).
pub fn to_int(s: &str) -> i64 {
    conv_to_int(s, 10).unwrap_or_else(|_| panic!("Cannot convert '{s}' to int using base 10!"))
}

/// Like Go's `csv.ToIntOpt`: empty string is `None` (Go uses math.MaxInt).
pub fn to_int_opt(s: &str) -> Option<i64> {
    if s.is_empty() {
        None
    } else {
        Some(to_int(s))
    }
}

/// Like Go's `csv.ToWeekdays`: empty string is all weekdays (0b1111111).
pub fn to_weekdays(s: &str) -> i64 {
    if s.is_empty() {
        0b1111111
    } else {
        conv_to_int(s, 2).unwrap_or_else(|_| panic!("Cannot convert '{s}' to int using base 2!"))
    }
}

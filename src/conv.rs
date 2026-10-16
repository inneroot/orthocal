//! Go `internal/conv` port: CLI date argument parsing and reform handling.

use crate::cal::{self, JulianDate};
use crate::paschalion;

pub const ERR_INVALID_DATE_FORMAT: &str = "invalid date format";

pub fn verify_range(start: &JulianDate, end: &JulianDate) -> Result<(), String> {
    if end.before(start) {
        return Err("start year must be lower than end year".to_string());
    }
    if start.year < paschalion::LOWEST_YEAR {
        return Err("year too low".to_string());
    }
    if end.year > paschalion::HIGHEST_YEAR {
        return Err("year too high".to_string());
    }
    Ok(())
}

/// NOTE: used only by the website generator in the Go original.
#[allow(dead_code)]
pub fn to_date_range(
    start_year: i64,
    end_year: i64,
    reform_date: &JulianDate,
) -> Result<(JulianDate, JulianDate), String> {
    let start = cal::new_julian_date_with_reform(start_year, 1, 1, reform_date)?;
    let end = cal::new_julian_date_with_reform(end_year, 12, -1, reform_date)?;
    verify_range(&start, &end)?;
    Ok((start, end))
}

pub fn to_opt_date_range(
    args: &[String],
    reform_date: &JulianDate,
) -> Result<(JulianDate, JulianDate), String> {
    let nums = parse_numbers(args)?;
    let (start, end) = if nums.is_empty() {
        // calendar of today
        let start = cal::current_julian_date();
        let end = start;
        (start, end)
    } else if nums.len() == 3 {
        // calendar of the given day
        let day = nums[0];
        let month = nums[1];
        let year = nums[2];
        let start = cal::new_julian_date_with_reform(year, month as i32, day as i32, reform_date)?;
        let end = start;
        (start, end)
    } else if nums.len() == 2 {
        // calendar of the entire given month
        let month = nums[0];
        let year = nums[1];
        let start = cal::new_julian_date_with_reform(year, month as i32, 1, reform_date)?;
        let end = cal::new_julian_date_with_reform(year, month as i32, -1, reform_date)?;
        (start, end)
    } else {
        // calendar of the entire given year
        let year = nums[0];
        let start = cal::new_julian_date_with_reform(year, 1, 1, reform_date)?;
        let end = cal::new_julian_date_with_reform(year, 12, -1, reform_date)?;
        (start, end)
    };
    verify_range(&start, &end)?;
    Ok((start, end))
}

/// Gregorian reform on 1 March (year before the lowest supported year):
/// all dates are interpreted as Gregorian.
pub const REFORM_DATE_PROLEPTIC_GREGORIAN: JulianDate = JulianDate {
    year: paschalion::LOWEST_YEAR - 1,
    month: 1,
    day: 1,
};

/// No Gregorian reform: all dates are interpreted as Julian.
pub const REFORM_DATE_NO_REFORM: JulianDate = JulianDate {
    year: paschalion::HIGHEST_YEAR + 1,
    month: 12,
    day: 1,
};

pub fn to_reform(s: &str, reform_proleptic: bool) -> Result<JulianDate, String> {
    if reform_proleptic || s == "proleptic" {
        return Ok(REFORM_DATE_PROLEPTIC_GREGORIAN);
    }
    if !s.is_empty() {
        return parse_reform_date(s);
    }
    // Use no Gregorian reform
    Ok(REFORM_DATE_NO_REFORM)
}

fn parse_reform_date(s: &str) -> Result<JulianDate, String> {
    let parts: Vec<&str> = s.split(',').collect();
    parse_date(&parts)
}

fn parse_date(parts: &[&str]) -> Result<JulianDate, String> {
    let nums = parse_numbers_str(parts)?;
    if nums.len() != 3 {
        // NOTE: the Go original panics here (out-of-bounds index); this port
        // returns an error instead.
        return Err(ERR_INVALID_DATE_FORMAT.to_string());
    }
    Ok(JulianDate {
        year: nums[2],
        month: nums[1] as i32,
        day: nums[0] as i32,
    })
}

pub fn parse_numbers(parts: &[String]) -> Result<Vec<i64>, String> {
    parse_numbers_str(&parts.iter().map(|s| s.as_str()).collect::<Vec<&str>>())
}

fn parse_numbers_str(parts: &[&str]) -> Result<Vec<i64>, String> {
    if parts.len() > 3 {
        return Err(ERR_INVALID_DATE_FORMAT.to_string());
    }
    let mut nums = Vec::with_capacity(parts.len());
    for part in parts {
        nums.push(to_int(part, 10)?);
    }
    Ok(nums)
}

/// Parses an integer like Go's `strconv.ParseInt(s, base, 64)`, producing the
/// same error messages for invalid input.
pub fn to_int(s: &str, base: u32) -> Result<i64, String> {
    let mut t = s;
    let mut negative = false;
    if let Some(rest) = t.strip_prefix('+') {
        t = rest;
    } else if let Some(rest) = t.strip_prefix('-') {
        t = rest;
        negative = true;
    }
    if t.is_empty() {
        return Err(format!(
            "strconv.ParseInt: parsing \"{}\": invalid syntax",
            s
        ));
    }
    for ch in t.chars() {
        if !(ch.is_ascii_digit() && ch.to_digit(base).is_some_and(|d| d < base)) {
            return Err(format!(
                "strconv.ParseInt: parsing \"{}\": invalid syntax",
                s
            ));
        }
    }
    let mut value: i64 = 0;
    for ch in t.chars() {
        let digit = ch.to_digit(base).unwrap() as i64;
        value = value
            .checked_mul(base as i64)
            .and_then(|v| v.checked_sub(digit))
            .ok_or_else(|| format!("strconv.ParseInt: parsing \"{}\": value out of range", s))?;
    }
    if negative {
        Ok(value)
    } else {
        value
            .checked_neg()
            .ok_or_else(|| format!("strconv.ParseInt: parsing \"{}\": value out of range", s))
    }
}

//! Go `pkg/cal` port: Julian and Gregorian dates, weekday math, and date ranges.

use crate::mmath;
use chrono::Datelike;

/// A date in the proleptic Julian calendar. Year 1 = 1 AD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JulianDate {
    /// 1 = 1 AD, 0 = 1 BC
    pub year: i64,
    /// Starts at 1 for the first month of the year
    pub month: i32,
    /// Starts at 1 for the first day of the month
    pub day: i32,
}

/// A date in the proleptic Gregorian calendar. Year 1 = 1 AD.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GregorianDate {
    /// 1 = 1 AD, 0 = 1 BC
    pub year: i64,
    /// Starts at 1 for the first month of the year
    pub month: i32,
    /// Starts at 1 for the first day of the month
    pub day: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JulianPeriod {
    pub start: JulianDate,
    pub end: JulianDate,
}

#[allow(dead_code)]
const JULIAN_LONGEST_MONTH: i32 = 31;
#[allow(dead_code)]
const GREGORIAN_LONGEST_MONTH: i32 = 31;

/// Creates a JulianDate if the given year, month and day are valid.
///
/// If day is -1, the last day of the month is picked.
pub fn new_julian_date(year: i64, month: i32, day: i32) -> Result<JulianDate, String> {
    let months = julian_months(year);
    if month > 0 && month <= months.len() as i32 {
        let mut day = day;
        if day == -1 {
            day = months[(month - 1) as usize];
        }
        if day > 0 && day <= months[(month - 1) as usize] {
            return Ok(JulianDate { year, month, day });
        }
    }
    Err("invalid Julian date".to_string())
}

/// Creates a GregorianDate if the given year, month and day are valid.
///
/// If day is -1, the last day of the month is picked.
pub fn new_gregorian_date(year: i64, month: i32, day: i32) -> Result<GregorianDate, String> {
    let months = gregorian_months(year);
    if month > 0 && month <= months.len() as i32 {
        let mut day = day;
        if day == -1 {
            day = months[(month - 1) as usize];
        }
        if day > 0 && day <= months[(month - 1) as usize] {
            return Ok(GregorianDate { year, month, day });
        }
    }
    Err("invalid Gregorian date".to_string())
}

/// Creates a JulianDate. The given year, month and day are interpreted based on
/// the given Gregorian reform date.
/// The reform date must be the first day on the Julian calendar on which the
/// Gregorian calendar was used.
/// Dates before the reform are interpreted as Julian and dates after the
/// reform date are interpreted as Gregorian.
pub fn new_julian_date_with_reform(
    year: i64,
    month: i32,
    day: i32,
    reform_date: &JulianDate,
) -> Result<JulianDate, String> {
    let skip_start = *reform_date;
    let skip_end = reform_date.to_gregorian().shift_day(-1);
    // If the date can be converted to a Julian date and it is before the reform
    // date, it is a Julian date.
    let jd = new_julian_date(year, month, day)?;
    if jd.before(&skip_start) {
        return Ok(jd);
    }
    // Else, if the date can be converted to a Gregorian date and it is after the
    // skipped days, it is a Gregorian date.
    let gd = new_gregorian_date(year, month, day)?;
    if skip_end.before(&gd) {
        return Ok(gd.to_julian());
    }
    Err(
        "invalid date: the given date does not exist because of the Gregorian calendar reform"
            .to_string(),
    )
}

/// The current local date, converted from Gregorian to Julian.
pub fn current_julian_date() -> JulianDate {
    let now = chrono::Local::now();
    GregorianDate {
        year: now.year() as i64,
        month: now.month() as i32,
        day: now.day() as i32,
    }
    .to_julian()
}

fn gregorian_julian_delta_days(year: i64) -> i64 {
    year / 100 - year / 400 - 2
}

impl JulianDate {
    /// Converts the given Julian date to a Gregorian date.
    pub fn to_gregorian(&self) -> GregorianDate {
        let d = GregorianDate {
            year: self.year,
            month: self.month,
            day: self.day,
        };
        d.shift_day(gregorian_julian_delta_days(d.year))
    }

    /// Returns the ordinal day, which ranges from 1 to 366.
    #[allow(dead_code)]
    pub fn day_of_year(&self) -> i64 {
        let mut sum = self.day as i64;
        let months = julian_months(self.year);
        for month_index in 0..(self.month - 1) {
            sum += months[month_index as usize] as i64;
        }
        sum
    }

    /// Returns the year of the Byzantine calendar for the given Julian date.
    /// This corresponds to the number of years since the creation of the world.
    #[allow(dead_code)]
    pub fn to_byzantine_year(&self) -> i64 {
        let mut y = self.year;
        y += 5509;
        // The year starts on 1 September
        if self.month < 9 {
            y -= 1;
        }
        if y < 1 {
            panic!("Anno Mundi out of bounds: {}", y);
        }
        y
    }

    pub fn before(&self, b: &JulianDate) -> bool {
        if self.year == b.year {
            if self.month == b.month {
                if self.day == b.day {
                    return false;
                }
                return self.day < b.day;
            }
            return self.month < b.month;
        }
        self.year < b.year
    }

    pub fn before_equal(&self, b: &JulianDate) -> bool {
        *self == *b || self.before(b)
    }

    /// Returns whether d is between a and b (inclusive).
    pub fn between(&self, a: &JulianDate, b: &JulianDate) -> bool {
        a.before_equal(self) && self.before_equal(b)
    }

    /// Returns the distance from the given date, in days.
    /// If 'self' is before 'to': self.distance(to) = +
    pub fn distance(&self, to: &JulianDate) -> i64 {
        let (start, end, negative) = if to.before(self) {
            (*to, *self, true)
        } else {
            (*self, *to, false)
        };
        let mut days: i64 = 0;
        for year in start.year..=end.year {
            if year % 4 == 0 {
                days += 366;
            } else {
                days += 365;
            }
        }

        // Days between "1 January start.year" and "start"
        let mut deltastart = 0;
        let start_months = julian_months(start.year);
        for i in 0..(start.month - 1) {
            deltastart += start_months[i as usize] as i64;
        }
        deltastart += start.day as i64;
        days -= deltastart;

        // Days between "end" and "31 December end.year"
        let mut deltaend = 0;
        let end_months = julian_months(end.year);
        for i in end.month..12 {
            deltaend += end_months[i as usize] as i64;
        }
        deltaend += end_months[(end.month - 1) as usize] as i64 - end.day as i64;
        days -= deltaend;

        if negative {
            -days
        } else {
            days
        }
    }

    /// Returns the weekday of the Julian date. Sunday = 0, Saturday = 6.
    pub fn weekday(&self) -> i64 {
        let reference_sunday = JulianDate {
            year: 2025,
            month: 4,
            day: 7,
        };
        mmath::r#mod(reference_sunday.distance(self), 7)
    }

    /// Returns the day that is the nth weekday before or after this date.
    ///
    /// Possible values for nth_weekday:
    ///     8 = the 2nd Sunday before
    ///     7 = the 1st Saturday before
    ///     ...
    ///     1 = the 1st Sunday before
    ///     0 = invalid
    ///     1 = the 1st Sunday after
    ///     ...
    ///     7 = the 1st Saturday after
    pub fn nth_weekday(&self, nth_weekday: i64) -> Result<JulianDate, String> {
        if nth_weekday == 0 {
            return Err(format!("invalid nth weekday: {}", nth_weekday));
        }
        let inc: i64 = if nth_weekday < 0 { -1 } else { 1 };
        let mut days: i64 = 0;
        let start_weekday = self.weekday();
        let mut weekday = start_weekday;
        let target_weekday = (mmath::abs(nth_weekday) - 1) % 7;
        let mut reset = 0;
        let mut target_reset = (mmath::abs(nth_weekday) - 1) / 7;
        // If the start weekday is already the target weekday, seek the next one
        if target_weekday == weekday {
            target_reset += 1;
        }
        while weekday != target_weekday || reset != target_reset {
            days += inc;
            weekday = mmath::r#mod(weekday + inc, 7);
            if weekday == start_weekday {
                reset += 1;
            }
        }
        Ok(self.shift_day(days))
    }

    /// Returns the closest day to this date that is the given weekday.
    /// If this date is the given weekday, this date is returned.
    ///
    /// Possible values for closest_weekday:
    ///     1 = the closest Sunday
    ///     ...
    ///     7 = the closest Saturday
    pub fn closest_weekday(&self, closest_weekday: i64) -> Result<JulianDate, String> {
        if closest_weekday < 1 || closest_weekday > 7 {
            return Err(format!("invalid closest weekday: {}", closest_weekday));
        }
        let target_weekday = closest_weekday - 1;
        let days = target_weekday - self.weekday();
        Ok(self.shift_day(days))
    }

    /// Shifts the date by the given number of days.
    pub fn shift_day(&self, days: i64) -> JulianDate {
        let mut date = *self;
        date.day = (date.day as i64 + days) as i32;
        let (year, month, day) = scale_day_month_year(date.year, date.month, date.day, false);
        date.year = year;
        date.month = month;
        date.day = day;
        date
    }
}

impl GregorianDate {
    /// Converts the given Gregorian date to a Julian date.
    pub fn to_julian(&self) -> JulianDate {
        let d = JulianDate {
            year: self.year,
            month: self.month,
            day: self.day,
        };
        d.shift_day(-gregorian_julian_delta_days(d.year))
    }

    /// Returns the ordinal day, which ranges from 1 to 366.
    #[allow(dead_code)]
    pub fn day_of_year(&self) -> i64 {
        let mut sum = self.day as i64;
        let months = gregorian_months(self.year);
        for month_index in 0..(self.month - 1) {
            sum += months[month_index as usize] as i64;
        }
        sum
    }

    pub fn before(&self, b: &GregorianDate) -> bool {
        if self.year == b.year {
            if self.month == b.month {
                if self.day == b.day {
                    return false;
                }
                return self.day < b.day;
            }
            return self.month < b.month;
        }
        self.year < b.year
    }

    /// Shifts the date by the given number of days.
    pub fn shift_day(&self, days: i64) -> GregorianDate {
        let mut date = *self;
        date.day = (date.day as i64 + days) as i32;
        let (year, month, day) = scale_day_month_year(date.year, date.month, date.day, true);
        date.year = year;
        date.month = month;
        date.day = day;
        date
    }

    /// Shifts the date by the given number of months.
    /// If the day does not exist in the target month, the next lowest day is taken.
    ///
    /// NOTE: the Go original uses `julianMonths` here (port preserved verbatim).
    #[allow(dead_code)]
    pub fn shift_month(&self, months: i64) -> GregorianDate {
        let old_day = self.day;
        let mut date = *self;
        date.day = 1;
        date.month = (date.month as i64 + months) as i32;
        let (year, month) = scale_month_year(date.year, date.month, false);
        date.year = year;
        date.month = month;
        date.day = old_day;
        let days_of_new_month = julian_months(date.year)[(date.month - 1) as usize];
        if date.day > days_of_new_month {
            date.day = days_of_new_month;
        }
        date
    }

    /// Shifts the date by the given number of years.
    /// If the day does not exist in the target month, the next lowest day is taken.
    ///
    /// NOTE: the Go original uses `julianMonths` here (port preserved verbatim).
    #[allow(dead_code)]
    pub fn shift_year(&self, years: i64) -> GregorianDate {
        let old_day = self.day;
        let mut date = *self;
        date.day = 1;
        date.year += years;
        let (year, month) = scale_month_year(date.year, date.month, false);
        date.year = year;
        date.month = month;
        date.day = old_day;
        let days_of_new_month = julian_months(date.year)[(date.month - 1) as usize];
        if date.day > days_of_new_month {
            date.day = days_of_new_month;
        }
        date
    }
}

/// Returns an iterator-friendly vec over all the days between start and end (inclusive).
/// If end is before start, no days are yielded.
pub fn days_julian(start: &JulianDate, end: &JulianDate) -> Vec<JulianDate> {
    let mut days = Vec::new();
    let mut d = *start;
    loop {
        if end.before(&d) {
            break;
        }
        days.push(d);
        d = d.shift_day(1);
    }
    days
}

/// Splits the range between start and end (inclusive) into per-year periods.
pub fn years_julian(start: &JulianDate, end: &JulianDate) -> Vec<(i64, JulianPeriod)> {
    let mut out = Vec::new();
    let mut i: i64 = 0;
    loop {
        let year = start.year + i;
        let (sm, sd, em, ed) = if year == start.year && year == end.year {
            (start.month, start.day, end.month, end.day)
        } else if year == start.year {
            (start.month, start.day, 12, 31)
        } else if year < end.year {
            (1, 1, 12, 31)
        } else if year == end.year {
            (1, 1, end.month, end.day)
        } else {
            break;
        };
        out.push((
            year,
            JulianPeriod {
                start: JulianDate {
                    year,
                    month: sm,
                    day: sd,
                },
                end: JulianDate {
                    year,
                    month: em,
                    day: ed,
                },
            },
        ));
        i += 1;
    }
    out
}

fn is_gregorian_leap_year(year: i64) -> bool {
    year % 400 == 0 || (year % 4 == 0 && year % 100 != 0)
}

fn gregorian_months(year: i64) -> [i32; 12] {
    let mut months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_gregorian_leap_year(year) {
        months[1] = 29;
    }
    months
}

fn is_julian_leap_year(year: i64) -> bool {
    year % 4 == 0
}

fn julian_months(year: i64) -> [i32; 12] {
    let mut months = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if is_julian_leap_year(year) {
        months[1] = 29;
    }
    months
}

fn scale_day_month_year(year: i64, month: i32, day: i32, gregorian: bool) -> (i64, i32, i32) {
    let mut year = year;
    let mut month = month;
    let mut day = day;
    let mut months: [i32; 12];

    (year, month) = scale_month_year(year, month, gregorian);
    months = month_lengths(year, gregorian);
    while day > months[(month - 1) as usize] {
        day -= months[(month - 1) as usize];
        month += 1;
        (year, month) = scale_month_year(year, month, gregorian);
        months = month_lengths(year, gregorian);
    }
    while day < 1 {
        month -= 1;
        (year, month) = scale_month_year(year, month, gregorian);
        months = month_lengths(year, gregorian);
        day += months[(month - 1) as usize];
    }
    (year, month, day)
}

fn scale_month_year(year: i64, month: i32, gregorian: bool) -> (i64, i32) {
    let mut year = year;
    let mut month = month;
    let mut months: [i32; 12];

    months = month_lengths(year, gregorian);
    while month > months.len() as i32 {
        month -= months.len() as i32;
        year += 1;
        months = month_lengths(year, gregorian);
    }
    while month < 1 {
        year -= 1;
        months = month_lengths(year, gregorian);
        month += months.len() as i32;
    }
    (year, month)
}

fn month_lengths(year: i64, gregorian: bool) -> [i32; 12] {
    if gregorian {
        gregorian_months(year)
    } else {
        julian_months(year)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GREGORIAN_LEAP_START: JulianDate = JulianDate {
        year: 1582,
        month: 10,
        day: 4,
    };
    const GREGORIAN_LEAP_END: JulianDate = JulianDate {
        year: 1582,
        month: 10,
        day: 15,
    };

    fn jd(year: i64, month: i32, day: i32) -> JulianDate {
        JulianDate { year, month, day }
    }

    fn gd(year: i64, month: i32, day: i32) -> GregorianDate {
        GregorianDate { year, month, day }
    }

    #[test]
    fn test_new_julian_date() {
        let tt = [
            (jd(2004, 3, 20), jd(2004, 3, 20)),
            (jd(2004, 12, 31), jd(2004, 12, 31)),
            (jd(2004, 12, -1), jd(2004, 12, 31)),
        ];
        for (in_d, exp) in tt {
            let got = new_julian_date(in_d.year, in_d.month, in_d.day).unwrap();
            assert_eq!(exp, got);
        }
    }

    #[test]
    fn test_new_julian_date_err() {
        let tt = [
            jd(2004, 0, 2),
            jd(2004, 3, 0),
            jd(2004, 13, 0),
            jd(2004, -1, 0),
        ];
        for in_d in tt {
            assert!(new_julian_date(in_d.year, in_d.month, in_d.day).is_err());
        }
    }

    #[test]
    fn test_new_gregorian_date() {
        let tt = [
            (gd(2004, 3, 20), gd(2004, 3, 20)),
            (gd(2004, 12, 31), gd(2004, 12, 31)),
            (gd(2004, 12, -1), gd(2004, 12, 31)),
        ];
        for (in_d, exp) in tt {
            let got = new_gregorian_date(in_d.year, in_d.month, in_d.day).unwrap();
            assert_eq!(exp, got);
        }
    }

    #[test]
    fn test_new_gregorian_date_err() {
        let tt = [
            gd(2004, 0, 2),
            gd(2004, 3, 0),
            gd(2004, 13, 0),
            gd(2004, -1, 0),
        ];
        for in_d in tt {
            assert!(new_gregorian_date(in_d.year, in_d.month, in_d.day).is_err());
        }
    }

    #[test]
    fn test_new_julian_date_with_reform() {
        let british_empire = jd(1752, 9, 3);
        let tt = [
            (jd(2004, 3, 20), &british_empire, jd(2004, 3, 7)),
            (jd(1600, 3, 20), &british_empire, jd(1600, 3, 20)),
        ];
        for (in_d, reform, exp) in tt {
            let got = new_julian_date_with_reform(in_d.year, in_d.month, in_d.day, reform).unwrap();
            assert_eq!(exp, got);
        }

        // Non existing dates should return an error
        for i in 0..gregorian_julian_delta_days(british_empire.year) {
            let r = new_julian_date_with_reform(
                british_empire.year,
                british_empire.month,
                british_empire.day + i as i32,
                &british_empire,
            );
            assert!(r.is_err());
        }
    }

    #[test]
    fn test_julian_to_gregorian() {
        let tt = [
            (
                GREGORIAN_LEAP_START,
                gd(
                    GREGORIAN_LEAP_START.year,
                    GREGORIAN_LEAP_START.month,
                    GREGORIAN_LEAP_END.day - 1,
                ),
            ),
            (jd(-5508, 3, 1), gd(-5508, 1, 17)),
            (jd(-4, 3, 1), gd(-4, 2, 28)),
            (jd(-3, 3, 1), gd(-3, 2, 27)),
            (jd(325, 2, 28), gd(325, 3, 1)),
            (jd(324, 2, 28), gd(324, 2, 29)),
            (jd(324, 2, 29), gd(324, 3, 1)),
            (jd(1700, 12, 31), gd(1701, 1, 11)),
            (jd(2000, 4, 22), gd(2000, 5, 5)),
            (jd(2025, 4, 26), gd(2025, 5, 9)),
            (jd(5507, 6, 29), gd(5507, 8, 8)),
        ];
        for (julian, gregorian) in tt {
            assert_eq!(gregorian, julian.to_gregorian());
        }
    }

    #[test]
    fn test_julian_day_of_year() {
        let tt = [
            (jd(2, 9, 1), 244),
            (jd(1998, 9, 1), 244),
            (jd(2018, 1, 3), 3),
            (jd(1700, 3, 2), 62),
        ];
        for (date, day_of_year) in tt {
            assert_eq!(day_of_year, date.day_of_year());
        }
    }

    #[test]
    fn test_gregorian_day_of_year() {
        let tt = [
            (gd(2, 9, 1), 244),
            (gd(1998, 9, 1), 244),
            (gd(2018, 1, 3), 3),
            (gd(1700, 3, 2), 61),
        ];
        for (date, day_of_year) in tt {
            assert_eq!(day_of_year, date.day_of_year());
        }
    }

    #[test]
    fn test_julian_to_byzantine_year() {
        let tt = [
            (jd(-5508, 9, 1), 1),
            (jd(-4, 3, 1), 5504),
            (jd(-3, 3, 1), 5505),
            (jd(-3, 10, 1), 5506),
            (jd(2000, 2, 4), 7508),
            (jd(2000, 2, 28), 7508),
            (jd(2025, 8, 31), 7533),
            (jd(2025, 9, 1), 7534),
        ];
        for (julian, am) in tt {
            assert_eq!(am, julian.to_byzantine_year());
        }
    }

    #[test]
    #[should_panic]
    fn test_julian_to_byzantine_year_panic() {
        let julian = jd(-5508, 3, 1);
        let _ = julian.to_byzantine_year();
    }

    #[test]
    fn test_julian_distance() {
        let tt = [
            ((jd(2000, 2, 7), jd(2000, 2, 7)), 0),
            ((jd(2000, 2, 7), jd(2000, 2, 10)), 3),
            ((jd(2000, 4, 7), jd(2000, 5, 7)), 30),
            ((jd(2001, 4, 7), jd(2001, 5, 7)), 30),
            ((jd(1800, 5, 7), jd(1800, 6, 7)), 31),
            ((jd(2010, 8, 14), jd(2011, 5, 7)), 266),
            ((jd(2010, 8, 14), jd(2011, 11, 2)), 445),
            ((jd(2010, 4, 7), jd(2012, 5, 7)), 761),
        ];
        for ((from, to), distance) in tt {
            assert_eq!(distance, from.distance(&to));
            assert_eq!(-distance, to.distance(&from));
        }
    }

    #[test]
    fn test_julian_shift_day() {
        let tt = [
            (jd(2000, 2, 7), 0, jd(2000, 2, 7)),
            (jd(2000, 2, 7), 1, jd(2000, 2, 8)),
            (jd(2000, 2, 7), 10, jd(2000, 2, 17)),
            (jd(2000, 2, 7), 23, jd(2000, 3, 1)),
            (jd(2000, 12, 7), -1, jd(2000, 12, 6)),
            (jd(2000, 12, 1), -1, jd(2000, 11, 30)),
            (jd(2000, 2, 7), -12, jd(2000, 1, 26)),
            (jd(2000, 2, 7), -7, jd(2000, 1, 31)),
            (jd(2000, 2, 7), -100, jd(1999, 10, 30)),
        ];
        for (start, days, shifted) in tt {
            let got = start.shift_day(days);
            assert_eq!(shifted, got);
            // Reverse shift
            assert_eq!(start, got.shift_day(-days));
        }
    }

    #[test]
    fn test_gregorian_shift_month() {
        let tt = [
            (gd(2000, 2, 7), 0, gd(2000, 2, 7)),
            (gd(2000, 2, 7), 1, gd(2000, 3, 7)),
            (gd(2000, 2, 7), 2, gd(2000, 4, 7)),
            (gd(2000, 2, 7), 13, gd(2001, 3, 7)),
            (gd(2000, 12, 1), -1, gd(2000, 11, 1)),
            (gd(2000, 1, 5), -1, gd(1999, 12, 5)),
            (gd(2000, 2, 7), -12, gd(1999, 2, 7)),
            (gd(2000, 8, 7), 1, gd(2000, 9, 7)),
            (gd(2000, 8, 31), -2, gd(2000, 6, 30)),
            (gd(2025, 8, 30), -2, gd(2025, 6, 30)),
            (gd(2020, 2, 29), -12, gd(2019, 2, 28)),
            (gd(2019, 2, 28), 12, gd(2020, 2, 28)),
            (gd(2016, 2, 29), 4 * 12, gd(2020, 2, 29)),
        ];
        for (date, s, shifted) in tt {
            let got = date.shift_month(s);
            assert_eq!(shifted, got);
        }
    }

    #[test]
    fn test_gregorian_shift_year() {
        let tt = [
            (gd(2025, 8, 7), 1, gd(2026, 8, 7)),
            (gd(2020, 2, 29), -2, gd(2018, 2, 28)),
            (gd(2020, 2, 28), -2, gd(2018, 2, 28)),
            (gd(2016, 2, 29), 4, gd(2020, 2, 29)),
        ];
        for (date, s, shifted) in tt {
            let got = date.shift_year(s);
            assert_eq!(shifted, got);
        }
    }

    #[test]
    fn test_before() {
        let before = [(jd(2025, 4, 10), jd(2025, 5, 10))];
        for (a, b) in before {
            assert!(a.before(&b));
        }
        let not_before = [(jd(2025, 8, 10), jd(2025, 5, 10))];
        for (a, b) in not_before {
            assert!(!a.before(&b));
        }
    }

    #[test]
    fn test_between() {
        let d = jd(2025, 4, 10);
        let between = [
            (jd(2025, 4, 10), jd(2025, 5, 10)),
            (jd(2025, 4, 8), jd(2025, 5, 10)),
            (jd(2024, 4, 12), jd(2025, 5, 10)),
        ];
        for (a, b) in between {
            assert!(d.between(&a, &b));
        }
        let not_between = [
            (jd(2025, 8, 10), jd(2025, 5, 10)),
            (jd(2026, 8, 10), jd(2025, 5, 10)),
            (jd(2025, 5, 8), jd(2025, 5, 10)),
            (jd(2025, 4, 11), jd(2025, 5, 10)),
        ];
        for (a, b) in not_between {
            assert!(!d.between(&a, &b));
        }
    }

    #[test]
    fn test_weekday() {
        let tt = [
            (jd(2025, 4, 6), 6),
            (jd(2025, 4, 7), 0),
            (jd(2025, 4, 8), 1),
            (jd(2025, 5, 29), 3),
            (jd(900, 6, 1), 0),
        ];
        for (d, w) in tt {
            assert_eq!(w, d.weekday());
        }
    }

    #[test]
    fn test_nth_weekday() {
        let tt = [
            (jd(2025, 5, 29), 1, jd(2025, 6, 2)),
            (jd(2025, 6, 11), 1, jd(2025, 6, 16)),
            (jd(2025, 4, 7), 1, jd(2025, 4, 14)),
            (jd(2025, 4, 7), 2, jd(2025, 4, 8)),
            (jd(2025, 4, 7), 8, jd(2025, 4, 21)),
            (jd(2025, 4, 7), 9, jd(2025, 4, 15)),
            (jd(2025, 4, 10), 9, jd(2025, 4, 22)),
            (jd(2025, 4, 7), -1, jd(2025, 3, 31)),
            (jd(2025, 4, 6), -1, jd(2025, 3, 31)),
            (jd(2025, 4, 6), -2, jd(2025, 4, 1)),
            (jd(2025, 4, 6), -8, jd(2025, 3, 24)),
            (jd(2025, 8, 1), -9, jd(2025, 7, 22)),
        ];
        for (date, nth, w) in tt {
            let res = date.nth_weekday(nth).unwrap();
            assert_eq!(w, res, "Input date: {:?}", date);
        }
    }

    #[test]
    fn test_closest_weekday() {
        let tt = [
            (jd(2025, 5, 29), 1, jd(2025, 5, 26)),
            (jd(2025, 5, 29), 4, jd(2025, 5, 29)),
            (jd(900, 2, 29), 7, jd(900, 3, 1)),
        ];
        for (date, closest, w) in tt {
            let res = date.closest_weekday(closest).unwrap();
            assert_eq!(w, res, "Input date: {:?}", date);
        }
    }

    #[test]
    fn test_days_seq() {
        let tt = [
            (jd(2025, 5, 3), jd(2025, 6, 12)),
            (jd(2024, 8, 3), jd(2025, 6, 12)),
            (jd(2024, 8, 3), jd(2028, 6, 12)),
        ];
        for (start, end) in tt {
            let mut exp = start;
            for d in days_julian(&start, &end) {
                assert_eq!(exp, d);
                exp = d.shift_day(1);
            }
        }
    }

    #[test]
    fn test_years_seq() {
        let period = |s: JulianDate, e: JulianDate| JulianPeriod { start: s, end: e };
        let tt: [(JulianPeriod, Vec<JulianPeriod>); 3] = [
            (
                period(jd(2025, 5, 3), jd(2025, 6, 12)),
                vec![period(jd(2025, 5, 3), jd(2025, 6, 12))],
            ),
            (
                period(jd(2025, 5, 3), jd(2025, 5, 3)),
                vec![period(jd(2025, 5, 3), jd(2025, 5, 3))],
            ),
            (
                period(jd(2024, 8, 3), jd(2025, 6, 12)),
                vec![
                    period(jd(2024, 8, 3), jd(2024, 12, 31)),
                    period(jd(2025, 1, 1), jd(2025, 6, 12)),
                ],
            ),
        ];
        for (in_p, exp) in tt {
            let got = years_julian(&in_p.start, &in_p.end);
            assert_eq!(exp.len(), got.len());
            for (i, (year, p)) in got.iter().enumerate() {
                assert_eq!(p.start.year, *year);
                assert_eq!(p.end.year, *year);
                assert_eq!(exp[i], *p);
            }
        }
    }
}

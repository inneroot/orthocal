//! Go `internal/i18ndom` port: i18n helpers for domain objects.

use crate::cal::{GregorianDate, JulianDate};
use crate::i18n::T;
use crate::litcal::ProjectedCommemoration;

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

const WEEKDAYS: [&str; 7] = [
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
];

fn fast_strictness_key(s: i64) -> &'static str {
    match s {
        80 => "no-food",
        70 => "no-cooked-food",
        60 => "no-oil",
        50 => "no-fish",
        40 => "no-caviar",
        30 => "no-dairy",
        20 => "no-meat",
        10 => "free",
        0 => "none",
        _ => "",
    }
}

pub fn month(t: &T, month: i32) -> String {
    t.translate(&format!("month-{}", MONTHS[(month - 1) as usize]), &[])
}

pub fn weekday(t: &T, weekday: i64) -> String {
    t.translate(&format!("weekday-{}", WEEKDAYS[weekday as usize]), &[])
}

pub fn julian_date(t: &T, date: &JulianDate) -> String {
    let wd = weekday(t, date.weekday());
    let mn = month(t, date.month);
    let day = date.day.to_string();
    let year = date.year.to_string();
    t.translate(
        "date-julian",
        &[
            ("weekday", &wd),
            ("month-name", &mn),
            ("day", &day),
            ("year", &year),
        ],
    )
}

pub fn gregorian_date(t: &T, date: &GregorianDate) -> String {
    // Conversion to Julian calendar is possible,
    // because weekday does not change
    let wd = weekday(t, date.to_julian().weekday());
    let mn = month(t, date.month);
    let day = date.day.to_string();
    let year = date.year.to_string();
    t.translate(
        "date-gregorian",
        &[
            ("weekday", &wd),
            ("month-name", &mn),
            ("day", &day),
            ("year", &year),
        ],
    )
}

pub fn title_commemoration(t: &T, c: &ProjectedCommemoration) -> String {
    match c.projection {
        crate::litcal::Projection::Forefeast => {
            t.translate("forefeast-of", &[("name", &c.commemoration.name)])
        }
        crate::litcal::Projection::ForefeastEve => {
            t.translate("eve-of", &[("name", &c.commemoration.name)])
        }
        crate::litcal::Projection::Afterfeast => {
            t.translate("afterfeast-of", &[("name", &c.commemoration.name)])
        }
        crate::litcal::Projection::AfterfeastLeavetaking => {
            t.translate("leavetaking-of", &[("name", &c.commemoration.name)])
        }
        // Projection::StartEnd
        _ => c.commemoration.name.clone(),
    }
}

pub fn fast_strictness(t: &T, strictness: i64) -> String {
    t.translate(
        &format!("fast-strictness-{}", fast_strictness_key(strictness)),
        &[],
    )
}

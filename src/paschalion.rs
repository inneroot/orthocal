//! Go `pkg/paschalion` port: Pascha (Easter) computation.

use crate::cal::JulianDate;
use crate::mmath;

#[allow(dead_code)]
pub const GREAT_INDICTION_LENGTH: i64 = 532;
/// Pascha dates before this year are historically irrelevant.
pub const LOWEST_YEAR: i64 = 33;
pub const HIGHEST_YEAR: i64 = 9999;

/// Computes the date of Pascha according to the Julian calendar.
///
/// The computed date is undefined for years preceding LOWEST_YEAR.
///
/// The Holy Fathers of the Church defined the day of Pascha to be the first
/// Sunday after the first ecclesiastical full moon on or after the
/// ecclesiastical vernal equinox of the Northern Hemisphere.
/// The ecclesiastical vernal equinox is 21 March (Julian).
/// The ecclesiastical full moon is based on the Metonic cycle and not the
/// astronomical event.
pub fn pascha(year: i64) -> JulianDate {
    pascha_meeus(year)
}

/// Jean Meeus, Astronomical Algorithms, 1998, 2nd edition, p. 69
fn pascha_meeus(year: i64) -> JulianDate {
    let a = mmath::r#mod(year, 4);
    let b = mmath::r#mod(year, 7);
    let c = mmath::r#mod(year, 19);
    let d = mmath::r#mod(19 * c + 15, 30);
    let e = mmath::r#mod(2 * a + 4 * b - d + 34, 7);
    let month = (d + e + 114) / 31;
    let day = mmath::r#mod(d + e + 114, 31) + 1;
    JulianDate {
        year,
        month: month as i32,
        day: day as i32,
    }
}

/// Deobfuscated version of the Meeus algorithm.
/// Computational steps are split and ordered to make their correctness easier to understand.
/// Variables with the same name like in the original Meeus algorithm store the same value.
/// Ported for test equivalence only (not used by the CLI).
#[allow(dead_code)]
fn pascha_meeus_deobf(year: i64) -> JulianDate {
    // The golden index is like the traditional golden number, but starts at 0 instead of 1.
    let golden_index = mmath::r#mod(year, 19);

    // epact = mod(golden_index * 11, 30); counterepact = mod(golden_index * 19, 30)
    let counterepact = mmath::r#mod(golden_index * -11, 30);
    // Distance between the ecclesiastical vernal equinox and the next Metonic full moon
    // (the Paschal full moon). Ranges from 0 to 28.
    let d = mmath::r#mod(counterepact + 15, 30);

    // Distance from March 21 to the Sunday on or after March 21, simplified
    // dominical number.
    let a = mmath::r#mod(year, 4);
    let b = mmath::r#mod(year, 7);
    let pascha_shift = mmath::r#mod(2 * a + 4 * b, 7);

    // Distance from the Paschal full moon + 1 to Pascha, relative to the vernal
    // equinox.
    let e = mmath::r#mod(pascha_shift - (d + 1), 7);

    // The distance from the vernal equinox to Pascha. Ranges from 1 to 35 days.
    let f = d + 1 + e;

    // The first 10 days of f are in March. The remaining days of f are in April.
    let march_length: i64 = 31;
    let offset = 4 * march_length - 11; // = 113
    let month = (f + offset) / march_length;
    let day = mmath::r#mod(f + offset, march_length) + 1;
    JulianDate {
        year,
        month: month as i32,
        day: day as i32,
    }
}

/// https://www.calendarbede.com/book/calculation-orthodox-easter-sunday
/// Ported for test equivalence only (not used by the CLI).
#[allow(dead_code)]
fn pascha_byzantine(year: i64) -> JulianDate {
    // Years since the creation of the world, starting at 0
    let creation_of_world = year + 5508;
    // The lunar cycle is related to the golden number:
    // lunar_cycle == mm.Mod(GoldenNumber(year) - 4, 19) + 1
    let lunar_cycle = mmath::r#mod(creation_of_world - 1, 19) + 1;
    // The age of the moon on 1 March
    let mut foundation = mmath::r#mod(11 * lunar_cycle + 3, 30);
    if lunar_cycle > 16 {
        foundation += 1;
    }
    // March date of the first full moon on or after the vernal equinox (21 March)
    let mut ecclesiastical_full_moon = 30 + 14 + 3 - foundation;
    // If the full moon is before 21 March
    if ecclesiastical_full_moon < 21 {
        // Choose the next full moon
        ecclesiastical_full_moon += 30;
    }
    let solar_cycle = mmath::r#mod(creation_of_world - 1, 28) + 1;
    // Which day of the week falls on 1 September
    let vruceleto = mmath::r#mod(solar_cycle + solar_cycle / 4 - 1, 7) + 1;
    // First Sunday of March
    let mut first_sunday = 11 - vruceleto;
    if vruceleto < 4 {
        first_sunday = 4 - vruceleto;
    }
    // Ranges from 22 to 56
    let pascha =
        ecclesiastical_full_moon + 7 - mmath::r#mod(ecclesiastical_full_moon - first_sunday, 7);
    let mut month = 3;
    let mut day = pascha;
    if pascha > 31 {
        month = 4;
        day = pascha - 31;
    }
    JulianDate {
        year,
        month: month as i32,
        day: day as i32,
    }
}

/// Returns the position of the given Julian year within the Metonic cycle, starting at 1.
#[allow(dead_code)]
pub fn golden_number(year: i64) -> i64 {
    mmath::r#mod(year, 19) + 1
}

/// Returns the number of the current Great Indiction and the position within it, starting at 1.
#[allow(dead_code)]
pub fn great_indiction(year: i64) -> (i64, i64) {
    // The first indiction started in -5507 AD = 5508 BC = 1/2 AM
    let since_start_indiction1 = year + 5507;
    let indiction = since_start_indiction1 / GREAT_INDICTION_LENGTH + 1;
    let pos = since_start_indiction1 % GREAT_INDICTION_LENGTH + 1;
    (indiction, pos)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jd(year: i64, month: i32, day: i32) -> JulianDate {
        JulianDate { year, month, day }
    }

    #[test]
    fn test_pascha() {
        // Reference calendars:
        // https://www.holytrinityorthodox.com/htc/orthodox-calendar/
        // https://azbyka.ru/days/
        let tt: [(i64, i32, i32); 17] = [
            (2024, 4, 22),
            (2025, 4, 7),
            (2026, 3, 30),
            (2027, 4, 19),
            (2028, 4, 3),
            (2029, 3, 26),
            (2030, 4, 15),
            (2031, 3, 31),
            (2032, 4, 19),
            (2033, 4, 11),
            (2034, 3, 27),
            (2035, 4, 16),
            (2036, 4, 7),
            (2037, 3, 23),
            (2038, 4, 12),
            (2039, 4, 4),
            (2040, 4, 23),
        ];
        for (year, exp_month, exp_day) in tt {
            let got = pascha(year);
            let exp = jd(year, exp_month, exp_day);
            assert_eq!(exp, got);
        }
    }

    #[test]
    fn test_pascha_equivalence() {
        for year in 1..10_000 {
            assert_eq!(pascha(year), pascha_meeus(year));
            assert_eq!(pascha(year), pascha_meeus_deobf(year));
            assert_eq!(pascha(year), pascha_byzantine(year));
        }
    }

    #[test]
    fn test_great_indiction_pascha() {
        use std::collections::HashMap;
        let mut years: HashMap<i64, JulianDate> = HashMap::new();
        for year in 1..10_000 {
            let (_, indiction_number) = great_indiction(year);
            let indiction_index = indiction_number - 1;
            let d = pascha(year);
            match years.get(&indiction_index) {
                None => {
                    years.insert(indiction_index, d);
                }
                Some(prev) => {
                    assert_eq!(prev.month, d.month);
                    assert_eq!(prev.day, d.day);
                }
            }
        }
    }

    #[test]
    fn test_great_indiction() {
        let tt: [(i64, i64, i64); 16] = [
            (2025, 15, 85),
            (1941, 15, 1),
            (1409, 14, 1),
            (877, 13, 1),
            (345, 12, 1),
            (-187, 11, 1),
            (-719, 10, 1),
            (-1251, 9, 1),
            (-1783, 8, 1),
            (-2315, 7, 1),
            (-2847, 6, 1),
            (-3379, 5, 1),
            (-3911, 4, 1),
            (-4443, 3, 1),
            (-4975, 2, 1),
            (-5507, 1, 1),
        ];
        for (year, exp_i, exp_p) in tt {
            let (i, p) = great_indiction(year);
            assert_eq!((exp_i, exp_p), (i, p));
        }
    }

    #[test]
    fn test_golden_number() {
        // Year 0 (1 BC) was a first year in the Metonic cycle
        let tt: [(i64, i64); 6] = [(-5507, 4), (-19, 1), (-1, 19), (0, 1), (285, 1), (2025, 12)];
        for (year, exp_golden_number) in tt {
            assert_eq!(exp_golden_number, golden_number(year));
        }
    }
}

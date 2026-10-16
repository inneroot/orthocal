//! Go `pkg/litcal` port: the liturgical calendar (movable feasts,
//! commemorations, fasting rules, Scripture readings).

use crate::cal::{self, JulianDate};
use crate::csvp;
use crate::i18n::T;
use crate::paschalion;
use crate::pdqsort::pdqsort;

const COMMEMORATIONS_CSV: &str = include_str!("../assets/commemorations.csv");
const FASTING_RULES_CSV: &str = include_str!("../assets/fasting_rules.csv");
const READINGS_CSV: &str = include_str!("../assets/readings.csv");

const ERR_UNDEFINED_DATE: &str = "undefined date";

/// 'Observance' is a term used to include commemorations, fasting rules and
/// scripture readings.
///
/// The start and end day of an observance are both inclusive.
/// If no end is specified, the observance ends on the start day.
/// A date can be specified as fixed, using month and day.
/// A date can be specified with a distance from Pascha.
///    -1 = 1 day before Pascha
///     0 = the day of Pascha
///     1 = the day of Pascha
///     2 = 1 day after Pascha
///
/// A date can be specified as a weekday before, after, or closest to the date,
/// regardless of whether the date is specified in month and day, or as distance
/// from Pascha.
///
/// nth weekday before or after:
///    -8 = the 2nd Sunday before
///    -7 = the 1st Saturday before
///    -2 = the 1st Monday before
///    -1 = the 1st Sunday before
///     0 = invalid
///     1 = the 1st Sunday after
///     2 = the 1st Monday after
///     7 = the 1st Saturday after
///     8 = the 2nd Sunday after
/// closest weekday:
///     1 = closest Sunday
///     2 = closest Monday
///     7 = closest Saturday
///
/// Regardless of how the start date is specified, if the start date is beyond
/// 'start_month_max' and 'start_day_max', the start date is moved to
/// 'start_month_alt' and 'start_day_alt'.
///
/// Regardless of how the start date is specified, if the distance from pascha of
/// the start date is between 'start_distance_pascha_lower' (inclusive) and
/// 'start_distance_pascha_upper' (inclusive), the start date is moved to
/// 'start_distance_pascha_alt'.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObservancePeriod {
    start_month: Option<i64>,
    start_day: Option<i64>,
    start_month_max: Option<i64>,
    start_day_max: Option<i64>,
    start_month_alt: Option<i64>,
    start_day_alt: Option<i64>,
    start_distance_pascha: Option<i64>,
    start_distance_pascha_lower: Option<i64>,
    start_distance_pascha_upper: Option<i64>,
    start_distance_pascha_alt: Option<i64>,
    start_nth_weekday: Option<i64>,
    start_closest_weekday: Option<i64>,
    end_month: Option<i64>,
    end_day: Option<i64>,
    end_distance_pascha: Option<i64>,
    #[allow(dead_code)]
    end_nth_weekday: Option<i64>,
    end_closest_weekday: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Projection {
    StartEnd,
    Forefeast,
    /// Last day of forefeast
    ForefeastEve,
    Afterfeast,
    /// Last day of afterfeast
    AfterfeastLeavetaking,
}

/// The JSON string value of a projection, matching the Go `Projection` consts.
pub fn projection_str(p: &Projection) -> &'static str {
    match p {
        Projection::StartEnd => "start-end",
        Projection::Forefeast => "forefeast",
        Projection::ForefeastEve => "forefeast-eve",
        Projection::Afterfeast => "afterfeast",
        Projection::AfterfeastLeavetaking => "afterfeast-leavetaking",
    }
}

pub struct LiturgicalDay {
    pub julian_date: JulianDate,
    pub gregorian_date: cal::GregorianDate,
    /// Empty when there are no commemorations (Go marshals this as `null`).
    pub commemorations: Vec<ProjectedCommemoration>,
    pub fasting: ProjectedFastingRule,
    /// Empty when there are no readings (Go marshals this as `null`).
    pub readings: Vec<ProjectedReading>,
}

/// Raw data parsed from the embedded CSV assets, loaded once at startup.
pub struct Data {
    pub commemorations: Vec<RawCommemoration>,
    pub fasting_rules: Vec<RawFastingRule>,
    pub readings: Vec<RawReading>,
}

pub struct RawCommemoration {
    id: i64,
    /// Commemoration levels:
    ///     930 = Season
    ///     910 = Pascha
    ///     880 = Feast of the Lord
    ///     840 = Feast of the Mother of God
    ///     600 = Feast
    ///     550 = Red cross circle (vigil for great feast) (for levels >= 550)
    ///     540 = Red cross half-circle (vigil)
    ///     530 = Red cross (polyeleos)
    ///     520 = Red squigg (doxology)
    ///     510 = Black squigg (six verse)
    ///     400 = Liturgy of the Presanctified Gifts
    ///     300 = Liturgy
    level: i64,
    /// Commemorations with a forefeast number are feasts whose forefeast starts
    /// that number of days before the start of the feast and ends the day before
    /// the start of the feast.
    forefeast_days: Option<i64>,
    period: ObservancePeriod,
    /// Commemorations with an afterfeast number are feasts whose afterfeast starts
    /// the day after the end of the feast and ends that number of days after the
    /// end of the feast.
    /// The leavetaking of a feast is the last day of its afterfeast.
    afterfeast_days: Option<i64>,
    /// Afterfeasts with a limit are shortened such that they do not fall on a day
    /// that has the given pascha distance.
    afterfeast_limit_distance_pascha: Option<i64>,
    source: String,
}

#[derive(Clone)]
pub struct Commemoration {
    pub id: i64,
    pub name: String,
    pub level: i64,
    pub forefeast_start: Option<JulianDate>,
    pub start: JulianDate,
    pub end: JulianDate,
    pub afterfeast_end: Option<JulianDate>,
    pub source: String,
}

/// A [Commemoration] projected onto a specific day.
pub struct ProjectedCommemoration {
    pub commemoration: Commemoration,
    pub projection: Projection,
}

pub struct RawFastingRule {
    id: i64,
    commemoration_id: Option<i64>,
    level: i64,
    period: ObservancePeriod,
    /// If a fasting rule has a weekdays pattern, the fasting rule applies only to
    /// those weekdays during the start and end period.
    /// Common weekdays patterns:
    ///     1111111 = all weekdays (default)
    ///     1000001 = on weekends
    ///     0101010 = on Monday, Wednesday, Friday
    ///     0010100 = on Tuesday, Thursday
    ///     0001010 = on Wednesday, Friday
    weekdays: i64,
    strictness: i64,
}

#[derive(Clone)]
pub struct FastingRule {
    pub id: i64,
    pub commemoration_id: Option<i64>,
    pub name: String,
    pub level: i64,
    pub start: JulianDate,
    pub end: JulianDate,
    pub weekdays: i64,
    pub strictness: i64,
}

pub struct ProjectedFastingRule {
    pub rule: Option<FastingRule>,
    pub strictness: i64,
}

pub struct RawReading {
    id: i64,
    commemoration_id: Option<i64>,
    /// Readings of level 0 are not overwritten by other readings
    level: i64,
    period: ObservancePeriod,
    weekdays: i64,
    book: String,
    range: ReadingRange,
    source: String,
}

#[derive(Clone, Copy)]
pub struct ReadingRange {
    pub start: ReadingPos,
    pub end: ReadingPos,
}

#[derive(Clone, Copy)]
pub struct ReadingPos {
    pub chapter: i64,
    pub verse: i64,
}

#[derive(Clone)]
pub struct Reading {
    pub id: i64,
    pub commemoration_id: Option<i64>,
    pub level: i64,
    pub start: JulianDate,
    pub end: JulianDate,
    pub weekdays: i64,
    pub book: String,
    pub range: ReadingRange,
    pub source: String,
}

pub struct ProjectedReading {
    pub components: Vec<Reading>,
    pub citation: String,
}

impl Data {
    pub fn load() -> Data {
        let mut commemorations = Vec::new();
        for cols in csvp::rows(COMMEMORATIONS_CSV) {
            commemorations.push(RawCommemoration {
                id: csvp::to_int(&cols[0]),
                level: csvp::to_int(&cols[1]),
                forefeast_days: csvp::to_int_opt(&cols[2]),
                period: parse_period(&cols[3..20]),
                afterfeast_days: csvp::to_int_opt(&cols[20]),
                afterfeast_limit_distance_pascha: csvp::to_int_opt(&cols[21]),
                source: cols[22].clone(),
            });
        }
        let mut fasting_rules = Vec::new();
        for cols in csvp::rows(FASTING_RULES_CSV) {
            fasting_rules.push(RawFastingRule {
                id: csvp::to_int(&cols[0]),
                commemoration_id: csvp::to_int_opt(&cols[1]),
                level: csvp::to_int(&cols[2]),
                period: parse_period(&cols[3..20]),
                weekdays: csvp::to_weekdays(&cols[20]),
                strictness: csvp::to_int(&cols[21]),
            });
        }
        let mut readings = Vec::new();
        for cols in csvp::rows(READINGS_CSV) {
            readings.push(RawReading {
                id: csvp::to_int(&cols[0]),
                commemoration_id: csvp::to_int_opt(&cols[1]),
                level: csvp::to_int(&cols[2]),
                period: parse_period(&cols[3..20]),
                weekdays: csvp::to_weekdays(&cols[20]),
                book: cols[21].clone(),
                range: ReadingRange {
                    start: ReadingPos {
                        chapter: csvp::to_int(&cols[22]),
                        verse: csvp::to_int(&cols[23]),
                    },
                    end: ReadingPos {
                        chapter: csvp::to_int(&cols[24]),
                        verse: csvp::to_int(&cols[25]),
                    },
                },
                source: cols[26].clone(),
            });
        }
        Data {
            commemorations,
            fasting_rules,
            readings,
        }
    }
}

fn parse_period(cols: &[String]) -> ObservancePeriod {
    ObservancePeriod {
        start_month: csvp::to_int_opt(&cols[0]),
        start_day: csvp::to_int_opt(&cols[1]),
        start_month_max: csvp::to_int_opt(&cols[2]),
        start_day_max: csvp::to_int_opt(&cols[3]),
        start_month_alt: csvp::to_int_opt(&cols[4]),
        start_day_alt: csvp::to_int_opt(&cols[5]),
        start_distance_pascha: csvp::to_int_opt(&cols[6]),
        start_distance_pascha_lower: csvp::to_int_opt(&cols[7]),
        start_distance_pascha_upper: csvp::to_int_opt(&cols[8]),
        start_distance_pascha_alt: csvp::to_int_opt(&cols[9]),
        start_nth_weekday: csvp::to_int_opt(&cols[10]),
        start_closest_weekday: csvp::to_int_opt(&cols[11]),
        end_month: csvp::to_int_opt(&cols[12]),
        end_day: csvp::to_int_opt(&cols[13]),
        end_distance_pascha: csvp::to_int_opt(&cols[14]),
        end_nth_weekday: csvp::to_int_opt(&cols[15]),
        end_closest_weekday: csvp::to_int_opt(&cols[16]),
    }
}

/// Loads the liturgical calendar between start and end (inclusive).
pub fn load(
    start: &JulianDate,
    end: &JulianDate,
    no_readings: bool,
    t: &T,
    data: &Data,
) -> Result<Vec<LiturgicalDay>, String> {
    let mut days: Vec<LiturgicalDay> = Vec::new();
    for (year, period) in cal::years_julian(start, end) {
        let pascha_prev = paschalion::pascha(year - 1);
        let pascha = paschalion::pascha(year);
        let pascha_next = paschalion::pascha(year + 1);
        let cs = compile_commemorations(&pascha, t, data)?;
        let frs = compile_fasting_rules(&pascha, t, data)?;
        let mut readings: Vec<Reading> = Vec::new();
        if !no_readings {
            // Compile readings for surrounding years, because readings related to
            // Pascha on one year could fall into the previous or next year.
            readings.extend(compile_readings(&pascha_prev, data)?);
            readings.extend(compile_readings(&pascha, data)?);
            readings.extend(compile_readings(&pascha_next, data)?);
        }
        for d in cal::days_julian(&period.start, &period.end) {
            let pcs = project_commemorations(&d, &cs);
            let pfr = project_fasting_rules(&d, &frs);
            let prs = project_readings(&d, &readings, t);
            let dg = d.to_gregorian();
            days.push(LiturgicalDay {
                julian_date: d,
                gregorian_date: dg,
                commemorations: pcs,
                fasting: pfr,
                readings: prs,
            });
        }
    }
    Ok(days)
}

fn compile_commemorations(
    pascha: &JulianDate,
    t: &T,
    data: &Data,
) -> Result<Vec<Commemoration>, String> {
    data.commemorations
        .iter()
        .map(|raw| raw.compile(pascha, t))
        .collect()
}

impl RawCommemoration {
    fn compile(&self, pascha: &JulianDate, t: &T) -> Result<Commemoration, String> {
        let start = derive_start(pascha, &self.period)
            .map_err(|e| format!("{}: commemoration id {}", e, self.id))?;
        let end = derive_end(pascha, &self.period, &start)
            .map_err(|e| format!("{}: commemoration id {}", e, self.id))?;

        let mut forefeast_start: Option<JulianDate> = None;
        if let Some(forefeast_days) = self.forefeast_days {
            forefeast_start = Some(start.shift_day(-forefeast_days));
        }

        let mut afterfeast_end: Option<JulianDate> = None;
        if let Some(afterfeast_days) = self.afterfeast_days {
            let mut r = end.shift_day(afterfeast_days);
            afterfeast_end = Some(r);
            // Shorten afterfeast to make it end before the limit date
            if let Some(limit_dp) = self.afterfeast_limit_distance_pascha {
                let limit = pascha_shift(pascha, limit_dp);
                if !r.before(&limit) {
                    r = limit.shift_day(-1);
                    afterfeast_end = Some(r);
                }
                if r.before_equal(&end) {
                    afterfeast_end = None;
                }
            }
        }

        Ok(Commemoration {
            id: self.id,
            name: t.translate(&format!("commemoration-{}", self.id), &[]),
            level: self.level,
            forefeast_start,
            start,
            end,
            afterfeast_end,
            source: self.source.clone(),
        })
    }
}

fn project_commemorations(d: &JulianDate, cs: &[Commemoration]) -> Vec<ProjectedCommemoration> {
    let mut pcs: Vec<ProjectedCommemoration> =
        cs.iter().filter_map(|c| c.project_onto(d)).collect();
    // Go: slices.SortFunc by (b.Level - a.Level); ported pdqsort keeps the
    // exact same order for equal levels.
    pdqsort(
        &mut pcs,
        &|a: &ProjectedCommemoration, b: &ProjectedCommemoration| {
            b.commemoration.level - a.commemoration.level
        },
    );
    pcs
}

impl Commemoration {
    fn project_onto(&self, day: &JulianDate) -> Option<ProjectedCommemoration> {
        let projection = if day.between(&self.start, &self.end) {
            Projection::StartEnd
        } else if let Some(forefeast_start) = self.forefeast_start {
            let forefeast_end = self.start.shift_day(-1);
            if *day == forefeast_end {
                Projection::ForefeastEve
            } else if day.between(&forefeast_start, &forefeast_end) {
                Projection::Forefeast
            } else {
                return None;
            }
        } else if let Some(afterfeast_end) = self.afterfeast_end {
            if *day == afterfeast_end {
                Projection::AfterfeastLeavetaking
            } else if day.between(&self.end.shift_day(1), &afterfeast_end) {
                Projection::Afterfeast
            } else {
                return None;
            }
        } else {
            return None;
        };
        Some(ProjectedCommemoration {
            commemoration: self.clone(),
            projection,
        })
    }
}

fn compile_fasting_rules(
    pascha: &JulianDate,
    t: &T,
    data: &Data,
) -> Result<Vec<FastingRule>, String> {
    data.fasting_rules
        .iter()
        .map(|raw| raw.compile(pascha, t))
        .collect()
}

impl RawFastingRule {
    fn compile(&self, pascha: &JulianDate, t: &T) -> Result<FastingRule, String> {
        let start = derive_start(pascha, &self.period)
            .map_err(|e| format!("{}: fasting rule id {}", e, self.id))?;
        let end = derive_end(pascha, &self.period, &start)
            .map_err(|e| format!("{}: fasting rule id {}", e, self.id))?;
        let mut name = t.translate(&format!("fast-{}", self.id), &[]);
        if let Some(cid) = self.commemoration_id {
            name = t.translate(&format!("commemoration-{}", cid), &[]);
        }
        Ok(FastingRule {
            id: self.id,
            commemoration_id: self.commemoration_id,
            name,
            level: self.level,
            start,
            end,
            weekdays: self.weekdays,
            strictness: self.strictness,
        })
    }
}

fn project_fasting_rules(d: &JulianDate, frs: &[FastingRule]) -> ProjectedFastingRule {
    let mut max_level = -1;
    let mut winner: Option<ProjectedFastingRule> = None;
    for fr in frs {
        // Keep the fasting rule with the highest level (first one wins ties)
        if let Some(p) = fr.project_onto(d) {
            if fr.level > max_level {
                max_level = fr.level;
                winner = Some(p);
            }
        }
    }
    match winner {
        Some(w) => w,
        None => ProjectedFastingRule {
            rule: None,
            strictness: 0,
        },
    }
}

impl FastingRule {
    fn project_onto(&self, day: &JulianDate) -> Option<ProjectedFastingRule> {
        if day.between(&self.start, &self.end) && matches_weekdays_pattern(self.weekdays, day) {
            Some(ProjectedFastingRule {
                rule: Some(self.clone()),
                strictness: self.strictness,
            })
        } else {
            None
        }
    }
}

fn compile_readings(pascha: &JulianDate, data: &Data) -> Result<Vec<Reading>, String> {
    data.readings
        .iter()
        .map(|raw| raw.compile(pascha))
        .collect()
}

impl RawReading {
    fn compile(&self, pascha: &JulianDate) -> Result<Reading, String> {
        let start = derive_start(pascha, &self.period)
            .map_err(|e| format!("{}: reading id {}", e, self.id))?;
        let end = derive_end(pascha, &self.period, &start)
            .map_err(|e| format!("{}: reading id {}", e, self.id))?;
        Ok(Reading {
            id: self.id,
            commemoration_id: self.commemoration_id,
            level: self.level,
            start,
            end,
            weekdays: self.weekdays,
            book: self.book.clone(),
            range: self.range,
            source: self.source.clone(),
        })
    }
}

fn project_readings(d: &JulianDate, rs: &[Reading], t: &T) -> Vec<ProjectedReading> {
    // Find readings of the day, group them and find max level.
    // Go iterates a map here (random order); this port keeps a deterministic
    // first-seen (CSV) order instead.
    let mut max_level = -1;
    let mut groups: Vec<(String, Vec<Reading>)> = Vec::new();
    let mut group_index: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for r in rs {
        if !r.project_onto(d) {
            continue;
        }
        let key = format!(
            "{}.{}.{}",
            r.book,
            r.level,
            r.commemoration_id
                .map_or("<nil>".to_string(), |c| c.to_string())
        );
        let g = match group_index.get(&key) {
            Some(&i) => &mut groups[i],
            None => {
                groups.push((key.clone(), Vec::new()));
                let i = groups.len() - 1;
                group_index.insert(key, i);
                &mut groups[i]
            }
        };
        g.1.push(r.clone());
        if r.level > max_level {
            max_level = r.level;
        }
    }
    // Keep readings with correct level and make citation string
    let mut prs: Vec<ProjectedReading> = Vec::new();
    for (_, mergeable_readings) in groups {
        if mergeable_readings[0].level == 0 || mergeable_readings[0].level == max_level {
            let citation = citation(&mergeable_readings, t);
            prs.push(ProjectedReading {
                components: mergeable_readings,
                citation,
            });
        }
    }
    pdqsort(&mut prs, &|a: &ProjectedReading, b: &ProjectedReading| {
        b.components[0].level - a.components[0].level
    });
    prs
}

impl Reading {
    fn project_onto(&self, day: &JulianDate) -> bool {
        day.between(&self.start, &self.end) && matches_weekdays_pattern(self.weekdays, day)
    }
}

/// Compiles the citation string for the given readings.
/// It is assumed that all readings have the same book.
fn citation(rs: &[Reading], t: &T) -> String {
    let book = t.translate(&format!("book-{}", rs[0].book), &[]);
    let mut rngs: Vec<ReadingRange> = rs.iter().map(|r| r.range).collect();
    // Go sorts with slices.SortFunc (pdqsort); the port keeps the same order.
    pdqsort(&mut rngs, &|a: &ReadingRange, b: &ReadingRange| {
        if a.start.chapter == b.start.chapter {
            a.start.verse - b.start.verse
        } else {
            a.start.chapter - b.start.chapter
        }
    });
    let rng_text = pretty_range(&rngs);
    format!("{} {}", book, rng_text)
}

fn pretty_range(rngs: &[ReadingRange]) -> String {
    let mut s = String::new();
    let mut prev_chapter = -1;
    for (i, rng) in rngs.iter().enumerate() {
        if rng.start.chapter == prev_chapter {
            s.push_str(", ");
            s.push_str(&rng.start.verse.to_string());
        } else {
            if i > 0 {
                s.push_str("; ");
            }
            s.push_str(&format!("{}:{}", rng.start.chapter, rng.start.verse));
        }
        s.push_str(&range_end(rng));
        prev_chapter = rng.start.chapter;
    }
    s
}

fn range_end(rng: &ReadingRange) -> String {
    if rng.start.chapter == rng.end.chapter {
        if rng.start.verse == rng.end.verse {
            String::new()
        } else {
            format!("-{}", rng.end.verse)
        }
    } else {
        format!("-{}:{}", rng.end.chapter, rng.end.verse)
    }
}

fn pascha_shift(pascha: &JulianDate, days: i64) -> JulianDate {
    // The day of pascha is the 1st day after pascha
    if days <= 0 {
        pascha.shift_day(days)
    } else {
        pascha.shift_day(days - 1)
    }
}

fn matches_weekdays_pattern(weekdays_pattern: i64, day: &JulianDate) -> bool {
    // Convert weekday number of day to weekdays pattern
    let w = 0b1000000 >> day.weekday();
    // If weekday matches weekdays pattern of observance
    w & weekdays_pattern > 0
}

fn derive_start(pascha: &JulianDate, p: &ObservancePeriod) -> Result<JulianDate, String> {
    let start =
        abs_rel(pascha, p.start_month, p.start_day, p.start_distance_pascha).map_err(|e| {
            if e == ERR_UNDEFINED_DATE {
                "undefined observance start".to_string()
            } else {
                e
            }
        })?;
    let initial_year = start.year;
    let start = rel_weekday(&start, p.start_nth_weekday, p.start_closest_weekday)?;
    let start = alt_abs(
        &start,
        initial_year,
        p.start_month_max,
        p.start_day_max,
        p.start_month_alt,
        p.start_day_alt,
    )?;
    let start = alt_distance_pascha(
        &start,
        pascha,
        p.start_distance_pascha_lower,
        p.start_distance_pascha_upper,
        p.start_distance_pascha_alt,
    );
    Ok(start)
}

fn derive_end(
    pascha: &JulianDate,
    p: &ObservancePeriod,
    default_end: &JulianDate,
) -> Result<JulianDate, String> {
    match abs_rel(pascha, p.end_month, p.end_day, p.end_distance_pascha) {
        Err(e) if e == ERR_UNDEFINED_DATE => Ok(*default_end),
        Err(e) => Err(e),
        Ok(end) => {
            // NOTE: the Go original passes EndClosestWeekday for both
            // distanceWeekday and closestWeekday; ported verbatim.
            rel_weekday(&end, p.end_closest_weekday, p.end_closest_weekday)
        }
    }
}

fn abs_rel(
    pascha: &JulianDate,
    month: Option<i64>,
    day: Option<i64>,
    distance_pascha: Option<i64>,
) -> Result<JulianDate, String> {
    // Define absolutely
    if let (Some(month), Some(day)) = (month, day) {
        return Ok(JulianDate {
            year: pascha.year,
            month: month as i32,
            day: day as i32,
        });
        // Define relative to pascha
    } else if let Some(dp) = distance_pascha {
        return Ok(pascha_shift(pascha, dp));
    }
    Err(ERR_UNDEFINED_DATE.to_string())
}

fn rel_weekday(
    date: &JulianDate,
    distance_weekday: Option<i64>,
    closest_weekday: Option<i64>,
) -> Result<JulianDate, String> {
    // Calculate relative to weekday
    let mut date = *date;
    if let Some(nth) = distance_weekday {
        date = date.nth_weekday(nth)?;
    } else if let Some(closest) = closest_weekday {
        date = date.closest_weekday(closest)?;
    }
    Ok(date)
}

/// Returns the alternative date if the given date is beyond the maximum date.
fn alt_abs(
    date: &JulianDate,
    year_target: i64,
    month_max: Option<i64>,
    day_max: Option<i64>,
    month_alt: Option<i64>,
    day_alt: Option<i64>,
) -> Result<JulianDate, String> {
    if let (Some(month_max), Some(day_max), Some(month_alt), Some(day_alt)) =
        (month_max, day_max, month_alt, day_alt)
    {
        let max_date = cal::new_julian_date(year_target, month_max as i32, day_max as i32)?;
        let alt_date = cal::new_julian_date(year_target, month_alt as i32, day_alt as i32)?;
        if max_date.before(date) {
            return Ok(alt_date);
        }
    }
    Ok(*date)
}

/// Returns the date with the alternative pascha distance, if the given date has
/// a pascha distance that is between the lower and upper pascha distances.
fn alt_distance_pascha(
    date: &JulianDate,
    pascha: &JulianDate,
    lower_dp: Option<i64>,
    upper_dp: Option<i64>,
    alt_dp: Option<i64>,
) -> JulianDate {
    if let (Some(lower_dp), Some(upper_dp), Some(alt_dp)) = (lower_dp, upper_dp, alt_dp) {
        let lower_bound = pascha_shift(pascha, lower_dp);
        let upper_bound = pascha_shift(pascha, upper_dp);
        if date.between(&lower_bound, &upper_bound) {
            return pascha_shift(pascha, alt_dp);
        }
    }
    *date
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pos(chapter: i64, verse: i64) -> ReadingPos {
        ReadingPos { chapter, verse }
    }

    fn rng(start_chapter: i64, start_verse: i64, end_chapter: i64, end_verse: i64) -> ReadingRange {
        ReadingRange {
            start: pos(start_chapter, start_verse),
            end: pos(end_chapter, end_verse),
        }
    }

    #[test]
    fn test_pretty_range() {
        let tt: [(String, Vec<ReadingRange>); 9] = [
            ("5:8-6:1".to_string(), vec![rng(5, 8, 6, 1)]),
            ("5:2-19".to_string(), vec![rng(5, 2, 5, 19)]),
            (
                "5:2-45:1; 60:2-3".to_string(),
                vec![rng(5, 2, 45, 1), rng(60, 2, 60, 3)],
            ),
            (
                "5:2-8:1; 8:3-5".to_string(),
                vec![rng(5, 2, 8, 1), rng(8, 3, 8, 5)],
            ),
            (
                "1:2-4, 7-9".to_string(),
                vec![rng(1, 2, 1, 4), rng(1, 7, 1, 9)],
            ),
            (
                "1:2-4, 7".to_string(),
                vec![rng(1, 2, 1, 4), rng(1, 7, 1, 7)],
            ),
            ("1:2, 7".to_string(), vec![rng(1, 2, 1, 2), rng(1, 7, 1, 7)]),
            (
                "1:2-4, 3".to_string(),
                vec![rng(1, 2, 1, 4), rng(1, 3, 1, 3)],
            ),
            (
                "1:3-7, 4-5".to_string(),
                vec![rng(1, 3, 1, 7), rng(1, 4, 1, 5)],
            ),
        ];
        for (exp, in_) in tt {
            assert_eq!(exp, pretty_range(&in_));
        }
    }
}

//! Date and time operations for the Tarqeem runtime.
//!
//! Backs the date builtins of the `وقت` stdlib module, which codegen has always
//! declared while nothing defined them, so any program reaching one failed to
//! link (#241).
//!
//! No interpreter implementation exists to diff against, so the conventions
//! fixed here are the contract the other backends must match:
//!
//! - Weekdays run 0 = الأحد through 6 = السبت, matching the stdlib's `أيام_عربي`.
//! - Week numbers follow ISO-8601: week 1 is the one holding the first Thursday.
//! - `DDD`/`MMM` render Arabic names, matching `أيام_عربي`/`أشهر_عربي`.
//! - An impossible date yields [`INVALID`] rather than a panic. Unwinding across
//!   the C ABI is undefined behaviour and this crate has no `catch_unwind`.

use crate::string::trq_string_new;
use crate::types::TrqString;

/// Returned by every integer-valued function here when handed a date that does
/// not exist. No correct answer can collide with it, so a caller cannot mistake
/// a failure for a result.
pub const INVALID: i64 = i64::MIN;

const DAY_NAMES: [&str; 7] = [
    "الأحد",
    "الاثنين",
    "الثلاثاء",
    "الأربعاء",
    "الخميس",
    "الجمعة",
    "السبت",
];

/// 1-indexed, mirroring `أشهر_عربي`, whose slot 0 is deliberately empty.
const MONTH_NAMES: [&str; 13] = [
    "",
    "يناير",
    "فبراير",
    "مارس",
    "أبريل",
    "مايو",
    "يونيو",
    "يوليو",
    "أغسطس",
    "سبتمبر",
    "أكتوبر",
    "نوفمبر",
    "ديسمبر",
];

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn month_length(year: i64, month: i64) -> Option<i64> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap(year) => 29,
        2 => 28,
        _ => return None,
    })
}

fn is_valid_date(year: i64, month: i64, day: i64) -> bool {
    month_length(year, month).is_some_and(|len| (1..=len).contains(&day))
}

fn is_valid_time(hour: i64, minute: i64, second: i64, milli: i64) -> bool {
    (0..24).contains(&hour)
        && (0..60).contains(&minute)
        // 60 admits a leap second, which a caller reading a real clock can see.
        && (0..=60).contains(&second)
        && (0..1000).contains(&milli)
}

/// Days since 1970-01-01, by Howard Hinnant's `days_from_civil`. Exact for the
/// whole proleptic Gregorian range, with no lookup table and no overflow for
/// any year an `i64` can hold.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let year_of_era = y - era * 400;
    let shifted_month = (month + 9) % 12;
    let day_of_era_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_era_year;
    era * 146097 + day_of_era - 719468
}

/// 0 = Sunday. 1970-01-01 was a Thursday, hence the offset.
fn weekday(year: i64, month: i64, day: i64) -> i64 {
    (days_from_civil(year, month, day) + 4).rem_euclid(7)
}

/// 1 = Monday .. 7 = Sunday, as ISO-8601 counts them.
fn iso_weekday(year: i64, month: i64, day: i64) -> i64 {
    match weekday(year, month, day) {
        0 => 7,
        other => other,
    }
}

fn day_of_year(year: i64, month: i64, day: i64) -> i64 {
    days_from_civil(year, month, day) - days_from_civil(year, 1, 1) + 1
}

/// A year holds 53 ISO weeks exactly when it starts on a Thursday, or is a leap
/// year starting on a Wednesday.
fn iso_weeks_in_year(year: i64) -> i64 {
    let first = iso_weekday(year, 1, 1);
    if first == 4 || (is_leap(year) && first == 3) {
        53
    } else {
        52
    }
}

fn iso_week(year: i64, month: i64, day: i64) -> i64 {
    let week = (day_of_year(year, month, day) - iso_weekday(year, month, day) + 10) / 7;
    if week < 1 {
        iso_weeks_in_year(year - 1)
    } else if week > iso_weeks_in_year(year) {
        1
    } else {
        week
    }
}

/// Substitute the longest matching directive at each position, copying anything
/// else through verbatim. `fields` must be ordered longest-token-first so that
/// `DDD` wins over `DD` and `MMM` over `MM`.
fn apply_pattern(pattern: &str, fields: &[(&str, String)]) -> String {
    let mut out = String::with_capacity(pattern.len());
    let mut rest = pattern;

    while !rest.is_empty() {
        match fields.iter().find(|(token, _)| rest.starts_with(token)) {
            Some((token, value)) => {
                out.push_str(value);
                rest = &rest[token.len()..];
            }
            None => {
                let mut chars = rest.chars();
                match chars.next() {
                    Some(ch) => {
                        out.push(ch);
                        rest = chars.as_str();
                    }
                    None => break,
                }
            }
        }
    }

    out
}

fn empty_string() -> *mut TrqString {
    trq_string_new(std::ptr::null(), 0)
}

fn into_trq_string(text: &str) -> *mut TrqString {
    trq_string_new(text.as_ptr(), text.len() as i64)
}

/// # Safety
///
/// `s` must be null or a valid pointer to a `TrqString`.
unsafe fn as_str<'a>(s: *const TrqString) -> Option<&'a str> {
    if s.is_null() || (*s).data.is_null() || (*s).len < 0 {
        return None;
    }
    let bytes = std::slice::from_raw_parts((*s).data as *const u8, (*s).len as usize);
    std::str::from_utf8(bytes).ok()
}

fn date_fields(year: i64, month: i64, day: i64) -> Vec<(&'static str, String)> {
    vec![
        ("YYYY", format!("{:04}", year)),
        (
            "DDD",
            DAY_NAMES[weekday(year, month, day) as usize].to_string(),
        ),
        ("MMM", MONTH_NAMES[month as usize].to_string()),
        ("MM", format!("{:02}", month)),
        ("DD", format!("{:02}", day)),
    ]
}

fn time_fields(hour: i64, minute: i64, second: i64) -> Vec<(&'static str, String)> {
    vec![
        ("HH", format!("{:02}", hour)),
        ("mm", format!("{:02}", minute)),
        ("ss", format!("{:02}", second)),
    ]
}

/// Weekday of a date, 0 = الأحد. [`INVALID`] if the date does not exist.
#[no_mangle]
pub extern "C" fn trq_day_of_week(year: i64, month: i64, day: i64) -> i64 {
    if is_valid_date(year, month, day) {
        weekday(year, month, day)
    } else {
        INVALID
    }
}

/// Ordinal day within its year, 1-based. [`INVALID`] if the date does not exist.
#[no_mangle]
pub extern "C" fn trq_day_of_year(year: i64, month: i64, day: i64) -> i64 {
    if is_valid_date(year, month, day) {
        day_of_year(year, month, day)
    } else {
        INVALID
    }
}

/// ISO-8601 week number. Days in early January can belong to week 52 or 53 of
/// the previous year, and late December to week 1 of the next.
#[no_mangle]
pub extern "C" fn trq_week_number(year: i64, month: i64, day: i64) -> i64 {
    if is_valid_date(year, month, day) {
        iso_week(year, month, day)
    } else {
        INVALID
    }
}

/// Length of a month in days, accounting for leap years.
#[no_mangle]
pub extern "C" fn trq_days_in_month(year: i64, month: i64) -> i64 {
    month_length(year, month).unwrap_or(INVALID)
}

/// Whole days from the first date to the second; negative when the second is
/// earlier. [`INVALID`] if either date does not exist.
#[no_mangle]
pub extern "C" fn trq_date_diff_days(
    year1: i64,
    month1: i64,
    day1: i64,
    year2: i64,
    month2: i64,
    day2: i64,
) -> i64 {
    if is_valid_date(year1, month1, day1) && is_valid_date(year2, month2, day2) {
        days_from_civil(year2, month2, day2) - days_from_civil(year1, month1, day1)
    } else {
        INVALID
    }
}

/// Render a date through a pattern of `YYYY`, `MM`, `DD`, `DDD`, `MMM`.
///
/// Returns an empty string for an impossible date or unreadable pattern; the
/// caller owns the result and must release it.
///
/// # Safety
///
/// `pattern` must be null or a valid pointer to a `TrqString`.
#[no_mangle]
pub extern "C" fn trq_date_format(
    year: i64,
    month: i64,
    day: i64,
    pattern: *const TrqString,
) -> *mut TrqString {
    if !is_valid_date(year, month, day) {
        return empty_string();
    }
    let Some(pattern) = (unsafe { as_str(pattern) }) else {
        return empty_string();
    };
    into_trq_string(&apply_pattern(pattern, &date_fields(year, month, day)))
}

/// Render a time through a pattern of `HH`, `mm`, `ss`, `SSS`.
///
/// # Safety
///
/// `pattern` must be null or a valid pointer to a `TrqString`.
#[no_mangle]
pub extern "C" fn trq_time_format(
    hour: i64,
    minute: i64,
    second: i64,
    milli: i64,
    pattern: *const TrqString,
) -> *mut TrqString {
    if !is_valid_time(hour, minute, second, milli) {
        return empty_string();
    }
    let Some(pattern) = (unsafe { as_str(pattern) }) else {
        return empty_string();
    };

    let mut fields = vec![("SSS", format!("{:03}", milli))];
    fields.extend(time_fields(hour, minute, second));
    into_trq_string(&apply_pattern(pattern, &fields))
}

/// Render a date and time through the union of both pattern sets. There is no
/// millisecond parameter, so `SSS` is not available here.
///
/// # Safety
///
/// `pattern` must be null or a valid pointer to a `TrqString`.
#[no_mangle]
pub extern "C" fn trq_datetime_format(
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    pattern: *const TrqString,
) -> *mut TrqString {
    if !is_valid_date(year, month, day) || !is_valid_time(hour, minute, second, 0) {
        return empty_string();
    }
    let Some(pattern) = (unsafe { as_str(pattern) }) else {
        return empty_string();
    };

    let mut fields = date_fields(year, month, day);
    fields.extend(time_fields(hour, minute, second));
    into_trq_string(&apply_pattern(pattern, &fields))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::trq_release;

    fn formatted(result: *mut TrqString) -> String {
        unsafe {
            let s = &*result;
            let bytes = std::slice::from_raw_parts(s.data as *const u8, s.len as usize);
            let text = String::from_utf8_lossy(bytes).into_owned();
            trq_release(result as *mut u8);
            text
        }
    }

    fn pattern(text: &str) -> *mut TrqString {
        trq_string_new(text.as_ptr(), text.len() as i64)
    }

    #[test]
    fn test_weekday_matches_known_dates() {
        // 1970-01-01 was a Thursday, index 4 in أيام_عربي.
        assert_eq!(trq_day_of_week(1970, 1, 1), 4);
        assert_eq!(trq_day_of_week(2000, 1, 1), 6); // Saturday
        assert_eq!(trq_day_of_week(2024, 1, 15), 1); // Monday
    }

    #[test]
    fn test_weekday_before_the_epoch() {
        // The rem_euclid guard: a plain `%` would answer negatively here.
        assert_eq!(trq_day_of_week(1969, 12, 31), 3); // Wednesday
        assert_eq!(trq_day_of_week(1900, 1, 1), 1); // Monday
    }

    #[test]
    fn test_days_in_month_handles_leap_years() {
        assert_eq!(trq_days_in_month(2024, 2), 29);
        assert_eq!(trq_days_in_month(2023, 2), 28);
        assert_eq!(trq_days_in_month(1900, 2), 28); // century, not a leap year
        assert_eq!(trq_days_in_month(2000, 2), 29); // divisible by 400
        assert_eq!(trq_days_in_month(2024, 13), INVALID);
    }

    #[test]
    fn test_day_of_year() {
        assert_eq!(trq_day_of_year(2024, 1, 1), 1);
        assert_eq!(trq_day_of_year(2024, 12, 31), 366); // leap
        assert_eq!(trq_day_of_year(2023, 12, 31), 365);
        assert_eq!(trq_day_of_year(2024, 3, 1), 61);
    }

    #[test]
    fn test_iso_week_boundaries() {
        assert_eq!(trq_week_number(2024, 1, 15), 3);
        // 2021-01-01 was a Friday, so it belongs to week 53 of 2020.
        assert_eq!(trq_week_number(2021, 1, 1), 53);
        // 2019-12-30 was a Monday, already week 1 of 2020.
        assert_eq!(trq_week_number(2019, 12, 30), 1);
        assert_eq!(trq_week_number(2020, 1, 1), 1);
    }

    #[test]
    fn test_date_diff_days() {
        assert_eq!(trq_date_diff_days(2024, 1, 1, 2024, 1, 31), 30);
        assert_eq!(trq_date_diff_days(2024, 1, 31, 2024, 1, 1), -30);
        assert_eq!(trq_date_diff_days(2024, 1, 1, 2024, 1, 1), 0);
        // Spans the leap day.
        assert_eq!(trq_date_diff_days(2024, 2, 28, 2024, 3, 1), 2);
        assert_eq!(trq_date_diff_days(2023, 2, 28, 2023, 3, 1), 1);
    }

    #[test]
    fn test_impossible_dates_are_rejected() {
        assert_eq!(trq_day_of_week(2023, 2, 29), INVALID);
        assert_eq!(trq_day_of_week(2024, 0, 1), INVALID);
        assert_eq!(trq_day_of_year(2024, 4, 31), INVALID);
        assert_eq!(trq_date_diff_days(2024, 1, 1, 2023, 2, 29), INVALID);
    }

    #[test]
    fn test_date_format_directives() {
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, pattern("YYYY-MM-DD"))),
            "2024-01-15"
        );
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, pattern("YYYY/MM/DD"))),
            "2024/01/15"
        );
    }

    #[test]
    fn test_date_format_uses_arabic_names() {
        // DDD must beat DD and MMM must beat MM, or these read as "15D"/"01M".
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, pattern("DDD"))),
            "الاثنين"
        );
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, pattern("MMM"))),
            "يناير"
        );
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, pattern("DDD DD MMM YYYY"))),
            "الاثنين 15 يناير 2024"
        );
    }

    #[test]
    fn test_pattern_copies_unknown_text_through() {
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, pattern("في YYYY"))),
            "في 2024"
        );
        assert_eq!(formatted(trq_date_format(2024, 1, 15, pattern(""))), "");
    }

    #[test]
    fn test_time_format() {
        assert_eq!(
            formatted(trq_time_format(14, 30, 5, 250, pattern("HH:mm:ss.SSS"))),
            "14:30:05.250"
        );
        // MM is a month token and must not be mistaken for minutes.
        assert_eq!(
            formatted(trq_time_format(9, 5, 0, 0, pattern("HH:mm"))),
            "09:05"
        );
    }

    #[test]
    fn test_datetime_format_mixes_both_sets() {
        assert_eq!(
            formatted(trq_datetime_format(
                2024,
                1,
                15,
                14,
                30,
                5,
                pattern("YYYY-MM-DD HH:mm:ss")
            )),
            "2024-01-15 14:30:05"
        );
    }

    #[test]
    fn test_invalid_input_yields_empty_string() {
        assert_eq!(formatted(trq_date_format(2023, 2, 29, pattern("YYYY"))), "");
        assert_eq!(formatted(trq_time_format(25, 0, 0, 0, pattern("HH"))), "");
        assert_eq!(
            formatted(trq_date_format(2024, 1, 15, std::ptr::null())),
            ""
        );
    }
}

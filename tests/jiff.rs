#![cfg(feature = "jiff")]

use cron::Schedule;
use jiff::tz::TimeZone;
use jiff::{Timestamp, ZonedDateTime};
use std::str::FromStr;

fn zoned(text: &str) -> ZonedDateTime {
    text.parse().unwrap()
}

fn strings(datetimes: impl Iterator<Item = ZonedDateTime>) -> Vec<String> {
    datetimes.map(|datetime| datetime.to_string()).collect()
}

#[test]
fn test_readme_schedule() {
    let schedule =
        Schedule::from_str("0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2").unwrap();
    let start = zoned("2018-01-01T00:00:00+00:00[UTC]");
    assert_eq!(
        strings(schedule.after_zoned(&start).take(5)),
        [
            "2018-06-01T09:30:00+00:00[UTC]",
            "2018-06-01T12:30:00+00:00[UTC]",
            "2018-06-01T15:30:00+00:00[UTC]",
            "2018-06-15T09:30:00+00:00[UTC]",
            "2018-06-15T12:30:00+00:00[UTC]",
        ]
    );
}

#[test]
fn test_upcoming_zoned_is_in_the_future() {
    let schedule = Schedule::from_str("0 0 0 1 1 * *").unwrap();
    let before = Timestamp::now();
    let next = schedule.upcoming_zoned(TimeZone::UTC).next().unwrap();
    assert!(next.timestamp() > before);
    assert_eq!(next.time_zone(), &TimeZone::UTC);
    let next_owned = schedule.upcoming_zoned_owned(TimeZone::UTC).next().unwrap();
    assert_eq!(next_owned, next);
}

#[test]
fn test_after_zoned_and_back() {
    let schedule = Schedule::from_str("0 5,13,40-42 17 1 Jan *").unwrap();
    let start = zoned("2023-01-01T17:13:00+00:00[UTC]");
    let mut iter = schedule.after_zoned(&start);
    assert_eq!(
        iter.next().unwrap().to_string(),
        "2023-01-01T17:40:00+00:00[UTC]"
    );
    assert_eq!(
        iter.next_back().unwrap().to_string(),
        "2023-01-01T17:13:00+00:00[UTC]"
    );
    assert_eq!(
        iter.next_back().unwrap().to_string(),
        "2023-01-01T17:05:00+00:00[UTC]"
    );
    assert_eq!(
        iter.next_back().unwrap().to_string(),
        "2022-01-01T17:42:00+00:00[UTC]"
    );
}

#[test]
fn test_after_zoned_owned() {
    let schedule = Schedule::from_str("0 0 * * * * *").unwrap();
    let start = zoned("2024-05-01T10:30:00+02:00[Europe/Berlin]");
    let mut iter = schedule.after_zoned_owned(start.clone());
    assert_eq!(
        iter.next().unwrap().to_string(),
        "2024-05-01T11:00:00+02:00[Europe/Berlin]"
    );
    assert_eq!(
        strings(schedule.after_zoned_owned(start).rev().take(2)),
        [
            "2024-05-01T10:00:00+02:00[Europe/Berlin]",
            "2024-05-01T09:00:00+02:00[Europe/Berlin]",
        ]
    );
}

#[test]
fn test_includes_zoned() {
    let schedule = Schedule::from_str("0 30 9 * * Mon-Fri *").unwrap();
    // 2024-06-03 is a Monday and 2024-06-02 is a Sunday.
    assert!(schedule.includes_zoned(&zoned("2024-06-03T09:30:00-04:00[America/New_York]")));
    assert!(!schedule.includes_zoned(&zoned("2024-06-02T09:30:00-04:00[America/New_York]")));
    assert!(!schedule.includes_zoned(&zoned("2024-06-03T09:31:00-04:00[America/New_York]")));
}

#[test]
fn test_skips_local_times_in_a_gap() {
    let schedule = Schedule::from_str("0 30 2 * * * *").unwrap();
    let start = zoned("2024-03-09T00:00:00-05:00[America/New_York]");
    assert_eq!(
        strings(schedule.after_zoned(&start).take(3)),
        [
            "2024-03-09T02:30:00-05:00[America/New_York]",
            "2024-03-11T02:30:00-04:00[America/New_York]",
            "2024-03-12T02:30:00-04:00[America/New_York]",
        ]
    );
}

#[test]
fn test_repeats_local_times_in_a_fold() {
    let schedule = Schedule::from_str("0 0/30 1 * * * *").unwrap();
    let expected = [
        "2024-11-03T01:00:00-04:00[America/New_York]",
        "2024-11-03T01:30:00-04:00[America/New_York]",
        "2024-11-03T01:00:00-05:00[America/New_York]",
        "2024-11-03T01:30:00-05:00[America/New_York]",
        "2024-11-04T01:00:00-05:00[America/New_York]",
    ];
    let start = zoned("2024-11-03T00:00:00-04:00[America/New_York]");
    assert_eq!(strings(schedule.after_zoned(&start).take(5)), expected);

    let end = zoned("2024-11-04T01:30:00-05:00[America/New_York]");
    let mut backwards = strings(schedule.after_zoned(&end).rev().take(5));
    backwards.reverse();
    assert_eq!(backwards, expected);

    // Starting inside the first fold continues into the second fold.
    let start = zoned("2024-11-03T01:00:00-04:00[America/New_York]");
    assert_eq!(strings(schedule.after_zoned(&start).take(4)), expected[1..]);
}

#[test]
fn test_ends_after_the_last_supported_year() {
    let schedule = Schedule::from_str("0 0 0 1 1 * 2100").unwrap();
    let start = zoned("2100-06-01T00:00:00+00:00[UTC]");
    assert_eq!(schedule.after_zoned(&start).next(), None);
    assert_eq!(
        schedule
            .after_zoned(&start)
            .next_back()
            .unwrap()
            .to_string(),
        "2100-01-01T00:00:00+00:00[UTC]"
    );
}

/// Compares the jiff API with the chrono API over the same instants, including
/// starts just before, inside and after daylight saving transitions.
#[cfg(feature = "chrono")]
mod same_as_chrono {
    use super::*;
    use chrono::TimeZone as _;

    const EXPRESSIONS: &[&str] = &[
        "* * * * * * *",
        "0 0/30 * * * * *",
        "0 0/15 1,2,3 * * * *",
        "0 30 2 * * * *",
        "15 45 0,1,2,3 * * Sun,Sat *",
        "0 30 9,12,15 1,15 May-Aug Mon,Wed,Fri 2018/2",
        "0 0 0 29 2 * *",
        "0 0 0 31 * * *",
        "0 0 0 1-31/10 * ?",
        "@hourly",
        "@daily",
        "@weekly",
        "@monthly",
    ];

    const ZONES: &[&str] = &[
        "UTC",
        "America/New_York",
        "Europe/London",
        "Africa/Ceuta",
        "Australia/Lord_Howe",
        "Pacific/Chatham",
        "America/Sao_Paulo",
        "Asia/Kolkata",
    ];

    const COUNT: usize = 12;

    fn starts(zone: &TimeZone) -> Vec<Timestamp> {
        let mut starts = vec![
            Timestamp::new(1_546_300_800, 0).unwrap(),
            Timestamp::new(1_700_000_000, 250_000_000).unwrap(),
        ];
        let from = Timestamp::new(1_546_300_800, 0).unwrap();
        for transition in zone.following(from).take(4) {
            for step in -4..=4 {
                let start = transition.timestamp().as_second() + step * 30 * 60;
                starts.push(Timestamp::new(start, 0).unwrap());
                starts.push(Timestamp::new(start, 500_000_000).unwrap());
            }
        }
        starts
    }

    fn jiff_side(datetimes: impl Iterator<Item = ZonedDateTime>) -> Vec<(i64, i32, String)> {
        datetimes
            .map(|datetime| {
                (
                    datetime.timestamp().as_second(),
                    datetime.subsec_nanosecond(),
                    datetime.strftime("%Y-%m-%dT%H:%M:%S").to_string(),
                )
            })
            .collect()
    }

    fn chrono_side(
        datetimes: impl Iterator<Item = chrono::DateTime<chrono_tz::Tz>>,
    ) -> Vec<(i64, i32, String)> {
        datetimes
            .map(|datetime| {
                (
                    datetime.timestamp(),
                    datetime.timestamp_subsec_nanos() as i32,
                    datetime.format("%Y-%m-%dT%H:%M:%S").to_string(),
                )
            })
            .collect()
    }

    #[test]
    fn test_same_datetimes_as_chrono() {
        let mut compared = 0;
        for name in ZONES {
            let jiff_zone = TimeZone::get(name).unwrap();
            let chrono_zone: chrono_tz::Tz = name.parse().unwrap();
            for start in starts(&jiff_zone) {
                let jiff_start = start.to_zoned_datetime(jiff_zone.clone());
                let chrono_start = chrono_zone
                    .timestamp_opt(start.as_second(), start.subsec_nanosecond() as u32)
                    .unwrap();
                for expression in EXPRESSIONS {
                    let schedule = Schedule::from_str(expression).unwrap();
                    let context = format!("{expression:?} in {name} from {start}");
                    assert_eq!(
                        jiff_side(schedule.after_zoned(&jiff_start).take(COUNT)),
                        chrono_side(schedule.after(&chrono_start).take(COUNT)),
                        "forward: {context}"
                    );
                    assert_eq!(
                        jiff_side(schedule.after_zoned(&jiff_start).rev().take(COUNT)),
                        chrono_side(schedule.after(&chrono_start).rev().take(COUNT)),
                        "backward: {context}"
                    );
                    assert_eq!(
                        schedule.includes_zoned(&jiff_start),
                        schedule.includes(chrono_start),
                        "includes: {context}"
                    );
                    compared += 1;
                }
            }
        }
        assert!(compared > 3_000, "{compared}");
    }
}

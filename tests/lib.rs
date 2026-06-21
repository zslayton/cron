#[cfg(test)]
mod tests {
    use chrono::*;
    use chrono_tz::Tz;
    use cron::{
        CronScheduleParts, DayOfWeekNumbering, DowDomOperand, NonexistentTimeBehavior, Schedule,
        ScheduleConfig, TimeUnitSpec,
    };
    use std::ops::Bound::{Excluded, Included};
    use std::str::FromStr;

    fn utc(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
            .unwrap()
    }

    fn next_events(schedule: &Schedule, start: DateTime<Utc>, count: usize) -> Vec<DateTime<Utc>> {
        schedule.after(&start).take(count).collect()
    }

    fn prev_events(schedule: &Schedule, start: DateTime<Utc>, count: usize) -> Vec<DateTime<Utc>> {
        let mut events = schedule.after(&start);
        (0..count).map(|_| events.next_back().unwrap()).collect()
    }

    fn assert_next_events(schedule: &Schedule, start: DateTime<Utc>, expected: &[DateTime<Utc>]) {
        assert_eq!(expected, next_events(schedule, start, expected.len()));
    }

    fn assert_prev_events(schedule: &Schedule, start: DateTime<Utc>, expected: &[DateTime<Utc>]) {
        assert_eq!(expected, prev_events(schedule, start, expected.len()));
    }

    #[test]
    fn test_readme() {
        let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
        let schedule = Schedule::from_str(expression).unwrap();
        println!("README: Upcoming fire times for '{}':", expression);
        for datetime in schedule.upcoming(Utc).take(10) {
            println!("README: -> {}", datetime);
        }
    }

    #[test]
    fn test_anything_goes() {
        let expression = "* * * * * * *";
        let schedule = Schedule::from_str(expression).unwrap();
        println!("All stars: Upcoming fire times for '{}':", expression);
        for datetime in schedule.upcoming(Utc).take(10) {
            println!("All stars: -> {}", datetime);
        }
    }

    #[test]
    fn test_parse_with_year() {
        let expression = "1 2 3 4 5 6 2015";
        assert!(Schedule::from_str(expression).is_ok());
    }

    #[test]
    fn test_parse_with_seconds_list() {
        let expression = "1,30,40 2 3 4 5 Mon-Fri";
        assert!(Schedule::from_str(expression).is_ok());
    }

    #[test]
    fn test_parse_with_lists() {
        let expression = "1 2,17,51 1-3,6,9-11 4,29 2,3,7 Tues";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut date = Utc::now();
        println!("Fire times for {}:", expression);
        for _ in 0..20 {
            date = schedule.after(&date).next().expect("No further dates!");
            println!("-> {}", date);
        }
    }

    #[test]
    fn test_upcoming_iterator() {
        let expression = "0 2,17,51 1-3,6,9-11 4,29 2,3,7 Wed";
        let schedule = Schedule::from_str(expression).unwrap();
        println!("Upcoming fire times for '{}':", expression);
        for datetime in schedule.upcoming(Utc).take(12) {
            println!("-> {}", datetime);
        }
    }

    #[test]
    fn test_parse_without_year() {
        let expression = "1 2 3 4 5 6";
        assert!(Schedule::from_str(expression).is_ok());
    }

    #[test]
    fn test_parse_too_many_fields() {
        let expression = "1 2 3 4 5 6 7 8 9 2019";
        assert!(Schedule::from_str(expression).is_err());
    }

    #[test]
    fn test_not_enough_fields() {
        let expression = "1 2 3 2019";
        assert!(Schedule::from_str(expression).is_err());
    }

    #[test]
    fn test_parse_five_part_with_config() {
        let config = ScheduleConfig {
            cron_schedule_parts: CronScheduleParts::Five,
            ..ScheduleConfig::default()
        };
        let schedule = Schedule::from_str_with_config("30 9 * * Mon", config).unwrap();
        let next = schedule
            .after(&Utc.with_ymd_and_hms(2024, 1, 1, 9, 29, 59).unwrap())
            .next()
            .unwrap();
        assert_eq!(0, next.second());
    }

    #[test]
    fn test_default_config_rejects_five_part() {
        assert!(Schedule::from_str_with_config("30 9 * * Mon", ScheduleConfig::default()).is_err());
    }

    #[test]
    fn test_parse_five_or_six_part_modes() {
        let config = ScheduleConfig {
            cron_schedule_parts: CronScheduleParts::FiveOrSix,
            ..ScheduleConfig::default()
        };
        assert!(Schedule::from_str_with_config("0 30 9 * * Mon", config).is_ok());
        assert!(Schedule::from_str_with_config("30 9 * * Mon", config).is_ok());
        assert!(Schedule::from_str_with_config("0 30 9 * * Mon 2024", config).is_err());
    }

    #[test]
    fn test_invalid_day_of_month_month_combinations_are_rejected() {
        let five_part = ScheduleConfig {
            cron_schedule_parts: CronScheduleParts::Five,
            ..ScheduleConfig::default()
        };

        assert!(Schedule::from_str_with_config("0 0 31 2 *", five_part).is_err());
        assert!(Schedule::from_str_with_config("0 0 31 4 *", five_part).is_err());
        assert!(Schedule::from_str_with_config("0 0 31 1,2 *", five_part).is_ok());
        assert!(Schedule::from_str_with_config("0 0 29 2 *", five_part).is_ok());

        assert!(Schedule::from_str("0 0 0 31 2 *").is_err());
        assert!(Schedule::vixie().parse("0 0 0 31 2 mon").is_ok());
    }

    #[test]
    fn test_invalid_leap_day_year_combinations_are_rejected() {
        assert!(Schedule::from_str("0 0 0 29 2 * 2024").is_ok());
        assert!(Schedule::from_str("0 0 0 29 2 * 2024,2025").is_ok());
        assert!(Schedule::from_str("0 0 0 29 2 * 2025").is_err());
        assert!(Schedule::from_str("0 0 0 29 2 * 2025-2026").is_err());
    }

    #[test]
    fn test_configured_year_field_modes_iterate_datetimes() {
        for cron_schedule_parts in [
            CronScheduleParts::Seven,
            CronScheduleParts::SixOrSeven,
            CronScheduleParts::All,
        ] {
            let schedule = Schedule::builder()
                .allowed_cron_schedule_parts(cron_schedule_parts)
                .parse("0 0 0 1 1 * 2020/2")
                .unwrap();
            let start = Utc.with_ymd_and_hms(2019, 12, 31, 23, 59, 59).unwrap();
            let actual = schedule.after(&start).take(3).collect::<Vec<_>>();

            assert_eq!(
                actual,
                vec![
                    Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
                    Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                ]
            );
        }

        assert!(Schedule::builder()
            .allowed_cron_schedule_parts(CronScheduleParts::Six)
            .parse("0 0 0 1 1 * 2020")
            .is_err());
    }

    #[test]
    fn test_configured_year_field_iterates_past_2099() {
        let schedule = Schedule::builder()
            .allowed_cron_schedule_parts(CronScheduleParts::Seven)
            .parse("0 0 0 1 1 * 2100/2")
            .unwrap();
        let start = Utc.with_ymd_and_hms(2099, 12, 31, 23, 59, 59).unwrap();
        let actual = schedule.after(&start).take(3).collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2102, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2104, 1, 1, 0, 0, 0).unwrap(),
            ]
        );
    }

    #[test]
    fn test_configured_year_wraparound_iterates_datetimes() {
        let schedule = Schedule::builder()
            .allowed_cron_schedule_parts(CronScheduleParts::Seven)
            .wraparound_ranges(true)
            .parse("0 0 0 1 1 * 2098-1971")
            .unwrap();
        let start = Utc.with_ymd_and_hms(1969, 12, 31, 23, 59, 59).unwrap();
        let actual = schedule.after(&start).take(3).collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(1971, 1, 1, 0, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2098, 1, 1, 0, 0, 0).unwrap(),
            ]
        );
    }

    #[test]
    fn test_unrestricted_years_continue_past_supported_year_field_max() {
        let schedule = Schedule::from_str("0 0 0 1 1 *").unwrap();
        let start = Utc.with_ymd_and_hms(2099, 1, 2, 0, 0, 0).unwrap();
        let expected = Utc.with_ymd_and_hms(2100, 1, 1, 0, 0, 0).unwrap();

        assert_eq!(Some(expected), schedule.after(&start).next());
        assert!(schedule.includes(expected));
        assert!(schedule.years().includes(2100));
    }

    #[test]
    fn test_unrestricted_years_are_bounded_by_search_interval() {
        let schedule = Schedule::builder()
            .search_interval(TimeDelta::days(300))
            .parse("0 0 0 1 1 *")
            .unwrap();
        let start = Utc.with_ymd_and_hms(2099, 1, 2, 0, 0, 0).unwrap();

        assert_eq!(None, schedule.after(&start).next());
    }

    #[test]
    fn test_parse_vixie_day_of_week_numbering() {
        let config = ScheduleConfig {
            day_of_week_numbering: DayOfWeekNumbering::ZeroIndexed,
            ..ScheduleConfig::default()
        };
        let schedule = Schedule::from_str_with_config("0 0 0 * * 0", config).unwrap();
        let schedule_with_seven = Schedule::from_str_with_config("0 0 0 * * 7", config).unwrap();
        let sunday = Utc.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap();
        let monday = Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        assert!(schedule.includes(sunday));
        assert!(!schedule.includes(monday));
        assert!(schedule_with_seven.includes(sunday));
        assert!(!schedule_with_seven.includes(monday));
        assert!(Schedule::from_str_with_config("0 0 0 * * 8", config).is_err());
    }

    #[test]
    fn test_default_day_of_week_numbering_rejects_zero() {
        assert!(Schedule::from_str_with_config("0 0 0 * * 0", ScheduleConfig::default()).is_err());
    }

    #[test]
    fn test_default_day_of_week_numbering_treats_seven_as_saturday() {
        let schedule =
            Schedule::from_str_with_config("0 0 0 * * 7", ScheduleConfig::default()).unwrap();
        let saturday = Utc.with_ymd_and_hms(2024, 1, 6, 0, 0, 0).unwrap();
        let sunday = Utc.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap();
        assert!(schedule.includes(saturday));
        assert!(!schedule.includes(sunday));
    }

    #[test]
    fn test_wraparound_ranges_disabled_by_default() {
        assert!(Schedule::builder().parse("0 0 0 * Nov-Mar *").is_err());
        assert!(Schedule::builder().parse("0 0 22-2 * * *").is_err());
        assert!(Schedule::builder().parse("0 0 0 * * Fri-Mon").is_err());
    }

    #[test]
    fn test_wraparound_ranges_all_fields() {
        let schedule = Schedule::builder()
            .wraparound_ranges(true)
            .parse("55-2 55-2 22-2 28-3 Nov-Mar 6-2 2099-1971")
            .unwrap();

        assert!(schedule.seconds().includes(55));
        assert!(schedule.seconds().includes(0));
        assert!(schedule.seconds().includes(2));
        assert!(!schedule.seconds().includes(3));
        assert!(schedule.minutes().includes(55));
        assert!(schedule.minutes().includes(0));
        assert!(schedule.minutes().includes(2));
        assert!(!schedule.minutes().includes(3));
        assert!(schedule.hours().includes(22));
        assert!(schedule.hours().includes(0));
        assert!(schedule.hours().includes(2));
        assert!(!schedule.hours().includes(3));
        assert!(schedule.days_of_month().includes(28));
        assert!(schedule.days_of_month().includes(31));
        assert!(schedule.days_of_month().includes(1));
        assert!(schedule.days_of_month().includes(3));
        assert!(!schedule.days_of_month().includes(4));
        assert!(schedule.months().includes(11));
        assert!(schedule.months().includes(12));
        assert!(schedule.months().includes(1));
        assert!(schedule.months().includes(3));
        assert!(!schedule.months().includes(4));
        assert!(schedule.days_of_week().includes(6));
        assert!(schedule.days_of_week().includes(7));
        assert!(schedule.days_of_week().includes(1));
        assert!(schedule.days_of_week().includes(2));
        assert!(!schedule.days_of_week().includes(3));
        assert!(schedule.years().includes(2099));
        assert!(schedule.years().includes(2100));
        assert!(schedule.years().includes(1970));
        assert!(schedule.years().includes(1971));
        assert!(!schedule.years().includes(1972));
    }

    #[test]
    fn test_wraparound_ranges_equal_endpoints_are_full_cycle() {
        let sunday_to_sunday = Schedule::vixie().parse("0 0 0 * * Sun-Sun").unwrap();
        let monday_to_monday = Schedule::vixie().parse("0 0 0 * * Mon-Mon").unwrap();
        let january_to_january = Schedule::vixie().parse("0 0 0 * Jan-Jan *").unwrap();

        assert_eq!(7, sunday_to_sunday.days_of_week().count());
        assert_eq!(7, monday_to_monday.days_of_week().count());
        assert_eq!(12, january_to_january.months().count());
    }

    #[test]
    fn test_wraparound_ranges_apply_croniter_step_boundary_rules() {
        let schedule = Schedule::builder()
            .wraparound_ranges(true)
            .parse("0 0 22-2/2 * Nov-Mar/2 *")
            .unwrap();

        assert!(schedule.hours().includes(22));
        assert!(!schedule.hours().includes(23));
        assert!(schedule.hours().includes(0));
        assert!(!schedule.hours().includes(1));
        assert!(schedule.hours().includes(2));
        assert!(schedule.months().includes(11));
        assert!(!schedule.months().includes(12));
        assert!(schedule.months().includes(1));
        assert!(!schedule.months().includes(2));
        assert!(schedule.months().includes(3));
    }

    #[test]
    fn test_vixie_wraparound_day_of_week_step_skips_boundary_values() {
        let schedule = Schedule::vixie().parse("0 0 0 * * Thu-Tue/2").unwrap();

        let monday = Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let tuesday = Utc.with_ymd_and_hms(2024, 1, 9, 0, 0, 0).unwrap();
        let thursday = Utc.with_ymd_and_hms(2024, 1, 11, 0, 0, 0).unwrap();
        let friday = Utc.with_ymd_and_hms(2024, 1, 12, 0, 0, 0).unwrap();
        let saturday = Utc.with_ymd_and_hms(2024, 1, 13, 0, 0, 0).unwrap();
        let sunday = Utc.with_ymd_and_hms(2024, 1, 14, 0, 0, 0).unwrap();

        assert!(schedule.includes(thursday));
        assert!(schedule.includes(saturday));
        assert!(schedule.includes(tuesday));
        assert!(!schedule.includes(friday));
        assert!(!schedule.includes(sunday));
        assert!(!schedule.includes(monday));
    }

    #[test]
    fn test_wraparound_month_step_skips_boundary_values() {
        let schedule = Schedule::vixie().parse("0 0 0 * Apr-Mar/2 *").unwrap();

        assert!(schedule.months().includes(4));
        assert!(schedule.months().includes(6));
        assert!(schedule.months().includes(8));
        assert!(schedule.months().includes(10));
        assert!(schedule.months().includes(12));
        assert!(schedule.months().includes(3));
        assert!(!schedule.months().includes(1));
        assert!(!schedule.months().includes(2));
    }

    #[test]
    fn test_vixie_preset_accepts_full_quirks() {
        let sunday_zero = Schedule::vixie().parse("0 0 0 * * 0").unwrap();
        let sunday_seven = Schedule::vixie().parse("0 0 0 * * 7").unwrap();
        let sunday_alias_period = Schedule::vixie().parse("0 0 0 * * 7/2").unwrap();
        let seven_to_monday = Schedule::vixie().parse("0 0 0 * * 7-mon").unwrap();
        let friday_to_monday = Schedule::vixie().parse("0 0 0 * * Fri-Mon").unwrap();
        let november_to_march = Schedule::vixie().parse("0 0 0 * Nov-Mar *").unwrap();

        let friday = Utc.with_ymd_and_hms(2024, 1, 5, 0, 0, 0).unwrap();
        let saturday = Utc.with_ymd_and_hms(2024, 1, 6, 0, 0, 0).unwrap();
        let sunday = Utc.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap();
        let monday = Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let tuesday = Utc.with_ymd_and_hms(2024, 1, 9, 0, 0, 0).unwrap();
        let thursday = Utc.with_ymd_and_hms(2024, 1, 11, 0, 0, 0).unwrap();

        assert!(sunday_zero.includes(sunday));
        assert!(sunday_seven.includes(sunday));
        assert!(sunday_alias_period.includes(sunday));
        assert!(!sunday_alias_period.includes(monday));
        assert!(sunday_alias_period.includes(tuesday));
        assert!(sunday_alias_period.includes(thursday));
        assert!(seven_to_monday.includes(sunday));
        assert!(seven_to_monday.includes(monday));
        assert!(!seven_to_monday.includes(tuesday));
        assert!(friday_to_monday.includes(friday));
        assert!(friday_to_monday.includes(saturday));
        assert!(friday_to_monday.includes(sunday));
        assert!(friday_to_monday.includes(monday));
        assert!(!friday_to_monday.includes(tuesday));
        assert!(november_to_march.months().includes(11));
        assert!(november_to_march.months().includes(12));
        assert!(november_to_march.months().includes(1));
        assert!(november_to_march.months().includes(3));
        assert!(!november_to_march.months().includes(4));
    }

    #[test]
    fn test_special_specifiers_are_separately_configurable() {
        assert!(Schedule::builder().parse("0 0 0 l * *").is_err());
        assert!(Schedule::builder().parse("0 0 0 15w * *").is_err());
        assert!(Schedule::builder().parse("0 0 0 * * mon#2").is_err());
        assert!(Schedule::builder().parse("R 0 0 * * *").is_err());

        assert!(Schedule::builder()
            .last_specifiers(true)
            .parse("0 0 0 l * *")
            .is_ok());
        assert!(Schedule::builder()
            .last_specifiers(true)
            .parse("0 0 0 15w * *")
            .is_err());
        assert!(Schedule::builder()
            .nearest_weekday(true)
            .parse("0 0 0 15w * *")
            .is_ok());
        assert!(Schedule::builder()
            .nth_weekday_of_month(true)
            .parse("0 0 0 * * mon#2")
            .is_ok());
        assert!(Schedule::builder()
            .random_fields(true)
            .parse("R 0 0 * * *")
            .is_ok());
    }

    #[test]
    fn test_last_day_of_month_specifier() {
        let schedule = Schedule::builder()
            .last_specifiers(true)
            .parse("0 0 0 l * *")
            .unwrap();

        assert_next_events(
            &schedule,
            utc(2025, 2, 15, 0, 0, 0),
            &[
                utc(2025, 2, 28, 0, 0, 0),
                utc(2025, 3, 31, 0, 0, 0),
                utc(2025, 4, 30, 0, 0, 0),
            ],
        );
        assert_prev_events(
            &schedule,
            utc(2025, 3, 15, 0, 0, 0),
            &[
                utc(2025, 2, 28, 0, 0, 0),
                utc(2025, 1, 31, 0, 0, 0),
                utc(2024, 12, 31, 0, 0, 0),
            ],
        );
    }

    #[test]
    fn test_nearest_weekday_specifier() {
        let first_weekday = Schedule::builder()
            .nearest_weekday(true)
            .parse("0 0 0 1w * *")
            .unwrap();
        assert_next_events(
            &first_weekday,
            utc(2025, 3, 2, 0, 0, 0),
            &[
                utc(2025, 3, 3, 0, 0, 0),
                utc(2025, 4, 1, 0, 0, 0),
                utc(2025, 5, 1, 0, 0, 0),
            ],
        );
        assert_prev_events(
            &first_weekday,
            utc(2025, 3, 2, 0, 0, 0),
            &[
                utc(2025, 2, 3, 0, 0, 0),
                utc(2025, 1, 1, 0, 0, 0),
                utc(2024, 12, 2, 0, 0, 0),
            ],
        );

        let clamped_weekday = Schedule::builder()
            .nearest_weekday(true)
            .parse("0 0 0 w31 2 *")
            .unwrap();
        assert_next_events(
            &clamped_weekday,
            utc(2025, 1, 1, 0, 0, 0),
            &[utc(2025, 2, 28, 0, 0, 0), utc(2026, 2, 27, 0, 0, 0)],
        );
        assert_prev_events(
            &clamped_weekday,
            utc(2026, 3, 1, 0, 0, 0),
            &[utc(2026, 2, 27, 0, 0, 0), utc(2025, 2, 28, 0, 0, 0)],
        );
    }

    #[test]
    fn test_nth_and_last_weekday_specifiers() {
        let third_monday = Schedule::builder()
            .nth_weekday_of_month(true)
            .parse("0 0 0 * 6 mon#3")
            .unwrap();
        assert_next_events(
            &third_monday,
            utc(2025, 6, 12, 0, 0, 0),
            &[utc(2025, 6, 16, 0, 0, 0), utc(2026, 6, 15, 0, 0, 0)],
        );
        assert_prev_events(
            &third_monday,
            utc(2025, 6, 12, 0, 0, 0),
            &[utc(2024, 6, 17, 0, 0, 0), utc(2023, 6, 19, 0, 0, 0)],
        );

        let last_friday = Schedule::builder()
            .last_specifiers(true)
            .parse("0 0 0 * * Lfri")
            .unwrap();
        assert_next_events(
            &last_friday,
            utc(2025, 6, 12, 0, 0, 0),
            &[
                utc(2025, 6, 27, 0, 0, 0),
                utc(2025, 7, 25, 0, 0, 0),
                utc(2025, 8, 29, 0, 0, 0),
            ],
        );
        assert_prev_events(
            &last_friday,
            utc(2025, 6, 12, 0, 0, 0),
            &[
                utc(2025, 5, 30, 0, 0, 0),
                utc(2025, 4, 25, 0, 0, 0),
                utc(2025, 3, 28, 0, 0, 0),
            ],
        );
    }

    #[test]
    fn test_random_field_specifier_expands_at_parse_time() {
        let schedule = Schedule::builder()
            .random_fields(true)
            .parse("R(10-12) R(20-22) R(3-5) R(1-3) R(4-6) * R(2025-2027)")
            .unwrap();

        let next = schedule.after(&utc(2024, 1, 1, 0, 0, 0)).next().unwrap();
        let prev = schedule
            .after(&utc(2028, 1, 1, 0, 0, 0))
            .next_back()
            .unwrap();
        assert_eq!(next, prev);
        assert!((10..=12).contains(&next.second()));
        assert!((20..=22).contains(&next.minute()));
        assert!((3..=5).contains(&next.hour()));
        assert!((1..=3).contains(&next.day()));
        assert!((4..=6).contains(&next.month()));
        assert!((2025..=2027).contains(&next.year()));

        let day_of_week = Schedule::builder()
            .random_fields(true)
            .parse("0 0 0 * 4 R(2-4) 2025")
            .unwrap();
        let april_forward = next_events(&day_of_week, utc(2025, 3, 31, 23, 59, 59), 5)
            .into_iter()
            .take_while(|event| event.month() == 4)
            .collect::<Vec<_>>();
        let mut april_backward = Vec::new();
        let mut events = day_of_week.after(&utc(2025, 5, 1, 0, 0, 0));
        while let Some(event) = events.next_back() {
            if event.month() != 4 {
                break;
            }
            april_backward.push(event);
        }
        april_backward.reverse();
        assert!(!april_forward.is_empty());
        assert_eq!(april_forward, april_backward);
        assert!(april_forward
            .iter()
            .all(|event| (2..=4).contains(&event.weekday().number_from_sunday())));

        let stepped = Schedule::builder()
            .random_fields(true)
            .parse("R/15 0 0 * * *")
            .unwrap();
        let forward = next_events(&stepped, utc(2024, 12, 31, 23, 59, 59), 4);
        let mut backward = prev_events(&stepped, utc(2025, 1, 1, 0, 1, 0), 4);
        backward.reverse();
        assert_eq!(forward, backward);
        assert!(forward
            .windows(2)
            .all(|events| events[1] - events[0] == TimeDelta::seconds(15)));
    }

    #[test]
    fn test_builder_interface_custom_parts() {
        let schedule = Schedule::builder()
            .allowed_cron_schedule_parts(CronScheduleParts::FiveOrSix)
            .parse("30 9 * * Mon")
            .unwrap();
        let next = schedule
            .after(&Utc.with_ymd_and_hms(2024, 1, 1, 9, 29, 59).unwrap())
            .next()
            .unwrap();
        assert_eq!(0, next.second());
    }

    #[test]
    fn test_builder_interface_default() {
        let schedule = Schedule::default().parse("0 30 9 * * Mon").unwrap();
        let next = schedule
            .after(&Utc.with_ymd_and_hms(2024, 1, 1, 9, 29, 59).unwrap())
            .next()
            .unwrap();
        assert_eq!(30, next.minute());
    }

    #[test]
    fn test_builder_interface_vixie() {
        let schedule = Schedule::vixie().parse("0 0 0 * * 0").unwrap();
        let sunday = Utc.with_ymd_and_hms(2024, 1, 7, 0, 0, 0).unwrap();
        assert!(schedule.includes(sunday));
    }

    #[test]
    fn test_days_matching_default_and() {
        let schedule = Schedule::default().parse("0 0 0 1 * Mon").unwrap();
        let mon_1st = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let mon_8th = Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let thu_1st = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        assert!(schedule.includes(mon_1st));
        assert!(!schedule.includes(mon_8th));
        assert!(!schedule.includes(thu_1st));
    }

    #[test]
    fn test_days_matching_or() {
        let schedule = Schedule::builder()
            .dow_dom_operand(DowDomOperand::Or)
            .parse("0 0 0 1 * Mon")
            .unwrap();
        let mon_1st = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let mon_8th = Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let thu_1st = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        let tue_2nd = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        assert!(schedule.includes(mon_1st));
        assert!(schedule.includes(mon_8th));
        assert!(schedule.includes(thu_1st));
        assert!(!schedule.includes(tue_2nd));
    }

    #[test]
    fn test_vixie_includes_or_days_matching() {
        let schedule = Schedule::vixie().parse("0 0 0 1 * 1").unwrap();
        let mon_1st = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();
        let mon_8th = Utc.with_ymd_and_hms(2024, 1, 8, 0, 0, 0).unwrap();
        let thu_1st = Utc.with_ymd_and_hms(2024, 2, 1, 0, 0, 0).unwrap();
        assert!(schedule.includes(mon_1st));
        assert!(schedule.includes(mon_8th));
        assert!(schedule.includes(thu_1st));
    }

    #[test]
    fn test_search_interval_limits_next() {
        let schedule = Schedule::builder()
            .search_interval(TimeDelta::days(300))
            .parse("0 0 0 1 1 *")
            .unwrap();
        let start = Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap();
        assert!(schedule.after(&start).next().is_none());
    }

    #[test]
    fn test_search_interval_limits_prev() {
        let schedule = Schedule::builder()
            .search_interval(TimeDelta::days(300))
            .parse("0 0 0 1 1 *")
            .unwrap();
        let start = Utc.with_ymd_and_hms(2024, 12, 31, 0, 0, 0).unwrap();
        assert!(schedule.after(&start).next_back().is_none());
    }

    #[test]
    fn test_next_utc() {
        let expression = "1 2 3 4 10 Fri";
        let schedule = Schedule::from_str(expression).unwrap();
        let next = schedule
            .upcoming(Utc)
            .next()
            .expect("There was no upcoming fire time.");
        println!("Next fire time: {}", next.to_rfc3339());
    }

    #[test]
    fn test_prev_utc() {
        let expression = "1 2 3 4 10 Fri";
        let schedule = Schedule::from_str(expression).unwrap();
        let prev = schedule
            .upcoming(Utc)
            .next_back()
            .expect("There was no previous upcoming fire time.");
        println!("Previous fire time: {}", prev.to_rfc3339());
    }

    #[test]
    fn test_yearly() {
        let expression = "@yearly";
        let schedule = Schedule::from_str(expression).expect("Failed to parse @yearly.");
        let starting_date = Utc.with_ymd_and_hms(2017, 6, 15, 14, 29, 36).unwrap();
        let mut events = schedule.after(&starting_date);
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2019, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
    }

    #[test]
    fn test_monthly() {
        let expression = "@monthly";
        let schedule = Schedule::from_str(expression).expect("Failed to parse @monthly.");
        let starting_date = Utc.with_ymd_and_hms(2017, 10, 15, 14, 29, 36).unwrap();
        let mut events = schedule.after(&starting_date);
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 11, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 12, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
    }

    #[test]
    fn test_weekly() {
        let expression = "@weekly";
        let schedule = Schedule::from_str(expression).expect("Failed to parse @weekly.");
        let starting_date = Utc.with_ymd_and_hms(2016, 12, 23, 14, 29, 36).unwrap();
        let mut events = schedule.after(&starting_date);
        assert_eq!(
            Utc.with_ymd_and_hms(2016, 12, 25, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 1, 8, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
    }

    #[test]
    fn test_daily() {
        let expression = "@daily";
        let schedule = Schedule::from_str(expression).expect("Failed to parse @daily.");
        let starting_date = Utc.with_ymd_and_hms(2016, 12, 29, 14, 29, 36).unwrap();
        let mut events = schedule.after(&starting_date);
        assert_eq!(
            Utc.with_ymd_and_hms(2016, 12, 30, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2016, 12, 31, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
    }

    #[test]
    fn test_hourly() {
        let expression = "@hourly";
        let schedule = Schedule::from_str(expression).expect("Failed to parse @hourly.");
        let starting_date = Utc.with_ymd_and_hms(2017, 2, 25, 22, 29, 36).unwrap();
        let mut events = schedule.after(&starting_date);
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 2, 25, 23, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 2, 26, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2017, 2, 26, 1, 0, 0).unwrap(),
            events.next().unwrap()
        );
    }

    #[test]
    fn test_step_schedule() {
        let expression = "0/20 0/5 0 1 1 * *";
        let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
        let starting_date = Utc.with_ymd_and_hms(2017, 6, 15, 14, 29, 36).unwrap();
        let mut events = schedule.after(&starting_date);

        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 0, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 0, 20).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 0, 40).unwrap(),
            events.next().unwrap()
        );

        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 5, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 5, 20).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 5, 40).unwrap(),
            events.next().unwrap()
        );

        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 10, 0).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 10, 20).unwrap(),
            events.next().unwrap()
        );
        assert_eq!(
            Utc.with_ymd_and_hms(2018, 1, 1, 0, 10, 40).unwrap(),
            events.next().unwrap()
        );
    }

    #[test]
    fn test_invalid_step() {
        let expression = "0/0 * * * *";
        assert!(Schedule::from_str(expression).is_err());
    }

    #[test]
    fn test_time_unit_spec_years() {
        let expression = "* * * * * * 2015-2044";
        let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");

        // Membership
        assert!(schedule.years().includes(2031));
        assert!(!schedule.years().includes(1969));

        // Number of years specified
        assert_eq!(30, schedule.years().count());

        // Iterator
        let mut years_iter = schedule.years().iter();
        assert_eq!(Some(2015), years_iter.next());
        assert_eq!(Some(2016), years_iter.next());
        // ...

        // Range Iterator
        let mut five_year_plan = schedule.years().range((Included(2017), Excluded(2017 + 5)));
        assert_eq!(Some(2017), five_year_plan.next());
        assert_eq!(Some(2018), five_year_plan.next());
        assert_eq!(Some(2019), five_year_plan.next());
        assert_eq!(Some(2020), five_year_plan.next());
        assert_eq!(Some(2021), five_year_plan.next());
        assert_eq!(None, five_year_plan.next());
    }

    #[test]
    fn test_time_unit_spec_months() {
        let expression = "* * * * 5-8 * *";
        let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");

        // Membership
        assert!(!schedule.months().includes(4));
        assert!(schedule.months().includes(6));

        // Iterator
        let mut summer = schedule.months().iter();
        assert_eq!(Some(5), summer.next());
        assert_eq!(Some(6), summer.next());
        assert_eq!(Some(7), summer.next());
        assert_eq!(Some(8), summer.next());
        assert_eq!(None, summer.next());

        // Number of months specified
        assert_eq!(4, schedule.months().count());

        // Range Iterator
        let mut first_half_of_summer = schedule.months().range((Included(1), Included(6)));
        assert_eq!(Some(5), first_half_of_summer.next());
        assert_eq!(Some(6), first_half_of_summer.next());
        assert_eq!(None, first_half_of_summer.next());
    }

    #[test]
    fn test_time_unit_spec_days_of_month() {
        let expression = "* * * 1,15 * * *";
        let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
        // Membership
        assert!(schedule.days_of_month().includes(1));
        assert!(!schedule.days_of_month().includes(7));

        // Iterator
        let mut paydays = schedule.days_of_month().iter();
        assert_eq!(Some(1), paydays.next());
        assert_eq!(Some(15), paydays.next());
        assert_eq!(None, paydays.next());

        // Number of years specified
        assert_eq!(2, schedule.days_of_month().count());

        // Range Iterator
        let mut mid_month_paydays = schedule.days_of_month().range((Included(5), Included(25)));
        assert_eq!(Some(15), mid_month_paydays.next());
        assert_eq!(None, mid_month_paydays.next());
    }

    #[test]
    fn test_first_ordinals_not_in_set_1() {
        let schedule = "0 0/10 * * * * *".parse::<Schedule>().unwrap();
        let start_time_1 = NaiveDate::from_ymd_opt(2017, 10, 24)
            .unwrap()
            .and_hms_opt(0, 0, 59)
            .unwrap();
        let start_time_1 = Utc.from_utc_datetime(&start_time_1);
        let next_time_1 = schedule.after(&start_time_1).next().unwrap();

        let start_time_2 = NaiveDate::from_ymd_opt(2017, 10, 24)
            .unwrap()
            .and_hms_opt(0, 1, 0)
            .unwrap();
        let start_time_2 = Utc.from_utc_datetime(&start_time_2);
        let next_time_2 = schedule.after(&start_time_2).next().unwrap();
        assert_eq!(next_time_1, next_time_2);
    }

    #[test]
    fn test_first_ordinals_not_in_set_2() {
        let schedule_1 = "00 00 23 * * * *".parse::<Schedule>().unwrap();
        let start_time = NaiveDate::from_ymd_opt(2018, 11, 15)
            .unwrap()
            .and_hms_opt(22, 30, 00)
            .unwrap();
        let start_time = Utc.from_utc_datetime(&start_time);
        let next_time_1 = schedule_1.after(&start_time).next().unwrap();

        let schedule_2 = "00 00 * * * * *".parse::<Schedule>().unwrap();
        let next_time_2 = schedule_2.after(&start_time).next().unwrap();
        assert_eq!(next_time_1, next_time_2);
    }

    #[test]
    fn test_period_values_any_dom() {
        let schedule = Schedule::from_str("0 0 0 ? * *").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 9, 17, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        assert_eq!(
            schedule_tz.with_ymd_and_hms(2020, 9, 18, 0, 0, 0).unwrap(),
            schedule_iter.next().unwrap()
        );
    }

    #[test]
    fn test_period_values_any_dow() {
        let schedule = Schedule::from_str("0 0 0 * * ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 9, 17, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        assert_eq!(
            schedule_tz.with_ymd_and_hms(2020, 9, 18, 0, 0, 0).unwrap(),
            schedule_iter.next().unwrap()
        );
    }

    #[test]
    fn test_period_values_all_seconds() {
        let schedule = Schedule::from_str("*/17 * * * * ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 17).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 34).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 51).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 1, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 1, 17).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 1, 34).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_range() {
        let schedule = Schedule::from_str("0 0 0 1 1-4/2 ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 3, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2021, 3, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_range_hours() {
        let schedule = Schedule::from_str("0 0 10-12/2 * * ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 10, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 12, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 2, 10, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 2, 12, 0, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_range_days() {
        let schedule = Schedule::from_str("0 0 0 1-31/10 * ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 1, 11, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 21, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 31, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 2, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 2, 11, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 2, 21, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 3, 1, 0, 0, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_range_months() {
        let schedule = Schedule::from_str("0 0 0 1 January-June/1 *").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 2, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 3, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 4, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 5, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 6, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_range_years() {
        let schedule = Schedule::from_str("0 0 0 1 1 ? 2020-2040/10").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2040, 1, 1, 0, 0, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_point() {
        let schedule = Schedule::from_str("0 */21 * * * ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 21, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 42, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 1, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 1, 21, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 1, 42, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 2, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 2, 21, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2020, 1, 1, 2, 42, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_period_values_named_range() {
        let schedule = Schedule::from_str("0 0 0 1 January-April/2 ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        let mut schedule_iter = schedule.after(&dt);
        let expected_values = [
            schedule_tz.with_ymd_and_hms(2020, 3, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2021, 1, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2021, 3, 1, 0, 0, 0).unwrap(),
            schedule_tz.with_ymd_and_hms(2022, 1, 1, 0, 0, 0).unwrap(),
        ];
        for expected_value in expected_values.iter() {
            assert_eq!(*expected_value, schedule_iter.next().unwrap());
        }
    }

    #[test]
    fn test_is_all() {
        let schedule = Schedule::from_str("0-59 * 0-23 ?/2 1,2-4 ? *").unwrap();
        assert!(schedule.years().is_all());
        assert!(!schedule.days_of_month().is_all());
        assert!(schedule.days_of_week().is_all());
        assert!(!schedule.months().is_all());
        assert!(schedule.hours().is_all());
        assert!(schedule.minutes().is_all());
        assert!(schedule.seconds().is_all());
    }

    #[test]
    fn test_includes() {
        let schedule = Schedule::from_str("0 0 0 2-31/10 * ?").unwrap();
        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let included = schedule_tz.with_ymd_and_hms(2020, 1, 12, 0, 0, 0).unwrap();
        let not_included = schedule_tz.with_ymd_and_hms(2020, 1, 11, 0, 0, 0).unwrap();
        assert!(schedule.includes(included));
        assert!(!schedule.includes(not_included));
    }

    struct CronIterationTestCase {
        name: &'static str,
        timezone: Tz,
        cron: &'static str,
        expected: &'static [&'static str],
    }

    fn parse_expected_in_tz(timezone: Tz, expected: &str) -> DateTime<Tz> {
        DateTime::parse_from_rfc3339(expected)
            .unwrap()
            .with_timezone(&timezone)
    }

    fn dst_iteration_cases() -> Vec<CronIterationTestCase> {
        vec![
            CronIterationTestCase {
                name: "hourly_fall_back_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0 * * * * *",
                expected: &[
                    "2022-11-06T01:00:00-07:00",
                    "2022-11-06T01:00:00-08:00",
                    "2022-11-06T02:00:00-08:00",
                    "2022-11-06T03:00:00-08:00",
                    "2022-11-06T04:00:00-08:00",
                ],
            },
            CronIterationTestCase {
                name: "hourly_spring_forward_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0 * * * * *",
                expected: &[
                    "2022-03-13T01:00:00-08:00",
                    "2022-03-13T03:00:00-07:00",
                    "2022-03-13T04:00:00-07:00",
                    "2022-03-13T05:00:00-07:00",
                ],
            },
            CronIterationTestCase {
                name: "subhourly_fall_back_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0/30 * * * * *",
                expected: &[
                    "2022-11-06T01:00:00-07:00",
                    "2022-11-06T01:30:00-07:00",
                    "2022-11-06T01:00:00-08:00",
                    "2022-11-06T01:30:00-08:00",
                    "2022-11-06T02:00:00-08:00",
                    "2022-11-06T02:30:00-08:00",
                ],
            },
            CronIterationTestCase {
                name: "subhourly_spring_forward_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0/30 * * * * *",
                expected: &[
                    "2022-03-13T01:30:00-08:00",
                    "2022-03-13T03:00:00-07:00",
                    "2022-03-13T03:30:00-07:00",
                    "2022-03-13T04:00:00-07:00",
                ],
            },
            CronIterationTestCase {
                name: "daily_across_fall_back_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0 2 * * * *",
                expected: &["2022-11-06T02:00:00-08:00", "2022-11-07T02:00:00-08:00"],
            },
            CronIterationTestCase {
                name: "daily_across_spring_forward_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0 2 * * * *",
                expected: &["2022-03-14T02:00:00-07:00", "2022-03-15T02:00:00-07:00"],
            },
            CronIterationTestCase {
                name: "monthly_across_spring_forward_los_angeles",
                timezone: "America/Los_Angeles".parse().unwrap(),
                cron: "0 0 2 13 * * *",
                expected: &["2022-04-13T02:00:00-07:00", "2022-05-13T02:00:00-07:00"],
            },
            CronIterationTestCase {
                name: "every_15_minutes_repeats_full_hour_during_fall_back",
                timezone: "Europe/Berlin".parse().unwrap(),
                cron: "0 0/15 * * * * *",
                expected: &[
                    "2022-10-30T02:00:00+02:00",
                    "2022-10-30T02:15:00+02:00",
                    "2022-10-30T02:30:00+02:00",
                    "2022-10-30T02:45:00+02:00",
                    "2022-10-30T02:00:00+01:00",
                    "2022-10-30T02:15:00+01:00",
                    "2022-10-30T02:30:00+01:00",
                    "2022-10-30T02:45:00+01:00",
                    "2022-10-30T03:00:00+01:00",
                ],
            },
        ]
    }

    #[test]
    fn test_dst_iteration_cases_forward() {
        for case in dst_iteration_cases() {
            let schedule = Schedule::from_str(case.cron).unwrap();
            let start = parse_expected_in_tz(case.timezone, case.expected[0]);

            let mut actual = vec![start.to_rfc3339()];
            actual.extend(
                schedule
                    .after(&start)
                    .take(case.expected.len().saturating_sub(1))
                    .map(|dt| dt.to_rfc3339()),
            );

            let expected = case
                .expected
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>();
            assert_eq!(actual, expected, "forward case {}", case.name);
        }
    }

    #[test]
    fn test_dst_iteration_cases_backward() {
        for case in dst_iteration_cases() {
            let schedule = Schedule::from_str(case.cron).unwrap();
            let last = parse_expected_in_tz(case.timezone, case.expected[case.expected.len() - 1]);

            let mut actual = vec![last.to_rfc3339()];
            actual.extend(
                schedule
                    .after(&last)
                    .rev()
                    .take(case.expected.len().saturating_sub(1))
                    .map(|dt| dt.to_rfc3339()),
            );

            let expected_reversed = case
                .expected
                .iter()
                .rev()
                .map(|x| x.to_string())
                .collect::<Vec<_>>();
            assert_eq!(actual, expected_reversed, "backward case {}", case.name);
        }
    }

    #[test]
    fn test_nonexistent_time_next_existent_forward() {
        let timezone: Tz = "America/Los_Angeles".parse().unwrap();
        let schedule = Schedule::builder()
            .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
            .parse("0 0 2 * * * *")
            .unwrap();
        let start = parse_expected_in_tz(timezone, "2022-03-13T01:59:59-08:00");

        let actual = schedule
            .after(&start)
            .take(3)
            .map(|dt| dt.to_rfc3339())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                "2022-03-13T03:00:00-07:00",
                "2022-03-14T02:00:00-07:00",
                "2022-03-15T02:00:00-07:00",
            ]
        );
    }

    #[test]
    fn test_daily_nonexistent_time_next_existent_forward() {
        let timezone: Tz = "America/Los_Angeles".parse().unwrap();
        let schedule = Schedule::builder()
            .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
            .parse("0 30 2 * * * *")
            .unwrap();
        let start = parse_expected_in_tz(timezone, "2022-03-13T01:59:59-08:00");

        let actual = schedule
            .after(&start)
            .take(2)
            .map(|dt| dt.to_rfc3339())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec!["2022-03-13T03:00:00-07:00", "2022-03-14T02:30:00-07:00",]
        );
    }

    #[test]
    fn test_hourly_nonexistent_time_skips_to_next_existent_match() {
        let timezone: Tz = "America/Los_Angeles".parse().unwrap();
        let schedule = Schedule::builder()
            .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
            .parse("0 30 * * * * *")
            .unwrap();
        let start = parse_expected_in_tz(timezone, "2022-03-13T01:59:59-08:00");

        let actual = schedule
            .after(&start)
            .take(2)
            .map(|dt| dt.to_rfc3339())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec!["2022-03-13T03:30:00-07:00", "2022-03-13T04:30:00-07:00",]
        );
    }

    #[test]
    fn test_subhourly_nonexistent_time_skips_to_next_existent_match() {
        let timezone: Tz = "America/Los_Angeles".parse().unwrap();
        let schedule = Schedule::builder()
            .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
            .parse("0 */15 2 * * * *")
            .unwrap();
        let start = parse_expected_in_tz(timezone, "2022-03-13T01:59:59-08:00");

        let actual = schedule
            .after(&start)
            .take(2)
            .map(|dt| dt.to_rfc3339())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec!["2022-03-14T02:00:00-07:00", "2022-03-14T02:15:00-07:00",]
        );
    }

    #[test]
    fn test_nonexistent_time_next_existent_backward() {
        let timezone: Tz = "America/Los_Angeles".parse().unwrap();
        let schedule = Schedule::builder()
            .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
            .parse("0 0 2 * * * *")
            .unwrap();
        let start = parse_expected_in_tz(timezone, "2022-03-14T02:00:00-07:00");

        let actual = schedule
            .after(&start)
            .rev()
            .take(3)
            .map(|dt| dt.to_rfc3339())
            .collect::<Vec<_>>();

        assert_eq!(
            actual,
            vec![
                "2022-03-13T03:00:00-07:00",
                "2022-03-12T02:00:00-08:00",
                "2022-03-11T02:00:00-08:00",
            ]
        );
    }
}

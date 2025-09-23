#![cfg(all(test, feature = "vixie"))]
mod tests {
    use chrono::*;
    use chrono_tz::Tz;
    use cron::{Schedule, TimeUnitSpec};
    use std::ops::Bound::{Excluded, Included};
    use std::str::FromStr;

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
        let expected_values = vec![
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
        let expected_values = vec![
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
        let expected_values = vec![
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
        let expected_values = vec![
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

    #[test]
    fn test_next_and_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();

        let mut next = schedule.after(&Utc::now());
        let first = next.next();
        assert!(first.is_some());

        let mut next2 = schedule.after(&first.unwrap());
        let second = next2.next();
        assert!(second.is_some());

        let mut prev = schedule.after(&second.unwrap());
        let first_again = prev.next_back();
        assert!(first_again.is_some());
        assert_eq!(first, first_again);

        let mut prev2 = schedule.after(&(second.unwrap() + Duration::nanoseconds(100)));
        let second_again = prev2.next_back();
        assert!(second_again.is_some());
        assert_eq!(second, second_again);
    }

    #[test]
    fn test_next_after_past_date_next_year() {
        // Schedule after 2021-10-27
        let starting_point = Utc.with_ymd_and_hms(2021, 10, 27, 0, 0, 0).unwrap();

        // Triggers on 2022-06-01. Note that the month and day are smaller than
        // the month and day in `starting_point`.
        let expression = "0 5 17 1 6 ? 2022".to_string();
        let schedule = Schedule::from_str(&expression).unwrap();
        let mut iter = schedule.after(&starting_point);
        assert!(iter.next().is_some());
    }

    #[test]
    fn test_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut iter = schedule.after(&Utc::now());
        assert!(iter.next().is_some());
    }

    #[test]
    fn test_next_after() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut iter = schedule.after(&Utc::now());
        assert!(iter.next().is_some());
    }

    #[test]
    fn test_upcoming_utc() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming(Utc);
        let next1 = upcoming.next();
        assert!(next1.is_some());
        let next2 = upcoming.next();
        assert!(next2.is_some());
        let next3 = upcoming.next();
        assert!(next3.is_some());
        println!("Upcoming 1 for {} {:?}", expression, next1);
        println!("Upcoming 2 for {} {:?}", expression, next2);
        println!("Upcoming 3 for {} {:?}", expression, next3);
    }

    #[test]
    fn test_upcoming_utc_owned() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming_owned(Utc);
        let next1 = upcoming.next();
        assert!(next1.is_some());
        let next2 = upcoming.next();
        assert!(next2.is_some());
        let next3 = upcoming.next();
        assert!(next3.is_some());
        println!("Upcoming 1 for {} {:?}", expression, next1);
        println!("Upcoming 2 for {} {:?}", expression, next2);
        println!("Upcoming 3 for {} {:?}", expression, next3);
    }

    #[test]
    fn test_upcoming_rev_utc() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming(Utc).rev();
        let prev1 = upcoming.next();
        assert!(prev1.is_some());
        let prev2 = upcoming.next();
        assert!(prev2.is_some());
        let prev3 = upcoming.next();
        assert!(prev3.is_some());
        println!("Prev Upcoming 1 for {} {:?}", expression, prev1);
        println!("Prev Upcoming 2 for {} {:?}", expression, prev2);
        println!("Prev Upcoming 3 for {} {:?}", expression, prev3);
    }

    #[test]
    fn test_upcoming_rev_utc_owned() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming_owned(Utc).rev();
        let prev1 = upcoming.next();
        assert!(prev1.is_some());
        let prev2 = upcoming.next();
        assert!(prev2.is_some());
        let prev3 = upcoming.next();
        assert!(prev3.is_some());
        println!("Prev Upcoming 1 for {} {:?}", expression, prev1);
        println!("Prev Upcoming 2 for {} {:?}", expression, prev2);
        println!("Prev Upcoming 3 for {} {:?}", expression, prev3);
    }

    #[test]
    fn test_upcoming_local() {
        use chrono::Local;
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming(Local);
        let next1 = upcoming.next();
        assert!(next1.is_some());
        let next2 = upcoming.next();
        assert!(next2.is_some());
        let next3 = upcoming.next();
        assert!(next3.is_some());
        println!("Upcoming 1 for {} {:?}", expression, next1);
        println!("Upcoming 2 for {} {:?}", expression, next2);
        println!("Upcoming 3 for {} {:?}", expression, next3);
    }

    #[test]
    fn test_schedule_to_string() {
        let expression = "* 1,2,3 * * * *";
        let schedule: Schedule = Schedule::from_str(expression).unwrap();
        let result = String::from(schedule);
        assert_eq!(expression, result);
    }

    #[test]
    fn test_display_schedule() {
        use std::fmt::Write;
        let expression = "@monthly";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut result = String::new();
        write!(result, "{}", schedule).unwrap();
        assert_eq!(expression, result);
    }

    #[test]
    fn test_valid_from_str() {
        let schedule = Schedule::from_str("0 0,30 0,6,12,18 1,15 Jan-March Thurs");
        schedule.unwrap();
    }

    #[test]
    fn test_invalid_from_str() {
        let schedule = Schedule::from_str("cheesecake 0,30 0,6,12,18 1,15 Jan-March Thurs");
        assert!(schedule.is_err());
    }

    #[test]
    fn test_no_panic_on_nonexistent_time_after() {
        use chrono::offset::TimeZone;
        use chrono_tz::Tz;

        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz
            .with_ymd_and_hms(2019, 10, 27, 0, 3, 29)
            .unwrap()
            .checked_add_signed(chrono::Duration::hours(1)) // puts it in the middle of the DST transition
            .unwrap();
        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let next = schedule.after(&dt).next().unwrap();
        assert!(next > dt); // test is ensuring line above does not panic
    }

    #[test]
    fn test_no_panic_on_nonexistent_time_before() {
        use chrono::offset::TimeZone;
        use chrono_tz::Tz;

        let schedule_tz: Tz = "Europe/London".parse().unwrap();
        let dt = schedule_tz
            .with_ymd_and_hms(2019, 10, 27, 0, 3, 29)
            .unwrap()
            .checked_add_signed(chrono::Duration::hours(1)) // puts it in the middle of the DST transition
            .unwrap();
        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let prev = schedule.after(&dt).nth_back(1).unwrap();
        assert!(prev < dt); // test is ensuring line above does not panic
    }

    #[test]
    fn test_no_panic_on_leap_day_time_after() {
        let dt = chrono::DateTime::parse_from_rfc3339("2024-02-29T10:00:00.000+08:00").unwrap();
        let schedule = Schedule::from_str("0 0 0 * * * 2100").unwrap();
        let next = schedule.after(&dt).next().unwrap();
        assert!(next > dt); // test is ensuring line above does not panic
    }

    #[test]
    fn test_time_unit_spec_equality() {
        let schedule_1 = Schedule::from_str("@weekly").unwrap();
        let schedule_2 = Schedule::from_str("0 0 0 * * 0 *").unwrap();
        let schedule_3 = Schedule::from_str("0 0 0 * * 0-6 *").unwrap();
        let schedule_4 = Schedule::from_str("0 0 0 * * * *").unwrap();
        assert_ne!(schedule_1, schedule_2);
        assert!(schedule_1.timeunitspec_eq(&schedule_2));
        assert!(schedule_3.timeunitspec_eq(&schedule_4));
    }

    #[test]
    fn test_dst_ambiguous_time_after() {
        use chrono_tz::Tz;

        let schedule_tz: Tz = "America/Chicago".parse().unwrap();
        let dt = schedule_tz
            .with_ymd_and_hms(2022, 11, 5, 23, 30, 0)
            .unwrap();
        let schedule = Schedule::from_str("0 0 * * * * *").unwrap();
        let times = schedule
            .after(&dt)
            .map(|x| x.to_string())
            .take(5)
            .collect::<Vec<_>>();
        let expected_times = [
            "2022-11-06 00:00:00 CDT".to_string(),
            "2022-11-06 01:00:00 CDT".to_string(),
            "2022-11-06 01:00:00 CST".to_string(), // 1 AM happens again
            "2022-11-06 02:00:00 CST".to_string(),
            "2022-11-06 03:00:00 CST".to_string(),
        ];

        assert_eq!(times.as_slice(), expected_times.as_slice());
    }

    #[test]
    fn test_dst_ambiguous_time_before() {
        use chrono_tz::Tz;

        let schedule_tz: Tz = "America/Chicago".parse().unwrap();
        let dt = schedule_tz.with_ymd_and_hms(2022, 11, 6, 3, 30, 0).unwrap();
        let schedule = Schedule::from_str("0 0 * * * * *").unwrap();
        let times = schedule
            .after(&dt)
            .map(|x| x.to_string())
            .rev()
            .take(5)
            .collect::<Vec<_>>();
        let expected_times = [
            "2022-11-06 03:00:00 CST".to_string(),
            "2022-11-06 02:00:00 CST".to_string(),
            "2022-11-06 01:00:00 CST".to_string(),
            "2022-11-06 01:00:00 CDT".to_string(), // 1 AM happens again
            "2022-11-06 00:00:00 CDT".to_string(),
        ];

        assert_eq!(times.as_slice(), expected_times.as_slice());
    }
}

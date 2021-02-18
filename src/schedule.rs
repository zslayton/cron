
use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, Timelike, Utc, SubsecRound};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::iter;

use crate::time_unit::*;
use crate::ordinal::*;

impl From<Schedule> for String {
    fn from(schedule: Schedule) -> String {
        schedule.source
    }
}

#[derive(Clone, Debug, Eq)]
pub struct Schedule {
    source: String,
    fields: ScheduleFields,
}

impl Schedule {
    pub(crate) fn new(
        source: String,
        fields: ScheduleFields,
    ) -> Schedule {
        Schedule {
            source,
            fields,
        }
    }

    fn next_after<Z>(&self, after: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone,
    {
        let after_rounded = &after.clone().trunc_subsecs(0);
        self.years()
            .iter()
            .skip_while(|year| *year < after_rounded.year() as u32)
            .flat_map(|year| {
                iter::repeat(
                    after_rounded.with_year(year as i32)
                )
                .zip(self.months().iter())
            })
            .skip_while(|(date, month)| {
                date.as_ref() < Some(after_rounded) ||
                (date.as_ref() == Some(after_rounded) && *month < after_rounded.month())
            })
            .flat_map(|(date, month)| {
                iter::repeat(
                    date.map(|d| d.with_month(month)).flatten()
                )
                .zip(self.days_of_month().iter())
            })
            .skip_while(|(date, day)| {
                date.as_ref() < Some(after_rounded) ||
                (date.as_ref() == Some(after_rounded) && *day < after_rounded.day())
            })
            .map(|(date, day)| {
                date.map(|d| d.with_day(day)).flatten()
            })
            .filter(|date| {
                date.as_ref()
                    .map(|d| self.days_of_week().includes(d.weekday().number_from_sunday()))
                    .unwrap_or(false)
            })
            .flat_map(|date| {
                iter::repeat(date).zip(self.hours().iter())
            })
            .skip_while(|(date, hour)| {
                date.as_ref() < Some(after_rounded) ||
                (date.as_ref() == Some(after_rounded) && *hour < after_rounded.hour())
            })
            .flat_map(|(date, hour)| {
                iter::repeat(
                    date.map(|d| d.with_hour(hour)).flatten()
                )
                .zip(self.minutes().iter())
            })
            .skip_while(|(date, minute)| {
                date.as_ref() < Some(after_rounded) ||
                (date.as_ref() == Some(after_rounded) && *minute < after_rounded.minute())
            })
            .flat_map(|(date, minute)| {
                iter::repeat(
                    date.map(|d| d.with_minute(minute)).flatten()
                )
                .zip(self.seconds().iter())
            })
            .skip_while(|(date, second)| {
                date.as_ref() < Some(after_rounded) ||
                (date.as_ref() == Some(after_rounded) && *second <= after_rounded.second())
            })
            .map(|(date, second)| {
                date.map(|d| d.with_second(second)).flatten()
            })
            .next()
            .flatten()
    }

    fn prev_from<Z>(&self, before: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone,
    {
        let before_rounded = &before.clone().trunc_subsecs(0);
        self.years()
            .iter()
            .rev()
            .skip_while(|year| *year < before_rounded.year() as u32)
            .flat_map(|year| {
                iter::repeat(
                    before_rounded.with_year(year as i32)
                )
                .zip(self.months().iter().rev())
            })
            .skip_while(|(date, month)| {
                date.as_ref() > Some(before_rounded) ||
                (date.as_ref() == Some(before_rounded) && *month > before_rounded.month())
            })
            .flat_map(|(date, month)| {
                iter::repeat(
                    date.map(|d| d.with_month(month)).flatten()
                )
                .zip(self.days_of_month().iter().rev())
            })
            .skip_while(|(date, day)| {
                date.as_ref() > Some(before_rounded) ||
                (date.as_ref() == Some(before_rounded) && *day > before_rounded.day())
            })
            .map(|(date, day)| {
                date.map(|d| d.with_day(day)).flatten()
            })
            .filter(|date| {
                date.as_ref()
                    .map(|d| self.days_of_week().includes(d.weekday().number_from_sunday()))
                    .unwrap_or(false)
            })
            .flat_map(|date| {
                iter::repeat(date)
                .zip(self.hours().iter().rev())
            })
            .skip_while(|(date, hour)| {
                date.as_ref() > Some(before_rounded) ||
                (date.as_ref() == Some(before_rounded) && *hour > before_rounded.hour())
            })
            .flat_map(|(date, hour)| {
                iter::repeat(
                    date.map(|d| d.with_hour(hour)).flatten()
                )
                .zip(self.minutes().iter().rev())
            })
            .skip_while(|(date, minute)| {
                date.as_ref() > Some(before_rounded) ||
                (date.as_ref() == Some(before_rounded) && *minute > before_rounded.minute())
            })
            .flat_map(|(date, minute)| {
                iter::repeat(
                    date.map(|d| d.with_minute(minute)).flatten()
                )
                .zip(self.seconds().iter().rev())
            })
            .skip_while(|(date, second)| {
                date.as_ref() > Some(before_rounded) ||
                (date.as_ref() == Some(before_rounded) && *second >= before_rounded.second())
            })
            .map(|(date, second)| {
                date.map(|d| d.with_second(second)).flatten()
            })
            .next()
            .flatten()
    }

    /// Provides an iterator which will return each DateTime that matches the schedule starting with
    /// the current time if applicable.
    pub fn upcoming<Z>(&self, timezone: Z) -> ScheduleIterator<Z>
    where
        Z: TimeZone,
    {
        self.after(&timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// Like the `upcoming` method, but allows you to specify a start time other than the present.
    pub fn after<Z>(&self, after: &DateTime<Z>) -> ScheduleIterator<Z>
    where
        Z: TimeZone,
    {
        ScheduleIterator::new(self, after)
    }

    pub fn includes<Z>(&self, date_time: DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.fields.years.includes(date_time.year() as Ordinal)  &&
        self.fields.months.includes(date_time.month() as Ordinal) &&
        self.fields.days_of_week.includes(date_time.weekday().number_from_sunday()) &&
        self.fields.days_of_month.includes(date_time.day() as Ordinal) &&
        self.fields.hours.includes(date_time.hour() as Ordinal) &&
        self.fields.minutes.includes(date_time.minute() as Ordinal) &&
        self.fields.minutes.includes(date_time.second() as Ordinal)
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the years included
    /// in this [Schedule](struct.Schedule.html).
    pub fn years(&self) -> &impl TimeUnitSpec {
        &self.fields.years
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the months of the year included
    /// in this [Schedule](struct.Schedule.html).
    pub fn months(&self) -> &impl TimeUnitSpec {
        &self.fields.months
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the days of the month included
    /// in this [Schedule](struct.Schedule.html).
    pub fn days_of_month(&self) -> &impl TimeUnitSpec {
        &self.fields.days_of_month
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the days of the week included
    /// in this [Schedule](struct.Schedule.html).
    pub fn days_of_week(&self) -> &impl TimeUnitSpec {
        &self.fields.days_of_week
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the hours of the day included
    /// in this [Schedule](struct.Schedule.html).
    pub fn hours(&self) -> &impl TimeUnitSpec {
        &self.fields.hours
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the minutes of the hour included
    /// in this [Schedule](struct.Schedule.html).
    pub fn minutes(&self) -> &impl TimeUnitSpec {
        &self.fields.minutes
    }

    /// Returns a [TimeUnitSpec](trait.TimeUnitSpec.html) describing the seconds of the minute included
    /// in this [Schedule](struct.Schedule.html).
    pub fn seconds(&self) -> &impl TimeUnitSpec {
        &self.fields.seconds
    }

    pub fn timeunitspec_eq(&self, other: &Schedule) -> bool {
        self.fields == other.fields
    }
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.source)
    }
}

impl PartialEq for Schedule {
    fn eq(&self, other: &Schedule) -> bool {
        self.source == other.source
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleFields {
    years: Years,
    days_of_week: DaysOfWeek,
    months: Months,
    days_of_month: DaysOfMonth,
    hours: Hours,
    minutes: Minutes,
    seconds: Seconds,
}

impl ScheduleFields {
    pub(crate) fn new(
        seconds: Seconds,
        minutes: Minutes,
        hours: Hours,
        days_of_month: DaysOfMonth,
        months: Months,
        days_of_week: DaysOfWeek,
        years: Years,
    ) -> ScheduleFields {
        ScheduleFields {
            years,
            days_of_week,
            months,
            days_of_month,
            hours,
            minutes,
            seconds,
        }
    }
}

pub struct ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    is_done: bool,
    schedule: &'a Schedule,
    previous_datetime: DateTime<Z>,
}
//TODO: Cutoff datetime?

impl<'a, Z> ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    fn new(schedule: &'a Schedule, starting_datetime: &DateTime<Z>) -> ScheduleIterator<'a, Z> {
        ScheduleIterator {
            is_done: false,
            schedule,
            previous_datetime: starting_datetime.clone(),
        }
    }
}

impl<'a, Z> Iterator for ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        if self.is_done {
            return None;
        }
        if let Some(next_datetime) = self.schedule.next_after(&self.previous_datetime) {
            self.previous_datetime = next_datetime.clone();
            Some(next_datetime)
        } else {
            self.is_done = true;
            None
        }
    }
}

impl<'a, Z> DoubleEndedIterator for ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.is_done {
            return None;
        }

        if let Some(prev_datetime) = self.schedule.prev_from(&self.previous_datetime) {
            self.previous_datetime = prev_datetime.clone();
            Some(prev_datetime)
        } else {
            self.is_done = true;
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::str::{FromStr};

    #[test]
    fn test_next_and_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();

        let next = schedule.next_after(&Utc::now());
        println!("NEXT AFTER for {} {:?}", expression, next);
        assert!(next.is_some());

        let next2 = schedule.next_after(&next.unwrap());
        println!("NEXT2 AFTER for {} {:?}", expression, next2);
        assert!(next2.is_some());

        let prev = schedule.prev_from(&next2.unwrap());
        println!("PREV FROM for {} {:?}", expression, prev);
        assert!(prev.is_some());
        assert_eq!(prev, next);
    }

    #[test]
    fn test_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let prev = schedule.prev_from(&Utc::now());
        println!("PREV FROM for {} {:?}", expression, prev);
        assert!(prev.is_some());
    }

    #[test]
    fn test_next_after() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let next = schedule.next_after(&Utc::now());
        println!("NEXT AFTER for {} {:?}", expression, next);
        assert!(next.is_some());
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
            .ymd(2019, 10, 27)
            .and_hms(0, 3, 29)
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
            .ymd(2019, 10, 27)
            .and_hms(0, 3, 29)
            .checked_add_signed(chrono::Duration::hours(1)) // puts it in the middle of the DST transition
            .unwrap();
        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let prev = schedule.after(&dt).rev().next().unwrap();
        assert!(prev < dt); // test is ensuring line above does not panic
    }
    
    #[test]
    fn test_time_unit_spec_equality() {
        let schedule_1 = Schedule::from_str("@weekly").unwrap();
        let schedule_2 = Schedule::from_str("0 0 0 * * 1 *").unwrap();
        let schedule_3 = Schedule::from_str("0 0 0 * * 1-7 *").unwrap();
        let schedule_4 = Schedule::from_str("0 0 0 * * * *").unwrap();
        assert_ne!(schedule_1, schedule_2);
        assert!(schedule_1.timeunitspec_eq(&schedule_2));
        assert!(schedule_3.timeunitspec_eq(&schedule_4));
    }
}
use chrono::offset::{LocalResult, TimeZone};
use chrono::{DateTime, Datelike, Timelike, Utc};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Bound::{Included, Unbounded};

#[cfg(feature = "serde")]
use core::fmt;
#[cfg(feature = "serde")]
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize, Serializer,
};

use crate::ordinal::*;
use crate::queries::*;
use crate::time_unit::*;

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
    pub(crate) fn new(source: String, fields: ScheduleFields) -> Schedule {
        Schedule { source, fields }
    }

    fn next_after<Z>(&self, after: &DateTime<Z>) -> LocalResult<DateTime<Z>>
    where
        Z: TimeZone,
    {
        let mut query = NextAfterQuery::from(after);
        for year in self
            .fields
            .years
            .ordinals()
            .range((Included(query.year_lower_bound()), Unbounded))
            .cloned()
        {
            // It's a future year, the current year's range is irrelevant.
            if year > after.year() as u32 {
                query.reset_month();
                query.reset_day_of_month();
            }
            let month_start = query.month_lower_bound();
            if !self.fields.months.ordinals().contains(&month_start) {
                query.reset_month();
            }
            let month_range = (Included(month_start), Included(Months::inclusive_max()));
            for month in self.fields.months.ordinals().range(month_range).cloned() {
                let day_of_month_start = query.day_of_month_lower_bound();
                let day_in_month = self.fields.days_of_month.days_in_month(month, year);
                if !day_in_month.contains(&day_of_month_start) {
                    query.reset_day_of_month();
                }
                let day_of_month_end = days_in_month(month, year);
                let day_of_month_range = (
                    Included(day_of_month_start.min(day_of_month_end)),
                    Included(day_of_month_end),
                );

                'day_loop: for day_of_month in day_in_month.range(day_of_month_range).cloned() {
                    let hour_start = query.hour_lower_bound();
                    if !self.fields.hours.ordinals().contains(&hour_start) {
                        query.reset_hour();
                    }
                    let hour_range = (Included(hour_start), Included(Hours::inclusive_max()));

                    for hour in self.fields.hours.ordinals().range(hour_range).cloned() {
                        let minute_start = query.minute_lower_bound();
                        if !self.fields.minutes.ordinals().contains(&minute_start) {
                            query.reset_minute();
                        }
                        let minute_range =
                            (Included(minute_start), Included(Minutes::inclusive_max()));

                        for minute in self.fields.minutes.ordinals().range(minute_range).cloned() {
                            let second_start = query.second_lower_bound();
                            if !self.fields.seconds.ordinals().contains(&second_start) {
                                query.reset_second();
                            }
                            let second_range =
                                (Included(second_start), Included(Seconds::inclusive_max()));

                            for second in
                                self.fields.seconds.ordinals().range(second_range).cloned()
                            {
                                let timezone = after.timezone();
                                let candidate = match timezone.with_ymd_and_hms(
                                    year as i32,
                                    month,
                                    day_of_month,
                                    hour,
                                    minute,
                                    second,
                                ) {
                                    LocalResult::None => continue,
                                    candidate => candidate,
                                };
                                if !self
                                    .fields
                                    .days_of_week
                                    .match_day_of(&candidate.clone().latest().unwrap())
                                {
                                    continue 'day_loop;
                                }
                                return candidate;
                            }
                            query.reset_minute();
                        } // End of minutes range
                        query.reset_hour();
                    } // End of hours range
                    query.reset_day_of_month();
                } // End of Day of Month range
                query.reset_month();
            } // End of Month range
        }

        // We ran out of dates to try.
        LocalResult::None
    }

    fn prev_from<Z>(&self, before: &DateTime<Z>) -> LocalResult<DateTime<Z>>
    where
        Z: TimeZone,
    {
        let mut query = PrevFromQuery::from(before);
        for year in self
            .fields
            .years
            .ordinals()
            .range((Unbounded, Included(query.year_upper_bound())))
            .rev()
            .cloned()
        {
            let month_start = query.month_upper_bound();

            if !self.fields.months.ordinals().contains(&month_start) {
                query.reset_month();
            }
            let month_range = (Included(Months::inclusive_min()), Included(month_start));

            for month in self
                .fields
                .months
                .ordinals()
                .range(month_range)
                .rev()
                .cloned()
            {
                let day_of_month_end = query.day_of_month_upper_bound();
                let day_in_month = self.fields.days_of_month.days_in_month(month, year);
                if !day_in_month.contains(&day_of_month_end) {
                    query.reset_day_of_month();
                }

                let day_of_month_end = days_in_month(month, year).min(day_of_month_end);

                let day_of_month_range = (
                    Included(DaysOfMonth::inclusive_min()),
                    Included(day_of_month_end),
                );

                'day_loop: for day_of_month in day_in_month.range(day_of_month_range).rev().cloned()
                {
                    let hour_start = query.hour_upper_bound();
                    if !self.fields.hours.ordinals().contains(&hour_start) {
                        query.reset_hour();
                    }
                    let hour_range = (Included(Hours::inclusive_min()), Included(hour_start));

                    for hour in self
                        .fields
                        .hours
                        .ordinals()
                        .range(hour_range)
                        .rev()
                        .cloned()
                    {
                        let minute_start = query.minute_upper_bound();
                        if !self.fields.minutes.ordinals().contains(&minute_start) {
                            query.reset_minute();
                        }
                        let minute_range =
                            (Included(Minutes::inclusive_min()), Included(minute_start));

                        for minute in self
                            .fields
                            .minutes
                            .ordinals()
                            .range(minute_range)
                            .rev()
                            .cloned()
                        {
                            let second_start = query.second_upper_bound();
                            if !self.fields.seconds.ordinals().contains(&second_start) {
                                query.reset_second();
                            }
                            let second_range =
                                (Included(Seconds::inclusive_min()), Included(second_start));

                            for second in self
                                .fields
                                .seconds
                                .ordinals()
                                .range(second_range)
                                .rev()
                                .cloned()
                            {
                                let timezone = before.timezone();
                                let candidate = match timezone.with_ymd_and_hms(
                                    year as i32,
                                    month,
                                    day_of_month,
                                    hour,
                                    minute,
                                    second,
                                ) {
                                    LocalResult::None => continue,
                                    some => some,
                                };
                                if !self
                                    .fields
                                    .days_of_week
                                    .match_day_of(&candidate.clone().latest().unwrap())
                                {
                                    continue 'day_loop;
                                }
                                return candidate;
                            }
                            query.reset_minute();
                        } // End of minutes range
                        query.reset_hour();
                    } // End of hours range
                    query.reset_day_of_month();
                } // End of Day of Month range
                query.reset_month();
            } // End of Month range
        }

        // We ran out of dates to try.
        LocalResult::None
    }

    /// Provides an iterator which will return each DateTime that matches the schedule starting with
    /// the current time if applicable.
    pub fn upcoming<Z>(&self, timezone: Z) -> ScheduleIterator<'_, Z>
    where
        Z: TimeZone,
    {
        self.after(&timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// The same, but with an iterator with a static ownership
    pub fn upcoming_owned<Z: TimeZone>(&self, timezone: Z) -> OwnedScheduleIterator<Z> {
        self.after_owned(timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// Like the `upcoming` method, but allows you to specify a start time other than the present.
    pub fn after<Z>(&self, after: &DateTime<Z>) -> ScheduleIterator<'_, Z>
    where
        Z: TimeZone,
    {
        ScheduleIterator::new(self, after)
    }

    /// The same, but with a static ownership.
    pub fn after_owned<Z: TimeZone>(&self, after: DateTime<Z>) -> OwnedScheduleIterator<Z> {
        OwnedScheduleIterator::new(self.clone(), after)
    }

    pub fn includes<Z>(&self, date_time: DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.fields.years.includes(date_time.year() as Ordinal)
            && self.fields.months.includes(date_time.month() as Ordinal)
            && self.fields.days_of_week.match_day_of(&date_time)
            && self
                .fields
                .days_of_month
                .days_in_month(date_time.month() as Ordinal, date_time.year() as Ordinal)
                .contains(&(date_time.day() as Ordinal))
            && self.fields.hours.includes(date_time.hour() as Ordinal)
            && self.fields.minutes.includes(date_time.minute() as Ordinal)
            && self.fields.seconds.includes(date_time.second() as Ordinal)
    }

    /// Returns a [TimeUnitSpec] describing the years included in this [Schedule].
    pub fn years(&self) -> &impl TimeUnitSpec {
        &self.fields.years
    }

    /// Returns a [TimeUnitSpec] describing the months of the year included in this [Schedule].
    pub fn months(&self) -> &impl TimeUnitSpec {
        &self.fields.months
    }

    /// Returns a [TimeUnitSpec] describing the days of the month included in this [Schedule].
    pub fn days_of_month(&self) -> &impl TimeUnitSpec {
        &self.fields.days_of_month
    }

    /// Returns a [TimeUnitSpec] describing the days of the week included in this [Schedule].
    pub fn days_of_week(&self) -> &impl TimeUnitSpec {
        &self.fields.days_of_week
    }

    /// Returns a [TimeUnitSpec] describing the hours of the day included in this [Schedule].
    pub fn hours(&self) -> &impl TimeUnitSpec {
        &self.fields.hours
    }

    /// Returns a [TimeUnitSpec] describing the minutes of the hour included in this [Schedule].
    pub fn minutes(&self) -> &impl TimeUnitSpec {
        &self.fields.minutes
    }

    /// Returns a [TimeUnitSpec] describing the seconds of the minute included in this [Schedule].
    pub fn seconds(&self) -> &impl TimeUnitSpec {
        &self.fields.seconds
    }

    pub fn timeunitspec_eq(&self, other: &Schedule) -> bool {
        self.fields == other.fields
    }

    /// Returns a reference to the source cron expression.
    pub fn source(&self) -> &str {
        &self.source
    }
}

impl Display for Schedule {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
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
    schedule: &'a Schedule,
    previous_datetime: Option<DateTime<Z>>,
    later_datetime: Option<DateTime<Z>>,
    earlier_datetime: Option<DateTime<Z>>,
}
//TODO: Cutoff datetime?

impl<'a, Z> ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    fn new(schedule: &'a Schedule, starting_datetime: &DateTime<Z>) -> Self {
        ScheduleIterator {
            schedule,
            previous_datetime: Some(starting_datetime.clone()),
            later_datetime: None,
            earlier_datetime: None,
        }
    }
}

impl<Z> Iterator for ScheduleIterator<'_, Z>
where
    Z: TimeZone,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        let previous = self.previous_datetime.take()?;

        if let Some(later) = self.later_datetime.take() {
            self.previous_datetime = Some(later.clone());
            Some(later)
        } else {
            match self.schedule.next_after(&previous) {
                LocalResult::Single(next) => {
                    self.previous_datetime = Some(next.clone());
                    Some(next)
                }
                LocalResult::Ambiguous(earlier, later) => {
                    self.previous_datetime = Some(earlier.clone());
                    self.later_datetime = Some(later);
                    Some(earlier)
                }
                LocalResult::None => None,
            }
        }
    }
}

impl<Z> DoubleEndedIterator for ScheduleIterator<'_, Z>
where
    Z: TimeZone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        if let Some(earlier) = self.earlier_datetime.take() {
            self.previous_datetime = Some(earlier.clone());
            Some(earlier)
        } else {
            match self.schedule.prev_from(&previous) {
                LocalResult::Single(prev) => {
                    self.previous_datetime = Some(prev.clone());
                    Some(prev)
                }
                LocalResult::Ambiguous(earlier, later) => {
                    self.previous_datetime = Some(later.clone());
                    self.earlier_datetime = Some(earlier);
                    Some(later)
                }
                LocalResult::None => None,
            }
        }
    }
}

/// A `ScheduleIterator` with a static lifetime.
pub struct OwnedScheduleIterator<Z>
where
    Z: TimeZone,
{
    schedule: Schedule,
    previous_datetime: Option<DateTime<Z>>,
    // In the case of the Daylight Savings Time transition where an hour is
    // gained, store the time that occurs twice.  Depending on which direction
    // the iteration goes, this needs to be stored separately to keep the
    // direction of time (becoming earlier or later) consistent.
    later_datetime: Option<DateTime<Z>>,
    earlier_datetime: Option<DateTime<Z>>,
}

impl<Z> OwnedScheduleIterator<Z>
where
    Z: TimeZone,
{
    pub fn new(schedule: Schedule, starting_datetime: DateTime<Z>) -> Self {
        Self {
            schedule,
            previous_datetime: Some(starting_datetime),
            later_datetime: None,
            earlier_datetime: None,
        }
    }
}

impl<Z> Iterator for OwnedScheduleIterator<Z>
where
    Z: TimeZone,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        let previous = self.previous_datetime.take()?;

        if let Some(later) = self.later_datetime.take() {
            self.previous_datetime = Some(later.clone());
            Some(later)
        } else {
            match self.schedule.next_after(&previous) {
                LocalResult::Single(next) => {
                    self.previous_datetime = Some(next.clone());
                    Some(next)
                }
                // Handle an "Ambiguous" time, such as during the end of
                // Daylight Savings Time, transitioning from BST to GMT, where
                // for example, in London, 2AM occurs twice when the hour is
                // moved back during the fall.
                LocalResult::Ambiguous(earlier, later) => {
                    self.previous_datetime = Some(earlier.clone());
                    self.later_datetime = Some(later);
                    Some(earlier)
                }
                LocalResult::None => None,
            }
        }
    }
}

impl<Z: TimeZone> DoubleEndedIterator for OwnedScheduleIterator<Z> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        if let Some(earlier) = self.earlier_datetime.take() {
            self.previous_datetime = Some(earlier.clone());
            Some(earlier)
        } else {
            match self.schedule.prev_from(&previous) {
                LocalResult::Single(prev) => {
                    self.previous_datetime = Some(prev.clone());
                    Some(prev)
                }
                // Handle an "Ambiguous" time, such as during the end of
                // Daylight Savings Time, transitioning from BST to GMT, where
                // for example, in London, 2AM occurs twice when the hour is
                // moved back during the fall.
                LocalResult::Ambiguous(earlier, later) => {
                    self.previous_datetime = Some(later.clone());
                    self.earlier_datetime = Some(earlier);
                    Some(later)
                }
                LocalResult::None => None,
            }
        }
    }
}

#[cfg(feature = "serde")]
struct ScheduleVisitor;

#[cfg(feature = "serde")]
impl Visitor<'_> for ScheduleVisitor {
    type Value = Schedule;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a valid cron expression")
    }

    // Supporting `Deserializer`s shall provide an owned `String`.
    //
    // The `Schedule` will decode from a `&str` to it,
    // then store the owned `String` as `Schedule::source`.
    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Schedule::try_from(v).map_err(de::Error::custom)
    }

    // `Deserializer`s not providing an owned `String`
    // shall provide a `&str`.
    //
    // The `Schedule` will decode from the `&str`,
    // then clone into the heap to store as an owned `String`
    // as `Schedule::source`.
    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Schedule::try_from(v).map_err(de::Error::custom)
    }
}

#[cfg(feature = "serde")]
impl Serialize for Schedule {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.source())
    }
}

#[cfg(feature = "serde")]
impl<'de> Deserialize<'de> for Schedule {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Hint that the `Deserialize` type `Schedule`
        // would benefit from taking ownership of
        // buffered data owned by the `Deserializer`:
        //
        // The deserialization "happy path" decodes from a `&str`,
        // then stores the source as owned `String`.
        //
        // Thus, the optimized happy path receives an owned `String`
        // if the `Deserializer` in use supports providing one.
        deserializer.deserialize_string(ScheduleVisitor)
    }
}

#[cfg(test)]
mod test {
    use chrono::Duration;
    #[cfg(feature = "serde")]
    use serde_test::{assert_tokens, Token};

    use super::*;
    use std::str::FromStr;

    #[cfg(feature = "serde")]
    #[test]
    fn test_ser_de_schedule_tokens() {
        let schedule = Schedule::from_str("* * * * * * *").expect("valid format");
        assert_tokens(&schedule, &[Token::String("* * * * * * *")])
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_invalid_ser_de_schedule_tokens() {
        use serde_test::assert_de_tokens_error;

        assert_de_tokens_error::<Schedule>(
            &[Token::String(
                "definitively an invalid value for a cron schedule!",
            )],
            "definitively an invalid value for a cron schedule!\n\
                ^\n\
                The 'Seconds' field does not support using names. 'definitively' specified.",
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ser_de_schedule_shorthand() {
        let serialized = postcard::to_stdvec(&Schedule::try_from("@hourly").expect("valid format"))
            .expect("serializable schedule");

        let schedule: Schedule =
            postcard::from_bytes(&serialized).expect("deserializable schedule");

        let starting_date = Utc.with_ymd_and_hms(2017, 2, 25, 22, 29, 36).unwrap();
        assert!([
            Utc.with_ymd_and_hms(2017, 2, 25, 23, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2017, 2, 26, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2017, 2, 26, 1, 0, 0).unwrap(),
        ]
        .into_iter()
        .eq(schedule.after(&starting_date).take(3)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ser_de_schedule_period_values_range() {
        let serialized =
            postcard::to_stdvec(&Schedule::try_from("0 0 0 1-31/10 * ?").expect("valid format"))
                .expect("serializable schedule");

        let schedule: Schedule =
            postcard::from_bytes(&serialized).expect("deserializable schedule");

        let starting_date = Utc.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        assert!([
            Utc.with_ymd_and_hms(2020, 1, 11, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2020, 1, 21, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2020, 1, 31, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2020, 2, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2020, 2, 11, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2020, 2, 21, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2020, 3, 1, 0, 0, 0).unwrap(),
        ]
        .into_iter()
        .eq(schedule.after(&starting_date).take(7)));
    }

    #[test]
    fn test_next_and_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();

        let next = schedule.next_after(&Utc::now());
        println!("NEXT AFTER for {} {:?}", expression, next);
        assert!(next.single().is_some());

        let next2 = schedule.next_after(&next.unwrap());
        println!("NEXT2 AFTER for {} {:?}", expression, next2);
        assert!(next2.single().is_some());

        let prev = schedule.prev_from(&next2.unwrap());
        println!("PREV FROM for {} {:?}", expression, prev);
        assert!(prev.single().is_some());
        assert_eq!(prev, next);

        let prev2 = schedule.prev_from(&(next2.unwrap() + Duration::nanoseconds(100)));
        println!("PREV2 FROM for {} {:?}", expression, prev2);
        assert!(prev2.single().is_some());
        assert_eq!(prev2, next2);
    }

    #[test]
    fn test_next_after_past_date_next_year() {
        // Schedule after 2021-10-27
        let starting_point = Utc.with_ymd_and_hms(2021, 10, 27, 0, 0, 0).unwrap();

        // Triggers on 2022-06-01. Note that the month and day are smaller than
        // the month and day in `starting_point`.
        let expression = "0 5 17 1 6 ? 2022".to_string();
        let schedule = Schedule::from_str(&expression).unwrap();
        let next = schedule.next_after(&starting_point);
        println!("NEXT AFTER for {} {:?}", expression, next);
        assert!(next.single().is_some());
    }

    #[test]
    fn test_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let prev = schedule.prev_from(&Utc::now());
        println!("PREV FROM for {} {:?}", expression, prev);
        assert!(prev.single().is_some());
    }

    #[test]
    fn test_next_after() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let next = schedule.next_after(&Utc::now());
        println!("NEXT AFTER for {} {:?}", expression, next);
        assert!(next.single().is_some());
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
        let schedule_2 = Schedule::from_str("0 0 0 * * 1 *").unwrap();
        let schedule_3 = Schedule::from_str("0 0 0 * * 1-7 *").unwrap();
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

    #[test]
    fn test_last_specifier_in_days_of_month() {
        let schedule = Schedule::from_str("0 0 0 1-25/10,L,L-2 6 ? 2025").unwrap();
        let all_dates = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-06-12T00:00:00.000Z").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            all_dates.as_slice(),
            &[
                chrono::DateTime::parse_from_rfc3339("2025-06-21T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-28T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-30T00:00:00.000Z").unwrap(),
            ]
        );

        let all_dates = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-06-30T12:00:00.000Z").unwrap())
            .rev()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>();
        assert_eq!(
            all_dates.as_slice(),
            &[
                chrono::DateTime::parse_from_rfc3339("2025-06-01T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-11T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-21T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-28T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-30T00:00:00.000Z").unwrap(),
            ]
        );
    }

    #[test]
    fn test_too_big_last_specifier_in_days_of_month() {
        assert!(Schedule::from_str("0 0 0 L-28 6 ? 2025").is_ok());
        assert!(Schedule::from_str("0 0 0 L-29 6 ? 2025").is_err());
    }

    #[test]
    fn test_weekday_specifier_in_days_of_month() {
        let schedule = Schedule::from_str("0 0 0 10W,15W,21W,LW 6 ? 2025").unwrap();
        let all_dates = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-06-02T00:00:00.000Z").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            all_dates.as_slice(),
            &[
                chrono::DateTime::parse_from_rfc3339("2025-06-10T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-16T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-20T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-30T00:00:00.000Z").unwrap(),
            ]
        );

        // Test for particular case where the closest weekday is not in the same month:
        // (in such case we still return a weekday in the same month)

        // Test that if sunday is last day of the month, LW (or WL) returns the last friday
        let schedule = Schedule::from_str("0 0 0 WL 8 ? 2025").unwrap();

        let the_date = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-08-02T00:00:00.000Z").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            the_date,
            chrono::DateTime::parse_from_rfc3339("2025-08-29T00:00:00.000Z").unwrap()
        );

        // Test that if saturday is the first day of the month, LW return the first monday
        let schedule = Schedule::from_str("0 0 0 1W 3 ? 2025").unwrap();
        let the_date = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-02-28T00:00:00.000Z").unwrap())
            .next()
            .unwrap();
        assert_eq!(
            the_date,
            chrono::DateTime::parse_from_rfc3339("2025-03-03T00:00:00.000Z").unwrap()
        );
    }

    #[test]
    fn test_last_specifier_in_days_of_week() {
        let schedule = Schedule::from_str("0 0 0 * 6 FRIL,3L,L 2025").unwrap();
        let all_dates = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-06-12T00:00:00.000Z").unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            all_dates.as_slice(),
            &[
                chrono::DateTime::parse_from_rfc3339("2025-06-14T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-21T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-24T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-27T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-28T00:00:00.000Z").unwrap(),
            ]
        );
    }

    #[test]
    fn test_occurrence_specifier_in_days_of_week() {
        let schedule = Schedule::from_str("0 0 0 * 6 MON#3,5#4 2025").unwrap();
        let all_dates = schedule
            .after(&chrono::DateTime::parse_from_rfc3339("2025-06-12T00:00:00.000Z").unwrap())
            .collect::<Vec<_>>();
        println!("NEXT {:?} AFTER for {:?}", all_dates, schedule);
        assert_eq!(
            all_dates.as_slice(),
            &[
                chrono::DateTime::parse_from_rfc3339("2025-06-16T00:00:00.000Z").unwrap(),
                chrono::DateTime::parse_from_rfc3339("2025-06-26T00:00:00.000Z").unwrap(),
            ]
        );
    }
}

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
        #[cfg(feature = "vixie")]
        let dow_and_dom_specific =
            !self.fields.days_of_week.is_all() && !self.fields.days_of_month.is_all();

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
            }
            let month_start = query.month_lower_bound();
            if !self.fields.months.ordinals().contains(&month_start) {
                query.reset_month();
            }
            let month_range = (Included(month_start), Included(Months::inclusive_max()));
            for month in self.fields.months.ordinals().range(month_range).cloned() {
                let days_of_month = self.fields.days_of_month.ordinals();
                let day_of_month_start = query.day_of_month_lower_bound();
                let day_of_month_end = days_in_month(month, year);
                let day_of_month_range = (
                    Included(day_of_month_start.min(day_of_month_end)),
                    Included(day_of_month_end),
                );

                #[cfg(not(feature = "vixie"))]
                let mut day_of_month_candidates = days_of_month
                    .range(day_of_month_range)
                    .cloned()
                    .filter(|dom| {
                        self.fields
                            .days_of_week
                            .ordinals()
                            .contains(&day_of_week(year, month, *dom))
                    })
                    .peekable();

                #[cfg(feature = "vixie")]
                let mut day_of_month_candidates = {
                    let days_of_week = self.fields.days_of_week.ordinals();

                    (day_of_month_start..=day_of_month_end)
                        .into_iter()
                        .filter(|dom| {
                            if dow_and_dom_specific {
                                return days_of_month.contains(dom)
                                    || days_of_week.contains(&day_of_week(year, month, *dom));
                            }
                            days_of_month.contains(dom)
                                && days_of_week.contains(&day_of_week(year, month, *dom))
                        })
                        .peekable()
                };

                if day_of_month_candidates.peek() != Some(&day_of_month_start) {
                    query.reset_day_of_month();
                }

                for day_of_month in day_of_month_candidates {
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
                                return match timezone.with_ymd_and_hms(
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
        #[cfg(feature = "vixie")]
        let dow_and_dom_specific =
            !self.fields.days_of_week.is_all() && !self.fields.days_of_month.is_all();

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
                let days_of_month = self.fields.days_of_month.ordinals();
                let day_of_month_end = query.day_of_month_upper_bound();
                let day_of_month_range = (
                    Included(DaysOfMonth::inclusive_min()),
                    Included(days_in_month(month, year).min(day_of_month_end)),
                );

                #[cfg(not(feature = "vixie"))]
                let mut day_of_month_candidates = days_of_month
                    .range(day_of_month_range)
                    .rev()
                    .cloned()
                    .filter(|dom| {
                        self.fields
                            .days_of_week
                            .ordinals()
                            .contains(&day_of_week(year, month, *dom))
                    })
                    .peekable();

                #[cfg(feature = "vixie")]
                let mut day_of_month_candidates = {
                    let days_of_week = self.fields.days_of_week.ordinals();

                    (DaysOfMonth::inclusive_min()..=day_of_month_end)
                        .into_iter()
                        .rev()
                        .filter(|dom| {
                            if dow_and_dom_specific {
                                return days_of_month.contains(dom)
                                    || days_of_week.contains(&day_of_week(year, month, *dom));
                            }
                            days_of_month.contains(dom)
                                && days_of_week.contains(&day_of_week(year, month, *dom))
                        })
                        .peekable()
                };

                if day_of_month_candidates.peek() != Some(&day_of_month_end) {
                    query.reset_day_of_month();
                }

                for day_of_month in day_of_month_candidates {
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
                                return match timezone.with_ymd_and_hms(
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

    #[cfg(feature = "vixie")]
    /// Vixie cron behavior: If DOM is specific and DOW is inspecific, then only DOM is considered.
    /// If DOW is specific and DOM is inspecific, then only DOW is considered
    /// If both are specific, then either is considered.
    fn includes_dom_dow<Z>(&self, date_time: &DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        let dow_inspecific = self.fields.days_of_week.is_all();
        let dom_inspecific = self.fields.days_of_month.is_all();
        let dow_includes = self
            .fields
            .days_of_week
            .includes(date_time.weekday().number_from_sunday());
        let dom_includes = self
            .fields
            .days_of_month
            .includes(date_time.day() as Ordinal);

        (dow_inspecific || dom_inspecific)
            && (!dow_inspecific || dow_includes)
            && (!dom_inspecific || dom_includes)
    }

    #[cfg(not(feature = "vixie"))]
    /// Quartz (the default) cron behavior: Both DOM and DOW must match.
    fn includes_dom_dow<Z>(&self, date_time: &DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.fields
            .days_of_week
            .includes(date_time.weekday().number_from_sunday())
            && self
                .fields
                .days_of_month
                .includes(date_time.day() as Ordinal)
    }

    pub fn includes<Z>(&self, date_time: DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.fields.years.includes(date_time.year() as Ordinal)
            && self.fields.months.includes(date_time.month() as Ordinal)
            && self.includes_dom_dow(&date_time)
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

fn is_leap_year(year: Ordinal) -> bool {
    let by_four = year % 4 == 0;
    let by_hundred = year % 100 == 0;
    let by_four_hundred = year % 400 == 0;
    by_four && ((!by_hundred) || by_four_hundred)
}

fn days_in_month(month: Ordinal, year: Ordinal) -> u32 {
    let is_leap_year = is_leap_year(year);
    match month {
        9 | 4 | 6 | 11 => 30,
        2 if is_leap_year => 29,
        2 => 28,
        _ => 31,
    }
}

#[cfg(not(feature = "vixie"))]
fn day_of_week(year: u32, month: u32, day: u32) -> u32 {
    chrono::NaiveDate::from_ymd_opt(year as i32, month, day)
        .unwrap()
        .weekday()
        .number_from_sunday()
}

#[cfg(feature = "vixie")]
fn day_of_week(year: u32, month: u32, day: u32) -> u32 {
    chrono::NaiveDate::from_ymd_opt(year as i32, month, day)
        .unwrap()
        .weekday()
        .num_days_from_sunday()
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
    #[cfg(feature = "serde")]
    use serde_test::{assert_tokens, Token};

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
}

#[cfg(feature = "chrono")]
use chrono::offset::TimeZone;
#[cfg(feature = "chrono")]
use chrono::{DateTime, Utc};
use std::cmp::{max, min};
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::Bound::{Included, Unbounded};

#[cfg(feature = "serde")]
use core::fmt;
#[cfg(feature = "serde")]
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize, Serializer,
};

use crate::datetime::{CronDateTime, LocalFields, LocalMapping};
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

    fn next_after<T: CronDateTime>(&self, after: &T) -> Option<T> {
        let mut query = NextAfterQuery::from(after);
        // Ambiguous naive datetimes translate to two local datetimes. This
        // iteration uses naive datetimes and could skip the second local
        // datetime, where the second may match the pattern. This deferred
        // candidate retains that datetime for consideration on subsequent
        // iterations. For example, during fall back, `0 0/30 * * * * *` must
        // emit both 01:30 local datetimes.
        let mut deferred_candidate: Option<T> = None;
        // Folded time happens during the fall back DST transition where an
        // offset, typically one hour, is repeated in both timezones. This flag
        // tells iteration it is starting in the first repeated offset so it
        // does not get stuck repeating matches from the first fold or skip
        // matches in the second fold.
        let after_local = after.local_fields();
        let after_in_first_fold = after.fold().is_some_and(|(earlier, _)| *after == earlier);
        for year in self
            .fields
            .years
            .ordinals()
            .range((Included(query.year_lower_bound()), Unbounded))
            .cloned()
        {
            // It's a future year, the current year's range is irrelevant.
            if year > after_local.year as u32 {
                query.reset_month();
            }
            let month_start = query.month_lower_bound();
            if !self.fields.months.ordinals().contains(&month_start) {
                query.reset_month();
            }
            let month_range = (Included(month_start), Included(Months::inclusive_max()));
            for month in self.fields.months.ordinals().range(month_range).cloned() {
                let day_of_month_start = query.day_of_month_lower_bound();
                if !self
                    .fields
                    .days_of_month
                    .ordinals()
                    .contains(&day_of_month_start)
                {
                    query.reset_day_of_month();
                }
                let day_of_month_end = days_in_month(month, year);
                let day_of_month_range = (
                    Included(day_of_month_start.min(day_of_month_end)),
                    Included(day_of_month_end),
                );

                let mut day_iter = self
                    .fields
                    .days_of_month
                    .ordinals()
                    .range(day_of_month_range)
                    .cloned()
                    .filter(|&day| {
                        self.fields.days_of_week.is_all()
                            || T::date_weekday_from_sunday(year as i32, month, day)
                                .map(|weekday| {
                                    self.fields.days_of_week.ordinals().contains(&weekday)
                                })
                                .unwrap_or(false)
                    })
                    .peekable();
                if day_iter.peek() != Some(&day_of_month_start) {
                    query.reset_day_of_month();
                }
                for day_of_month in day_iter {
                    let hour_start = query.hour_lower_bound();
                    if !self.fields.hours.ordinals().contains(&hour_start) {
                        query.reset_hour();
                    }
                    let hour_range = (Included(hour_start), Included(Hours::inclusive_max()));

                    for hour in self.fields.hours.ordinals().range(hour_range).cloned() {
                        // The first fold is the first repeat of the DST offset,
                        // typically one hour. Because this iteration is done in
                        // naive time, it can skip matches after finding one in
                        // the first fold. For example, `0 0/30 * * * * *` must
                        // continue from 01:00 to 01:30 in both folds.
                        let fold_hour_scan = after_in_first_fold
                            && year as i32 == after_local.year
                            && month == after_local.month
                            && day_of_month == after_local.day
                            && hour == after_local.hour;
                        let query_minute_start = query.minute_lower_bound();
                        let minute_start = if fold_hour_scan {
                            Minutes::inclusive_min()
                        } else {
                            query_minute_start
                        };
                        if !self.fields.minutes.ordinals().contains(&minute_start) {
                            query.reset_minute();
                        }
                        let minute_range =
                            (Included(minute_start), Included(Minutes::inclusive_max()));

                        for minute in self.fields.minutes.ordinals().range(minute_range).cloned() {
                            let query_second_start = query.second_lower_bound();
                            let second_start = if fold_hour_scan {
                                Seconds::inclusive_min()
                            } else {
                                query_second_start
                            };
                            if !self.fields.seconds.ordinals().contains(&second_start) {
                                query.reset_second();
                            }
                            let second_range =
                                (Included(second_start), Included(Seconds::inclusive_max()));

                            for second in
                                self.fields.seconds.ordinals().range(second_range).cloned()
                            {
                                let local_result = after.with_local_fields(LocalFields {
                                    year: year as i32,
                                    month,
                                    day: day_of_month,
                                    hour,
                                    minute,
                                    second,
                                });
                                match local_result {
                                    LocalMapping::None => continue,
                                    LocalMapping::Single(candidate) => {
                                        if candidate <= *after {
                                            continue;
                                        }
                                        if let Some(deferred) = deferred_candidate.take() {
                                            return Some(min(deferred, candidate));
                                        }
                                        return Some(candidate);
                                    }
                                    LocalMapping::Ambiguous(earlier, later) => {
                                        if earlier > *after {
                                            if let Some(deferred) = deferred_candidate.take() {
                                                return Some(min(deferred, earlier));
                                            }
                                            return Some(earlier);
                                        }
                                        if later > *after {
                                            deferred_candidate = Some(match deferred_candidate {
                                                Some(existing) => min(existing, later),
                                                _ => later,
                                            });
                                        }
                                    }
                                }
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

        if let Some(candidate) = deferred_candidate {
            return Some(candidate);
        }
        // We ran out of dates to try.
        None
    }

    fn prev_from<T: CronDateTime>(&self, before: &T) -> Option<T> {
        let mut query = PrevFromQuery::from(before);
        // See `next_after` for folded-time details. This is the reverse scan's
        // deferred candidate for an earlier local datetime that may still be
        // the previous chronological match.
        let mut deferred_candidate: Option<T> = None;
        // See `next_after` for folded-time details. This flag handles starting
        // from the second fold while scanning backward.
        let before_local = before.local_fields();
        let before_in_second_fold = before.fold().is_some_and(|(_, later)| *before == later);
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
                if !self
                    .fields
                    .days_of_month
                    .ordinals()
                    .contains(&day_of_month_end)
                {
                    query.reset_day_of_month();
                }

                let day_of_month_end = days_in_month(month, year).min(day_of_month_end);

                let day_of_month_range = (
                    Included(DaysOfMonth::inclusive_min()),
                    Included(day_of_month_end),
                );

                let mut day_iter = self
                    .fields
                    .days_of_month
                    .ordinals()
                    .range(day_of_month_range)
                    .rev()
                    .cloned()
                    .filter(|&day| {
                        self.fields.days_of_week.is_all()
                            || T::date_weekday_from_sunday(year as i32, month, day)
                                .map(|weekday| {
                                    self.fields.days_of_week.ordinals().contains(&weekday)
                                })
                                .unwrap_or(false)
                    })
                    .peekable();
                if day_iter.peek() != Some(&day_of_month_end) {
                    query.reset_day_of_month();
                }
                for day_of_month in day_iter {
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
                        // See the forward `fold_hour_scan` for folded-time
                        // details. This is the reverse scan of the repeated
                        // hour from the second fold into the first.
                        let fold_hour_scan = before_in_second_fold
                            && year as i32 == before_local.year
                            && month == before_local.month
                            && day_of_month == before_local.day
                            && hour == before_local.hour;
                        let query_minute_start = query.minute_upper_bound();
                        let minute_start = if fold_hour_scan {
                            Minutes::inclusive_max()
                        } else {
                            query_minute_start
                        };
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
                            let query_second_start = query.second_upper_bound();
                            let second_start = if fold_hour_scan {
                                Seconds::inclusive_max()
                            } else {
                                query_second_start
                            };
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
                                let local_result = before.with_local_fields(LocalFields {
                                    year: year as i32,
                                    month,
                                    day: day_of_month,
                                    hour,
                                    minute,
                                    second,
                                });
                                match local_result {
                                    LocalMapping::None => continue,
                                    LocalMapping::Single(candidate) => {
                                        if candidate >= *before {
                                            continue;
                                        }
                                        if let Some(deferred) = deferred_candidate.take() {
                                            return Some(max(deferred, candidate));
                                        }
                                        return Some(candidate);
                                    }
                                    LocalMapping::Ambiguous(earlier, later) => {
                                        if later < *before {
                                            if let Some(deferred) = deferred_candidate.take() {
                                                return Some(max(deferred, later));
                                            }
                                            return Some(later);
                                        }
                                        if earlier < *before {
                                            deferred_candidate = Some(match deferred_candidate {
                                                Some(existing) => max(existing, earlier),
                                                _ => earlier,
                                            });
                                        }
                                    }
                                }
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

        if let Some(candidate) = deferred_candidate {
            return Some(candidate);
        }
        // We ran out of dates to try.
        None
    }

    /// Provides an iterator which will return each DateTime that matches the schedule starting with
    /// the current time if applicable.
    #[cfg(feature = "chrono")]
    pub fn upcoming<Z>(&self, timezone: Z) -> ScheduleIterator<'_, Z>
    where
        Z: TimeZone,
    {
        self.after(&timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// The same, but with an iterator with a static ownership
    #[cfg(feature = "chrono")]
    pub fn upcoming_owned<Z: TimeZone>(&self, timezone: Z) -> OwnedScheduleIterator<Z> {
        self.after_owned(timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// Like the `upcoming` method, but allows you to specify a start time other than the present.
    #[cfg(feature = "chrono")]
    pub fn after<Z>(&self, after: &DateTime<Z>) -> ScheduleIterator<'_, Z>
    where
        Z: TimeZone,
    {
        ScheduleIterator::new(self, after)
    }

    /// The same, but with a static ownership.
    #[cfg(feature = "chrono")]
    pub fn after_owned<Z: TimeZone>(&self, after: DateTime<Z>) -> OwnedScheduleIterator<Z> {
        OwnedScheduleIterator::new(self.clone(), after)
    }

    #[cfg(feature = "chrono")]
    pub fn includes<Z>(&self, date_time: DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.includes_datetime(&date_time)
    }

    fn includes_datetime<T: CronDateTime>(&self, date_time: &T) -> bool {
        let local = date_time.local_fields();
        self.fields.years.includes(local.year as Ordinal)
            && self.fields.months.includes(local.month)
            && self
                .fields
                .days_of_week
                .includes(date_time.weekday_from_sunday())
            && self.fields.days_of_month.includes(local.day)
            && self.fields.hours.includes(local.hour)
            && self.fields.minutes.includes(local.minute)
            && self.fields.seconds.includes(local.second)
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

/// Schedules for [`jiff::ZonedDateTime`]. Requires the `jiff` feature.
#[cfg(feature = "jiff")]
impl Schedule {
    /// Provides an iterator which will return each [`jiff::ZonedDateTime`] in `time_zone` that
    /// matches the schedule starting with the current time if applicable.
    pub fn upcoming_zoned(&self, time_zone: jiff::tz::TimeZone) -> ZonedScheduleIterator<'_> {
        self.after_zoned(&jiff::Timestamp::now().to_zoned_datetime(time_zone))
    }

    /// The same, but with an iterator with a static ownership.
    pub fn upcoming_zoned_owned(
        &self,
        time_zone: jiff::tz::TimeZone,
    ) -> OwnedZonedScheduleIterator {
        self.after_zoned_owned(jiff::Timestamp::now().to_zoned_datetime(time_zone))
    }

    /// Like the `upcoming_zoned` method, but allows you to specify a start time other than the
    /// present. The iterator uses the time zone of `after`.
    pub fn after_zoned(&self, after: &jiff::ZonedDateTime) -> ZonedScheduleIterator<'_> {
        ZonedScheduleIterator::new(self, after)
    }

    /// The same, but with a static ownership.
    pub fn after_zoned_owned(&self, after: jiff::ZonedDateTime) -> OwnedZonedScheduleIterator {
        OwnedZonedScheduleIterator::new(self.clone(), after)
    }

    /// Whether the schedule includes `date_time`, using its local date and time.
    pub fn includes_zoned(&self, date_time: &jiff::ZonedDateTime) -> bool {
        self.includes_datetime(date_time)
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

#[cfg(feature = "chrono")]
pub struct ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    schedule: &'a Schedule,
    previous_datetime: Option<DateTime<Z>>,
}
//TODO: Cutoff datetime?

#[cfg(feature = "chrono")]
impl<'a, Z> ScheduleIterator<'a, Z>
where
    Z: TimeZone,
{
    fn new(schedule: &'a Schedule, starting_datetime: &DateTime<Z>) -> Self {
        ScheduleIterator {
            schedule,
            previous_datetime: Some(starting_datetime.clone()),
        }
    }
}

#[cfg(feature = "chrono")]
impl<Z> Iterator for ScheduleIterator<'_, Z>
where
    Z: TimeZone,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        let previous = self.previous_datetime.take()?;

        let next = self.schedule.next_after(&previous)?;
        self.previous_datetime = Some(next.clone());
        Some(next)
    }
}

#[cfg(feature = "chrono")]
impl<Z> DoubleEndedIterator for ScheduleIterator<'_, Z>
where
    Z: TimeZone,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        let prev = self.schedule.prev_from(&previous)?;
        self.previous_datetime = Some(prev.clone());
        Some(prev)
    }
}

/// A `ScheduleIterator` with a static lifetime.
#[cfg(feature = "chrono")]
pub struct OwnedScheduleIterator<Z>
where
    Z: TimeZone,
{
    schedule: Schedule,
    previous_datetime: Option<DateTime<Z>>,
}

#[cfg(feature = "chrono")]
impl<Z> OwnedScheduleIterator<Z>
where
    Z: TimeZone,
{
    pub fn new(schedule: Schedule, starting_datetime: DateTime<Z>) -> Self {
        Self {
            schedule,
            previous_datetime: Some(starting_datetime),
        }
    }
}

#[cfg(feature = "chrono")]
impl<Z> Iterator for OwnedScheduleIterator<Z>
where
    Z: TimeZone,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        let previous = self.previous_datetime.take()?;

        let next = self.schedule.next_after(&previous)?;
        self.previous_datetime = Some(next.clone());
        Some(next)
    }
}

#[cfg(feature = "chrono")]
impl<Z: TimeZone> DoubleEndedIterator for OwnedScheduleIterator<Z> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        let prev = self.schedule.prev_from(&previous)?;
        self.previous_datetime = Some(prev.clone());
        Some(prev)
    }
}

/// An iterator over the [`jiff::ZonedDateTime`]s that match a [`Schedule`]. Requires the `jiff`
/// feature.
#[cfg(feature = "jiff")]
pub struct ZonedScheduleIterator<'a> {
    schedule: &'a Schedule,
    previous_datetime: Option<jiff::ZonedDateTime>,
}

#[cfg(feature = "jiff")]
impl<'a> ZonedScheduleIterator<'a> {
    fn new(schedule: &'a Schedule, starting_datetime: &jiff::ZonedDateTime) -> Self {
        ZonedScheduleIterator {
            schedule,
            previous_datetime: Some(starting_datetime.clone()),
        }
    }
}

#[cfg(feature = "jiff")]
impl Iterator for ZonedScheduleIterator<'_> {
    type Item = jiff::ZonedDateTime;

    fn next(&mut self) -> Option<jiff::ZonedDateTime> {
        let previous = self.previous_datetime.take()?;

        let next = self.schedule.next_after(&previous)?;
        self.previous_datetime = Some(next.clone());
        Some(next)
    }
}

#[cfg(feature = "jiff")]
impl DoubleEndedIterator for ZonedScheduleIterator<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        let prev = self.schedule.prev_from(&previous)?;
        self.previous_datetime = Some(prev.clone());
        Some(prev)
    }
}

/// A `ZonedScheduleIterator` with a static lifetime. Requires the `jiff` feature.
#[cfg(feature = "jiff")]
pub struct OwnedZonedScheduleIterator {
    schedule: Schedule,
    previous_datetime: Option<jiff::ZonedDateTime>,
}

#[cfg(feature = "jiff")]
impl OwnedZonedScheduleIterator {
    pub fn new(schedule: Schedule, starting_datetime: jiff::ZonedDateTime) -> Self {
        Self {
            schedule,
            previous_datetime: Some(starting_datetime),
        }
    }
}

#[cfg(feature = "jiff")]
impl Iterator for OwnedZonedScheduleIterator {
    type Item = jiff::ZonedDateTime;

    fn next(&mut self) -> Option<jiff::ZonedDateTime> {
        let previous = self.previous_datetime.take()?;

        let next = self.schedule.next_after(&previous)?;
        self.previous_datetime = Some(next.clone());
        Some(next)
    }
}

#[cfg(feature = "jiff")]
impl DoubleEndedIterator for OwnedZonedScheduleIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        let prev = self.schedule.prev_from(&previous)?;
        self.previous_datetime = Some(prev.clone());
        Some(prev)
    }
}

fn is_leap_year(year: Ordinal) -> bool {
    let by_four = year.is_multiple_of(4);
    let by_hundred = year.is_multiple_of(100);
    let by_four_hundred = year.is_multiple_of(400);
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

#[cfg(all(test, feature = "chrono"))]
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
        assert!(next.is_some());

        let next2 = schedule.next_after(&next.unwrap());
        println!("NEXT2 AFTER for {} {:?}", expression, next2);
        assert!(next2.is_some());

        let prev = schedule.prev_from(&next2.unwrap());
        println!("PREV FROM for {} {:?}", expression, prev);
        assert!(prev.is_some());
        assert_eq!(prev, next);

        let prev2 = schedule.prev_from(&(next2.unwrap() + Duration::nanoseconds(100)));
        println!("PREV2 FROM for {} {:?}", expression, prev2);
        assert!(prev2.is_some());
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
        assert!(next.is_some());
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
}

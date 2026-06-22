use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike};
use std::ops::Bound;
use std::ops::Bound::{Included, Unbounded};

use crate::ordinal::Ordinal;
use crate::schedule::ScheduleFields;
use crate::time_unit::{DaysOfMonth, Hours, Minutes, Months, Seconds, TimeUnitField};

pub(crate) trait Cursor<Z>
where
    Z: TimeZone,
{
    fn initial_datetime(&self) -> &DateTime<Z>;
    fn first_month(&mut self) -> &mut bool;
    fn first_day_of_month(&mut self) -> &mut bool;
    fn first_hour(&mut self) -> &mut bool;
    fn first_minute(&mut self) -> &mut bool;
    fn first_second(&mut self) -> &mut bool;

    fn cursor_month_bound(&mut self, year: Ordinal, default: Ordinal) -> Ordinal {
        if *self.first_month() {
            *self.first_month() = false;
            if year == self.initial_datetime().year() as u32 {
                return self.initial_datetime().month();
            }
        }
        default
    }

    fn cursor_day_of_month_bound(&mut self, default: Ordinal) -> Ordinal {
        if *self.first_day_of_month() {
            *self.first_day_of_month() = false;
            return self.initial_datetime().day();
        }
        default
    }

    fn cursor_hour_bound(&mut self, default: Ordinal) -> Ordinal {
        if *self.first_hour() {
            *self.first_hour() = false;
            return self.initial_datetime().hour();
        }
        default
    }

    fn cursor_minute_bound(&mut self, default: Ordinal) -> Ordinal {
        if *self.first_minute() {
            *self.first_minute() = false;
            return self.initial_datetime().minute();
        }
        default
    }

    fn cursor_second_bound(&mut self, default: Ordinal) -> Ordinal {
        if *self.first_second() {
            *self.first_second() = false;
            return self.initial_datetime().second();
        }
        default
    }

    fn reset_month(&mut self) {
        *self.first_month() = false;
        self.reset_day_of_month();
    }

    fn reset_day_of_month(&mut self) {
        *self.first_day_of_month() = false;
        self.reset_hour();
    }

    fn reset_hour(&mut self) {
        *self.first_hour() = false;
        self.reset_minute();
    }

    fn reset_minute(&mut self) {
        *self.first_minute() = false;
        self.reset_second();
    }

    fn reset_second(&mut self) {
        *self.first_second() = false;
    }
}

pub(crate) trait Query<Z>: Cursor<Z>
where
    Z: TimeZone,
{
    fn year_range(&self) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn month_default_bound(&self) -> Ordinal;
    fn month_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn day_of_month_default_bound(&self) -> Ordinal;
    fn day_of_month_range(
        &self,
        bound: Ordinal,
        day_of_month_end: Ordinal,
    ) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn hour_default_bound(&self) -> Ordinal;
    fn hour_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn minute_default_bound(&self) -> Ordinal;
    fn minute_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn second_default_bound(&self) -> Ordinal;
    fn second_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn is_reversed(&self) -> bool;
    fn preceeds_reference_datetime(&self, candidate: &DateTime<Z>) -> bool;
    fn preferred_candidate(&self, lhs: DateTime<Z>, rhs: DateTime<Z>) -> DateTime<Z>;

    fn years<'a>(&self, fields: &'a ScheduleFields) -> Box<dyn Iterator<Item = &'a Ordinal> + 'a> {
        order_iter(
            self.is_reversed(),
            fields.years_ordinals().range(self.year_range()),
        )
    }

    fn months<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        year: Ordinal,
    ) -> Box<dyn Iterator<Item = &'a Ordinal> + 'a> {
        let bound = self.cursor_month_bound(year, self.month_default_bound());
        if !fields.months_ordinals().contains(&bound) {
            self.reset_month();
        }
        order_iter(
            self.is_reversed(),
            fields.months_ordinals().range(self.month_range(bound)),
        )
    }

    fn days_of_month<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        year: Ordinal,
        month: Ordinal,
    ) -> Box<dyn Iterator<Item = &'a Ordinal> + 'a> {
        let day_of_month_end = days_in_month(month, year);
        let bound = self.cursor_day_of_month_bound(self.day_of_month_default_bound());

        let range = self.day_of_month_range(bound, day_of_month_end);
        let iter = fields
            .days_of_month_ordinals()
            .range(range)
            .filter(move |day| {
                let day = **day;
                fields.days_of_week_is_all()
                    || NaiveDate::from_ymd_opt(year as i32, month, day)
                        .map(|d| fields.includes_day_of_week(d.weekday().number_from_sunday()))
                        .unwrap_or(false)
            });

        let mut ordered = order_iter(self.is_reversed(), iter).peekable();
        let expected_start = bound.min(day_of_month_end);
        if ordered.peek().map(|day| **day) != Some(expected_start) {
            self.reset_day_of_month();
        }
        Box::new(ordered)
    }

    fn hours<'a>(
        &mut self,
        fields: &'a ScheduleFields,
    ) -> Box<dyn Iterator<Item = &'a Ordinal> + 'a> {
        let bound = self.cursor_hour_bound(self.hour_default_bound());
        if !fields.hours_ordinals().contains(&bound) {
            self.reset_hour();
        }
        order_iter(
            self.is_reversed(),
            fields.hours_ordinals().range(self.hour_range(bound)),
        )
    }

    fn minutes<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        fold_hour_scan: bool,
    ) -> Box<dyn Iterator<Item = &'a Ordinal> + 'a> {
        let query_bound = self.cursor_minute_bound(self.minute_default_bound());
        let bound = if fold_hour_scan {
            self.minute_default_bound()
        } else {
            query_bound
        };
        if !fields.minutes_ordinals().contains(&bound) {
            self.reset_minute();
        }
        order_iter(
            self.is_reversed(),
            fields.minutes_ordinals().range(self.minute_range(bound)),
        )
    }

    fn seconds<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        fold_hour_scan: bool,
    ) -> Box<dyn Iterator<Item = &'a Ordinal> + 'a> {
        let query_bound = self.cursor_second_bound(self.second_default_bound());
        let bound = if fold_hour_scan {
            self.second_default_bound()
        } else {
            query_bound
        };
        if !fields.seconds_ordinals().contains(&bound) {
            self.reset_second();
        }
        order_iter(
            self.is_reversed(),
            fields.seconds_ordinals().range(self.second_range(bound)),
        )
    }
}

fn order_iter<'a, I, T>(reverse: bool, iter: I) -> Box<dyn Iterator<Item = T> + 'a>
where
    I: DoubleEndedIterator<Item = T> + 'a,
    T: 'a,
{
    if reverse {
        Box::new(iter.rev())
    } else {
        Box::new(iter)
    }
}

pub struct NextAfterQuery<Z>
where
    Z: TimeZone,
{
    reference_datetime: DateTime<Z>,
    initial_datetime: DateTime<Z>,
    first_month: bool,
    first_day_of_month: bool,
    first_hour: bool,
    first_minute: bool,
    first_second: bool,
}

impl<Z> NextAfterQuery<Z>
where
    Z: TimeZone,
{
    pub fn from(after: &DateTime<Z>) -> NextAfterQuery<Z> {
        NextAfterQuery {
            reference_datetime: after.clone(),
            initial_datetime: after.clone(),
            first_month: true,
            first_day_of_month: true,
            first_hour: true,
            first_minute: true,
            first_second: true,
        }
    }
}

impl<Z> Cursor<Z> for NextAfterQuery<Z>
where
    Z: TimeZone,
{
    fn initial_datetime(&self) -> &DateTime<Z> {
        &self.initial_datetime
    }

    fn first_month(&mut self) -> &mut bool {
        &mut self.first_month
    }

    fn first_day_of_month(&mut self) -> &mut bool {
        &mut self.first_day_of_month
    }

    fn first_hour(&mut self) -> &mut bool {
        &mut self.first_hour
    }

    fn first_minute(&mut self) -> &mut bool {
        &mut self.first_minute
    }

    fn first_second(&mut self) -> &mut bool {
        &mut self.first_second
    }
}

impl<Z> Query<Z> for NextAfterQuery<Z>
where
    Z: TimeZone,
{
    fn year_range(&self) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(self.initial_datetime.year() as u32), Unbounded)
    }

    fn month_default_bound(&self) -> Ordinal {
        Months::inclusive_min()
    }

    fn month_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(bound), Included(Months::inclusive_max()))
    }

    fn day_of_month_default_bound(&self) -> Ordinal {
        DaysOfMonth::inclusive_min()
    }

    fn day_of_month_range(
        &self,
        bound: Ordinal,
        day_of_month_end: Ordinal,
    ) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (
            Included(bound.min(day_of_month_end)),
            Included(day_of_month_end),
        )
    }

    fn hour_default_bound(&self) -> Ordinal {
        Hours::inclusive_min()
    }

    fn hour_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(bound), Included(Hours::inclusive_max()))
    }

    fn minute_default_bound(&self) -> Ordinal {
        Minutes::inclusive_min()
    }

    fn minute_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(bound), Included(Minutes::inclusive_max()))
    }

    fn second_default_bound(&self) -> Ordinal {
        Seconds::inclusive_min()
    }

    fn second_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(bound), Included(Seconds::inclusive_max()))
    }

    fn is_reversed(&self) -> bool {
        false
    }

    fn preceeds_reference_datetime(&self, candidate: &DateTime<Z>) -> bool {
        *candidate > self.reference_datetime
    }

    fn preferred_candidate(&self, lhs: DateTime<Z>, rhs: DateTime<Z>) -> DateTime<Z> {
        if lhs < rhs {
            lhs
        } else {
            rhs
        }
    }
}

pub struct PrevFromQuery<Z>
where
    Z: TimeZone,
{
    reference_datetime: DateTime<Z>,
    initial_datetime: DateTime<Z>,
    first_month: bool,
    first_day_of_month: bool,
    first_hour: bool,
    first_minute: bool,
    first_second: bool,
}

impl<Z> PrevFromQuery<Z>
where
    Z: TimeZone,
{
    pub fn from(before: &DateTime<Z>) -> PrevFromQuery<Z> {
        let initial_datetime = if before.timestamp_subsec_nanos() > 0 {
            before.clone()
        } else {
            before.clone() - Duration::seconds(1)
        };
        PrevFromQuery {
            reference_datetime: before.clone(),
            initial_datetime,
            first_month: true,
            first_day_of_month: true,
            first_hour: true,
            first_minute: true,
            first_second: true,
        }
    }
}

impl<Z> Cursor<Z> for PrevFromQuery<Z>
where
    Z: TimeZone,
{
    fn initial_datetime(&self) -> &DateTime<Z> {
        &self.initial_datetime
    }

    fn first_month(&mut self) -> &mut bool {
        &mut self.first_month
    }

    fn first_day_of_month(&mut self) -> &mut bool {
        &mut self.first_day_of_month
    }

    fn first_hour(&mut self) -> &mut bool {
        &mut self.first_hour
    }

    fn first_minute(&mut self) -> &mut bool {
        &mut self.first_minute
    }

    fn first_second(&mut self) -> &mut bool {
        &mut self.first_second
    }
}

impl<Z> Query<Z> for PrevFromQuery<Z>
where
    Z: TimeZone,
{
    fn year_range(&self) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Unbounded, Included(self.initial_datetime.year() as u32))
    }

    fn month_default_bound(&self) -> Ordinal {
        Months::inclusive_max()
    }

    fn month_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(Months::inclusive_min()), Included(bound))
    }

    fn day_of_month_default_bound(&self) -> Ordinal {
        DaysOfMonth::inclusive_max()
    }

    fn day_of_month_range(
        &self,
        bound: Ordinal,
        day_of_month_end: Ordinal,
    ) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (
            Included(DaysOfMonth::inclusive_min()),
            Included(bound.min(day_of_month_end)),
        )
    }

    fn hour_default_bound(&self) -> Ordinal {
        Hours::inclusive_max()
    }

    fn hour_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(Hours::inclusive_min()), Included(bound))
    }

    fn minute_default_bound(&self) -> Ordinal {
        Minutes::inclusive_max()
    }

    fn minute_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(Minutes::inclusive_min()), Included(bound))
    }

    fn second_default_bound(&self) -> Ordinal {
        Seconds::inclusive_max()
    }

    fn second_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>) {
        (Included(Seconds::inclusive_min()), Included(bound))
    }

    fn is_reversed(&self) -> bool {
        true
    }

    fn preceeds_reference_datetime(&self, candidate: &DateTime<Z>) -> bool {
        *candidate < self.reference_datetime
    }

    fn preferred_candidate(&self, lhs: DateTime<Z>, rhs: DateTime<Z>) -> DateTime<Z> {
        if lhs > rhs {
            lhs
        } else {
            rhs
        }
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

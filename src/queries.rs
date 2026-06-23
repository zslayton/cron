use chrono::offset::TimeZone;
use chrono::{DateTime, Datelike, Duration, NaiveDate, Timelike};
use std::ops::Bound::{Included, Unbounded};
use std::ops::{Bound, RangeInclusive};

use crate::ordinal::{OrderedOrdinalSetIter, Ordinal};
use crate::schedule::ScheduleFields;
use crate::time_unit::{
    days_in_month, DaysOfMonth, Hours, Minutes, Months, Seconds, TimeUnitField, YearRangeIter,
};
use crate::DowDomOperand;

pub(crate) enum OrdinalQueryIter<'a> {
    Empty,
    Range {
        front: Ordinal,
        back: Ordinal,
        reverse: bool,
        exhausted: bool,
    },
    Set(OrderedOrdinalSetIter<'a>),
    YearRange {
        iter: YearRangeIter<'a>,
        reverse: bool,
    },
    Vec(std::vec::IntoIter<Ordinal>),
}

impl<'a> OrdinalQueryIter<'a> {
    fn empty() -> Self {
        Self::Empty
    }

    fn range(start: Ordinal, end: Ordinal, reverse: bool) -> Self {
        if start > end {
            Self::Empty
        } else {
            Self::Range {
                front: start,
                back: end,
                reverse,
                exhausted: false,
            }
        }
    }

    fn vec(mut ordinals: Vec<Ordinal>, reverse: bool) -> Self {
        if reverse {
            ordinals.reverse();
        }
        Self::Vec(ordinals.into_iter())
    }

    fn year_range(iter: YearRangeIter<'a>, reverse: bool) -> Self {
        Self::YearRange { iter, reverse }
    }
}

impl Iterator for OrdinalQueryIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Range {
                front,
                back,
                reverse,
                exhausted,
            } => {
                if *exhausted {
                    return None;
                }

                let candidate = if *reverse {
                    let candidate = *back;
                    if front == back {
                        *exhausted = true;
                    } else {
                        *back -= 1;
                    }
                    candidate
                } else {
                    let candidate = *front;
                    if front == back {
                        *exhausted = true;
                    } else {
                        *front += 1;
                    }
                    candidate
                };
                Some(candidate)
            }
            Self::Set(iter) => iter.next(),
            Self::YearRange { iter, reverse } => {
                if *reverse {
                    iter.next_back()
                } else {
                    iter.next()
                }
            }
            Self::Vec(iter) => iter.next(),
        }
    }
}

pub(crate) struct DayOfMonthQueryIter<'a> {
    pending: Option<Ordinal>,
    inner: DayOfMonthIter<'a>,
}

impl Iterator for DayOfMonthQueryIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        self.pending.take().or_else(|| self.inner.next())
    }
}

enum DayOfMonthIter<'a> {
    Plain(OrdinalQueryIter<'a>),
    Filtered {
        base: OrdinalQueryIter<'a>,
        fields: &'a ScheduleFields,
        year: Ordinal,
        month: Ordinal,
        operand: DowDomOperand,
        first_weekday: Ordinal,
    },
}

impl Iterator for DayOfMonthIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Plain(iter) => iter.next(),
            Self::Filtered {
                base,
                fields,
                year,
                month,
                operand,
                first_weekday,
            } => base.find(|day| {
                fields.day_matches(
                    *year,
                    *month,
                    *day,
                    weekday_for_day(*first_weekday, *day),
                    *operand,
                )
            }),
        }
    }
}

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
            self.reset_day_of_month();
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
    fn year_range(&self, search_interval: Duration) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn month_default_bound(&self) -> Ordinal;
    fn month_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn day_of_month_default_bound(&self) -> Ordinal;
    fn day_of_month_range(
        &self,
        bound: Ordinal,
        day_of_month_end: Ordinal,
    ) -> RangeInclusive<Ordinal>;
    fn hour_default_bound(&self) -> Ordinal;
    fn hour_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn minute_default_bound(&self) -> Ordinal;
    fn minute_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn second_default_bound(&self) -> Ordinal;
    fn second_range(&self, bound: Ordinal) -> (Bound<Ordinal>, Bound<Ordinal>);
    fn is_reversed(&self) -> bool;
    fn preceeds_reference_datetime(&self, candidate: &DateTime<Z>) -> bool;
    fn preferred_candidate(&self, lhs: DateTime<Z>, rhs: DateTime<Z>) -> DateTime<Z>;

    fn years<'a>(
        &self,
        fields: &'a ScheduleFields,
        search_interval: Duration,
        enforce_search_interval: bool,
    ) -> OrdinalQueryIter<'a> {
        if fields.years_are_unrestricted() && !enforce_search_interval {
            let year = self.initial_datetime().year() as Ordinal;
            return if self.is_reversed() {
                OrdinalQueryIter::range(0, year, true)
            } else {
                OrdinalQueryIter::range(year, i32::MAX as Ordinal, false)
            };
        }

        let Some((start, end)) = ordinal_bounds(self.year_range(search_interval)) else {
            return OrdinalQueryIter::empty();
        };

        if start > end {
            return OrdinalQueryIter::empty();
        }

        if fields.years_are_unrestricted() {
            return OrdinalQueryIter::range(start, end, self.is_reversed());
        }

        OrdinalQueryIter::year_range(fields.years_between(start, end), self.is_reversed())
    }

    fn months<'a>(&mut self, fields: &'a ScheduleFields, year: Ordinal) -> OrdinalQueryIter<'a> {
        let bound = self.cursor_month_bound(year, self.month_default_bound());
        let ordinals = fields.months_ordinals();
        if ordinals.is_all() {
            return all_range_iter(self.month_range(bound), self.is_reversed());
        }
        if !ordinals.contains(&bound) {
            self.reset_month();
        }
        OrdinalQueryIter::Set(ordinals.ordered_range(self.month_range(bound), self.is_reversed()))
    }

    fn days_of_month<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        year: Ordinal,
        month: Ordinal,
        operand: DowDomOperand,
    ) -> DayOfMonthQueryIter<'a> {
        let day_of_month_end = days_in_month(month, year);
        let bound = self.cursor_day_of_month_bound(self.day_of_month_default_bound());

        let range = self.day_of_month_range(bound, day_of_month_end);
        let start = *range.start();
        let end = *range.end();
        let both_restricted = !fields.days_of_month_is_all() && !fields.days_of_week_is_all();
        let should_scan_all_days = operand == DowDomOperand::Or && both_restricted;

        let base_iter = if should_scan_all_days || fields.days_of_month_is_all() {
            OrdinalQueryIter::range(start, end, self.is_reversed())
        } else if fields.days_of_month_has_special_specifiers() {
            OrdinalQueryIter::vec(
                fields
                    .days_of_month_ordinals_for_month(year, month, day_of_month_end, range)
                    .into_iter()
                    .collect(),
                self.is_reversed(),
            )
        } else {
            OrdinalQueryIter::Set(
                fields
                    .days_of_month_ordinals()
                    .ordered_range(range, self.is_reversed()),
            )
        };

        let mut inner =
            if fields.days_of_week_is_all() && !(should_scan_all_days && both_restricted) {
                DayOfMonthIter::Plain(base_iter)
            } else if let Some(first_weekday) = first_weekday_for_month(year, month) {
                DayOfMonthIter::Filtered {
                    base: base_iter,
                    fields,
                    year,
                    month,
                    operand,
                    first_weekday,
                }
            } else {
                DayOfMonthIter::Plain(OrdinalQueryIter::empty())
            };

        let expected_start = bound.min(day_of_month_end);
        let pending = inner.next();
        if pending != Some(expected_start) {
            self.reset_day_of_month();
        }
        DayOfMonthQueryIter { pending, inner }
    }

    fn hours<'a>(&mut self, fields: &'a ScheduleFields) -> OrdinalQueryIter<'a> {
        let bound = self.cursor_hour_bound(self.hour_default_bound());
        let ordinals = fields.hours_ordinals();
        if ordinals.is_all() {
            return all_range_iter(self.hour_range(bound), self.is_reversed());
        }
        if !ordinals.contains(&bound) {
            self.reset_hour();
        }
        OrdinalQueryIter::Set(ordinals.ordered_range(self.hour_range(bound), self.is_reversed()))
    }

    fn minutes<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        fold_hour_scan: bool,
    ) -> OrdinalQueryIter<'a> {
        let query_bound = self.cursor_minute_bound(self.minute_default_bound());
        let bound = if fold_hour_scan {
            self.minute_default_bound()
        } else {
            query_bound
        };
        let ordinals = fields.minutes_ordinals();
        if ordinals.is_all() {
            return all_range_iter(self.minute_range(bound), self.is_reversed());
        }
        if !ordinals.contains(&bound) {
            self.reset_minute();
        }
        OrdinalQueryIter::Set(ordinals.ordered_range(self.minute_range(bound), self.is_reversed()))
    }

    fn seconds<'a>(
        &mut self,
        fields: &'a ScheduleFields,
        fold_hour_scan: bool,
    ) -> OrdinalQueryIter<'a> {
        let query_bound = self.cursor_second_bound(self.second_default_bound());
        let bound = if fold_hour_scan {
            self.second_default_bound()
        } else {
            query_bound
        };
        let ordinals = fields.seconds_ordinals();
        if ordinals.is_all() {
            return all_range_iter(self.second_range(bound), self.is_reversed());
        }
        if !ordinals.contains(&bound) {
            self.reset_second();
        }
        OrdinalQueryIter::Set(ordinals.ordered_range(self.second_range(bound), self.is_reversed()))
    }
}

fn all_range_iter(
    range: (Bound<Ordinal>, Bound<Ordinal>),
    reverse: bool,
) -> OrdinalQueryIter<'static> {
    let Some((start, end)) = ordinal_bounds(range) else {
        return OrdinalQueryIter::empty();
    };
    OrdinalQueryIter::range(start, end, reverse)
}

fn ordinal_bounds(range: (Bound<Ordinal>, Bound<Ordinal>)) -> Option<(Ordinal, Ordinal)> {
    Some((lower_ordinal_bound(range.0)?, upper_ordinal_bound(range.1)?))
}

fn first_weekday_for_month(year: Ordinal, month: Ordinal) -> Option<Ordinal> {
    NaiveDate::from_ymd_opt(year as i32, month, 1).map(|date| date.weekday().number_from_sunday())
}

fn weekday_for_day(first_weekday: Ordinal, day: Ordinal) -> Ordinal {
    ((first_weekday + day - 2) % 7) + 1
}

fn lower_ordinal_bound(bound: Bound<Ordinal>) -> Option<Ordinal> {
    match bound {
        Included(ordinal) => Some(ordinal),
        Bound::Excluded(ordinal) => ordinal.checked_add(1),
        Unbounded => Some(0),
    }
}

fn upper_ordinal_bound(bound: Bound<Ordinal>) -> Option<Ordinal> {
    match bound {
        Included(ordinal) => Some(ordinal),
        Bound::Excluded(ordinal) => ordinal.checked_sub(1),
        Unbounded => Some(i32::MAX as Ordinal),
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
    fn year_range(&self, search_interval: Duration) -> (Bound<Ordinal>, Bound<Ordinal>) {
        let upper = self
            .initial_datetime
            .clone()
            .checked_add_signed(search_interval)
            .map(|datetime| Included(datetime.year() as u32))
            .unwrap_or(Unbounded);

        (Included(self.initial_datetime.year() as u32), upper)
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
    ) -> RangeInclusive<Ordinal> {
        bound.min(day_of_month_end)..=day_of_month_end
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
    fn year_range(&self, search_interval: Duration) -> (Bound<Ordinal>, Bound<Ordinal>) {
        let lower = self
            .initial_datetime
            .clone()
            .checked_sub_signed(search_interval)
            .map(|datetime| Included(datetime.year() as u32))
            .unwrap_or(Unbounded);

        (lower, Included(self.initial_datetime.year() as u32))
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
    ) -> RangeInclusive<Ordinal> {
        DaysOfMonth::inclusive_min()..=bound.min(day_of_month_end)
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

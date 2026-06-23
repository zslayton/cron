use chrono::offset::{LocalResult, Offset, TimeZone};
use chrono::{DateTime, Datelike, NaiveDate, NaiveDateTime, NaiveTime, TimeDelta, Timelike, Utc};
use chrono_tz::GapInfo;
use std::any::Any;
use std::fmt::{Display, Formatter, Result as FmtResult};
use std::ops::RangeInclusive;

#[cfg(feature = "serde")]
use core::fmt;
#[cfg(feature = "serde")]
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize, Serializer,
};

use crate::error::Error;
use crate::ordinal::*;
use crate::queries::*;
use crate::time_unit::*;
use crate::{
    CronScheduleParts, DayOfWeekNumbering, DowDomOperand, NonexistentTimeBehavior, ScheduleConfig,
};

// This is an offset probe, not the local gap search span. A nonexistent local
// time's corresponding UTC transition can be shifted by the zone's UTC offset.
const NONEXISTENT_OFFSET_PROBE_SECONDS: i64 = 18 * 60 * 60;

impl From<Schedule> for String {
    fn from(schedule: Schedule) -> String {
        schedule.source
    }
}

#[derive(Clone, Debug, Eq)]
pub struct Schedule {
    source: String,
    fields: ScheduleFields,
    config: ScheduleConfig,
}

impl Schedule {
    /// Returns a builder configured with default parsing behavior.
    pub fn builder() -> ScheduleConfigBuilder {
        ScheduleConfigBuilder::default()
    }

    /// Returns a default parser builder.
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> ScheduleConfigBuilder {
        Schedule::builder()
    }

    /// Returns a parser builder configured for Vixie cron behavior.
    pub fn vixie() -> ScheduleConfigBuilder {
        Schedule::builder()
            .day_of_week_numbering(DayOfWeekNumbering::ZeroIndexed)
            .wraparound_ranges(true)
            .last_specifiers(true)
            .nearest_weekday(true)
            .nth_weekday_of_month(true)
            .dow_dom_operand(DowDomOperand::Or)
    }

    pub(crate) fn new(source: String, fields: ScheduleFields, config: ScheduleConfig) -> Schedule {
        Schedule {
            source,
            fields,
            config,
        }
    }

    fn next_after<Z>(&self, after: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone + 'static,
    {
        self.find_with_query(NextAfterQuery::from(after), after)
    }

    fn prev_from<Z>(&self, before: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone + 'static,
    {
        self.find_with_query(PrevFromQuery::from(before), before)
    }

    fn find_with_query<Z, Q>(&self, mut query: Q, datetime: &DateTime<Z>) -> Option<DateTime<Z>>
    where
        Z: TimeZone + 'static,
        Q: Query<Z>,
    {
        let mut deferred_candidate: Option<DateTime<Z>> = None;
        let reference_naive = datetime.naive_local();
        let timezone = datetime.timezone();
        let enforce_search_interval = self.enforces_search_interval();
        let fold_scan_active = match timezone.from_local_datetime(&reference_naive) {
            LocalResult::Ambiguous(first, second) => {
                *datetime == query.preferred_candidate(first, second)
            }
            _ => false,
        };
        for year in query.years(
            &self.fields,
            self.config.search_interval,
            enforce_search_interval,
        ) {
            for month in query.months(&self.fields, year) {
                for day_of_month in
                    query.days_of_month(&self.fields, year, month, self.config.dow_dom_operand)
                {
                    for hour in query.hours(&self.fields) {
                        let fold_hour_scan = fold_scan_active
                            && year as i32 == reference_naive.year()
                            && month == reference_naive.month()
                            && day_of_month == reference_naive.day()
                            && hour == reference_naive.hour();
                        for minute in query.minutes(&self.fields, fold_hour_scan) {
                            for second in query.seconds(&self.fields, fold_hour_scan) {
                                let local_result = timezone.with_ymd_and_hms(
                                    year as i32,
                                    month,
                                    day_of_month,
                                    hour,
                                    minute,
                                    second,
                                );
                                match local_result {
                                    LocalResult::None => {
                                        if self.config.nonexistent_time_behavior
                                            == NonexistentTimeBehavior::Skip
                                            || self.fields.is_hourly_or_more_frequent()
                                        {
                                            continue;
                                        }

                                        let Some(candidate) = next_existent_datetime(
                                            &timezone,
                                            NaiveDateTime::new(
                                                NaiveDate::from_ymd_opt(
                                                    year as i32,
                                                    month,
                                                    day_of_month,
                                                )?,
                                                NaiveTime::from_hms_opt(hour, minute, second)?,
                                            ),
                                        ) else {
                                            continue;
                                        };

                                        if !query.preceeds_reference_datetime(&candidate) {
                                            continue;
                                        }
                                        if enforce_search_interval
                                            && !self.within_search_interval(
                                                datetime,
                                                &candidate,
                                                query.is_reversed(),
                                            )
                                        {
                                            return deferred_candidate;
                                        }
                                        if let Some(deferred) = deferred_candidate.take() {
                                            return Some(
                                                query.preferred_candidate(deferred, candidate),
                                            );
                                        }
                                        return Some(candidate);
                                    }
                                    LocalResult::Single(candidate) => {
                                        if !query.preceeds_reference_datetime(&candidate) {
                                            continue;
                                        }
                                        if enforce_search_interval
                                            && !self.within_search_interval(
                                                datetime,
                                                &candidate,
                                                query.is_reversed(),
                                            )
                                        {
                                            return deferred_candidate;
                                        }
                                        if let Some(deferred) = deferred_candidate.take() {
                                            return Some(
                                                query.preferred_candidate(deferred, candidate),
                                            );
                                        }
                                        return Some(candidate);
                                    }
                                    LocalResult::Ambiguous(earlier, later) => {
                                        let primary = query
                                            .preferred_candidate(earlier.clone(), later.clone());
                                        if query.preceeds_reference_datetime(&primary)
                                            && (!enforce_search_interval
                                                || self.within_search_interval(
                                                    datetime,
                                                    &primary,
                                                    query.is_reversed(),
                                                ))
                                        {
                                            if let Some(deferred) = deferred_candidate.take() {
                                                return Some(
                                                    query.preferred_candidate(deferred, primary),
                                                );
                                            }
                                            return Some(primary);
                                        }

                                        let secondary =
                                            if primary == earlier { later } else { earlier };
                                        if query.preceeds_reference_datetime(&secondary)
                                            && (!enforce_search_interval
                                                || self.within_search_interval(
                                                    datetime,
                                                    &secondary,
                                                    query.is_reversed(),
                                                ))
                                        {
                                            deferred_candidate =
                                                Some(match deferred_candidate.take() {
                                                    Some(existing) => query
                                                        .preferred_candidate(existing, secondary),
                                                    None => secondary,
                                                });
                                        }
                                    }
                                }
                            }
                            query.reset_minute();
                        }
                        query.reset_hour();
                    }
                    query.reset_day_of_month();
                }
                query.reset_month();
            }
        }

        deferred_candidate
    }

    /// Provides an iterator which will return each DateTime that matches the schedule starting with
    /// the current time if applicable.
    pub fn upcoming<Z>(&self, timezone: Z) -> ScheduleIterator<'_, Z>
    where
        Z: TimeZone + 'static,
    {
        self.after(&timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// The same, but with an iterator with a static ownership
    pub fn upcoming_owned<Z: TimeZone + 'static>(&self, timezone: Z) -> OwnedScheduleIterator<Z> {
        self.after_owned(timezone.from_utc_datetime(&Utc::now().naive_utc()))
    }

    /// Like the `upcoming` method, but allows you to specify a start time other than the present.
    pub fn after<Z>(&self, after: &DateTime<Z>) -> ScheduleIterator<'_, Z>
    where
        Z: TimeZone + 'static,
    {
        ScheduleIterator::new(self, after)
    }

    /// The same, but with a static ownership.
    pub fn after_owned<Z: TimeZone + 'static>(
        &self,
        after: DateTime<Z>,
    ) -> OwnedScheduleIterator<Z> {
        OwnedScheduleIterator::new(self.clone(), after)
    }

    pub fn includes<Z>(&self, date_time: DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        let day_of_month = date_time.day() as Ordinal;
        let day_of_week = date_time.weekday().number_from_sunday();
        self.fields.includes_year(date_time.year() as Ordinal)
            && self.fields.months.includes(date_time.month() as Ordinal)
            && self.fields.day_matches(
                date_time.year() as Ordinal,
                date_time.month() as Ordinal,
                day_of_month,
                day_of_week,
                self.config.dow_dom_operand,
            )
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

    /// Returns the parsing configuration associated with this schedule.
    pub fn config(&self) -> ScheduleConfig {
        self.config
    }

    fn within_search_interval<Z>(
        &self,
        reference_datetime: &DateTime<Z>,
        candidate: &DateTime<Z>,
        reversed: bool,
    ) -> bool
    where
        Z: TimeZone,
    {
        let elapsed = if reversed {
            reference_datetime
                .clone()
                .signed_duration_since(candidate.clone())
        } else {
            candidate
                .clone()
                .signed_duration_since(reference_datetime.clone())
        };
        elapsed <= self.config.search_interval
    }

    fn enforces_search_interval(&self) -> bool {
        !self.fields.years_are_unrestricted()
            || self.config.search_interval != ScheduleConfig::default().search_interval
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ScheduleConfigBuilder {
    config: ScheduleConfig,
}

impl ScheduleConfigBuilder {
    pub fn allowed_cron_schedule_parts(mut self, parts: CronScheduleParts) -> Self {
        self.config.cron_schedule_parts = parts;
        self
    }

    pub fn day_of_week_numbering(mut self, numbering: DayOfWeekNumbering) -> Self {
        self.config.day_of_week_numbering = numbering;
        self
    }

    pub fn wraparound_ranges(mut self, wraparound_ranges: bool) -> Self {
        self.config.wraparound_ranges = wraparound_ranges;
        self
    }

    pub fn last_specifiers(mut self, last_specifiers: bool) -> Self {
        self.config.last_specifiers = last_specifiers;
        self
    }

    pub fn nearest_weekday(mut self, nearest_weekday: bool) -> Self {
        self.config.nearest_weekday = nearest_weekday;
        self
    }

    pub fn nth_weekday_of_month(mut self, nth_weekday_of_month: bool) -> Self {
        self.config.nth_weekday_of_month = nth_weekday_of_month;
        self
    }

    pub fn random_fields(mut self, random_fields: bool) -> Self {
        self.config.random_fields = random_fields;
        self
    }

    pub fn dow_dom_operand(mut self, operand: DowDomOperand) -> Self {
        self.config.dow_dom_operand = operand;
        self
    }

    pub fn days_matching(self, operand: DowDomOperand) -> Self {
        self.dow_dom_operand(operand)
    }

    pub fn search_interval(mut self, interval: TimeDelta) -> Self {
        self.config.search_interval = interval;
        self
    }

    pub fn nonexistent_time_behavior(mut self, behavior: NonexistentTimeBehavior) -> Self {
        self.config.nonexistent_time_behavior = behavior;
        self
    }

    pub fn parse(self, expression: &str) -> Result<Schedule, Error> {
        Schedule::from_str_with_config(expression, self.config)
    }

    pub fn config(&self) -> ScheduleConfig {
        self.config
    }
}

fn next_existent_datetime<Z>(timezone: &Z, nonexistent: NaiveDateTime) -> Option<DateTime<Z>>
where
    Z: TimeZone + 'static,
{
    if let Some(candidate) = next_existent_chrono_tz_datetime(timezone, nonexistent) {
        return Some(candidate);
    }

    let before_probe =
        nonexistent.checked_sub_signed(TimeDelta::seconds(NONEXISTENT_OFFSET_PROBE_SECONDS))?;
    let after_probe =
        nonexistent.checked_add_signed(TimeDelta::seconds(NONEXISTENT_OFFSET_PROBE_SECONDS))?;
    let before_offset = timezone
        .offset_from_utc_datetime(&before_probe)
        .fix()
        .local_minus_utc();
    let after_offset = timezone
        .offset_from_utc_datetime(&after_probe)
        .fix()
        .local_minus_utc();

    // A nonexistent local time should come from a forward offset transition. If the generic
    // fallback cannot observe that across its probes, it cannot infer the gap boundary.
    if after_offset <= before_offset {
        return None;
    }

    let mut lower = nonexistent.checked_sub_signed(TimeDelta::seconds(i64::from(after_offset)))?;
    let mut upper = nonexistent.checked_sub_signed(TimeDelta::seconds(i64::from(before_offset)))?;
    let upper_local = timezone.from_utc_datetime(&upper).naive_local();

    if let Some(candidate) = likely_gap_end_datetime(timezone, nonexistent, upper_local) {
        return Some(candidate);
    }

    // Chrono's generic `TimeZone` exposes offsets but not transition tables. For a forward
    // transition, the surrounding offsets bracket the UTC instant where local time jumps over
    // `nonexistent`, so search that small UTC interval for the first existent local time after it.
    while upper.signed_duration_since(lower) > TimeDelta::seconds(1) {
        let midpoint = lower.checked_add_signed(upper.signed_duration_since(lower) / 2)?;
        if timezone.from_utc_datetime(&midpoint).naive_local() > nonexistent {
            upper = midpoint;
        } else {
            lower = midpoint;
        }
    }

    let candidate = timezone.from_utc_datetime(&upper);
    (candidate.naive_local() > nonexistent).then_some(candidate)
}

fn next_existent_chrono_tz_datetime<Z>(
    timezone: &Z,
    nonexistent: NaiveDateTime,
) -> Option<DateTime<Z>>
where
    Z: TimeZone + 'static,
{
    // `chrono_tz::Tz` exposes transition metadata, but stable Rust cannot specialize the generic
    // `TimeZone` path. Use a safe runtime downcast when the concrete timezone is exactly `Tz`.
    let chrono_tz = (timezone as &dyn Any).downcast_ref::<chrono_tz::Tz>()?;
    GapInfo::new(&nonexistent, chrono_tz)?
        .end
        .map(|candidate| candidate.with_timezone(timezone))
}

fn likely_gap_end_datetime<Z>(
    timezone: &Z,
    nonexistent: NaiveDateTime,
    upper_local: NaiveDateTime,
) -> Option<DateTime<Z>>
where
    Z: TimeZone,
{
    for interval_seconds in [60 * 60, 15 * 60] {
        let mut boundary = next_local_boundary(nonexistent, interval_seconds)?;
        while boundary <= upper_local {
            if let Some(candidate) = gap_end_at_boundary(timezone, boundary) {
                return Some(candidate);
            }
            boundary = boundary.checked_add_signed(TimeDelta::seconds(interval_seconds))?;
        }
    }

    None
}

fn next_local_boundary(datetime: NaiveDateTime, interval_seconds: i64) -> Option<NaiveDateTime> {
    let day_start = datetime.date().and_hms_opt(0, 0, 0)?;
    let seconds_from_midnight = i64::from(datetime.time().num_seconds_from_midnight());
    let next_boundary_seconds = ((seconds_from_midnight / interval_seconds) + 1) * interval_seconds;

    day_start.checked_add_signed(TimeDelta::seconds(next_boundary_seconds))
}

fn gap_end_at_boundary<Z>(timezone: &Z, boundary: NaiveDateTime) -> Option<DateTime<Z>>
where
    Z: TimeZone,
{
    let previous_second = boundary.checked_sub_signed(TimeDelta::seconds(1))?;
    if !matches!(
        timezone.from_local_datetime(&previous_second),
        LocalResult::None
    ) {
        return None;
    }

    match timezone.from_local_datetime(&boundary) {
        LocalResult::Single(candidate) => Some(candidate),
        LocalResult::Ambiguous(earlier, later) => {
            Some(if earlier < later { earlier } else { later })
        }
        LocalResult::None => None,
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

    pub(crate) fn years_are_unrestricted(&self) -> bool {
        self.years.is_unrestricted()
    }

    pub(crate) fn includes_year(&self, year: Ordinal) -> bool {
        self.years.contains_ordinal(year)
    }

    pub(crate) fn years_between(&self, start: Ordinal, end: Ordinal) -> YearRangeIter<'_> {
        self.years.ordinals_between(start, end)
    }

    pub(crate) fn months_ordinals(&self) -> &OrdinalSet {
        self.months.ordinals()
    }

    pub(crate) fn days_of_month_ordinals(&self) -> &OrdinalSet {
        self.days_of_month.ordinals()
    }

    pub(crate) fn days_of_month_has_special_specifiers(&self) -> bool {
        self.days_of_month.has_special_specifiers()
    }

    pub(crate) fn days_of_month_ordinals_for_month(
        &self,
        year: Ordinal,
        month: Ordinal,
        last_day: Ordinal,
        range: RangeInclusive<Ordinal>,
    ) -> Vec<Ordinal> {
        self.days_of_month
            .ordinals_for_month(year, month, last_day, range)
    }

    pub(crate) fn hours_ordinals(&self) -> &OrdinalSet {
        self.hours.ordinals()
    }

    pub(crate) fn minutes_ordinals(&self) -> &OrdinalSet {
        self.minutes.ordinals()
    }

    pub(crate) fn seconds_ordinals(&self) -> &OrdinalSet {
        self.seconds.ordinals()
    }

    pub(crate) fn is_hourly_or_more_frequent(&self) -> bool {
        self.hours.is_all()
            || self.minutes.ordinals().len() > 1
            || self.seconds.ordinals().len() > 1
    }

    pub(crate) fn days_of_week_is_all(&self) -> bool {
        self.days_of_week.is_all()
    }

    pub(crate) fn days_of_month_is_all(&self) -> bool {
        self.days_of_month.is_all()
    }

    pub(crate) fn includes_day_of_month(
        &self,
        year: Ordinal,
        month: Ordinal,
        day_of_month: Ordinal,
    ) -> bool {
        self.days_of_month.matches(year, month, day_of_month)
    }

    pub(crate) fn includes_day_of_week(
        &self,
        year: Ordinal,
        month: Ordinal,
        day_of_month: Ordinal,
        day_of_week: Ordinal,
    ) -> bool {
        self.days_of_week
            .matches(year, month, day_of_month, day_of_week)
    }

    pub(crate) fn day_matches(
        &self,
        year: Ordinal,
        month: Ordinal,
        day_of_month: Ordinal,
        day_of_week: Ordinal,
        operand: DowDomOperand,
    ) -> bool {
        let both_restricted = !self.days_of_month_is_all() && !self.days_of_week_is_all();
        let dom_matches = self.includes_day_of_month(year, month, day_of_month);
        if both_restricted && operand == DowDomOperand::Or {
            dom_matches || self.includes_day_of_week(year, month, day_of_month, day_of_week)
        } else {
            dom_matches && self.includes_day_of_week(year, month, day_of_month, day_of_week)
        }
    }
}

pub struct ScheduleIterator<'a, Z>
where
    Z: TimeZone + 'static,
{
    schedule: &'a Schedule,
    previous_datetime: Option<DateTime<Z>>,
}
//TODO: Cutoff datetime?

impl<'a, Z> ScheduleIterator<'a, Z>
where
    Z: TimeZone + 'static,
{
    fn new(schedule: &'a Schedule, starting_datetime: &DateTime<Z>) -> Self {
        ScheduleIterator {
            schedule,
            previous_datetime: Some(starting_datetime.clone()),
        }
    }
}

impl<Z> Iterator for ScheduleIterator<'_, Z>
where
    Z: TimeZone + 'static,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        let previous = self.previous_datetime.take()?;

        let next = self.schedule.next_after(&previous)?;
        self.previous_datetime = Some(next.clone());
        Some(next)
    }
}

impl<Z> DoubleEndedIterator for ScheduleIterator<'_, Z>
where
    Z: TimeZone + 'static,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        let prev = self.schedule.prev_from(&previous)?;
        self.previous_datetime = Some(prev.clone());
        Some(prev)
    }
}

/// A `ScheduleIterator` with a static lifetime.
pub struct OwnedScheduleIterator<Z>
where
    Z: TimeZone + 'static,
{
    schedule: Schedule,
    previous_datetime: Option<DateTime<Z>>,
}

impl<Z> OwnedScheduleIterator<Z>
where
    Z: TimeZone + 'static,
{
    pub fn new(schedule: Schedule, starting_datetime: DateTime<Z>) -> Self {
        Self {
            schedule,
            previous_datetime: Some(starting_datetime),
        }
    }
}

impl<Z> Iterator for OwnedScheduleIterator<Z>
where
    Z: TimeZone + 'static,
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        let previous = self.previous_datetime.take()?;

        let next = self.schedule.next_after(&previous)?;
        self.previous_datetime = Some(next.clone());
        Some(next)
    }
}

impl<Z: TimeZone + 'static> DoubleEndedIterator for OwnedScheduleIterator<Z> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        let prev = self.schedule.prev_from(&previous)?;
        self.previous_datetime = Some(prev.clone());
        Some(prev)
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
            "a valid cron expression",
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
    fn test_chrono_tz_next_existent_fast_path() {
        use chrono::NaiveDate;
        use chrono_tz::America::Los_Angeles;

        let nonexistent = NaiveDate::from_ymd_opt(2022, 3, 13)
            .unwrap()
            .and_hms_opt(2, 30, 0)
            .unwrap();
        let candidate = next_existent_chrono_tz_datetime(&Los_Angeles, nonexistent).unwrap();

        assert_eq!(
            candidate,
            Los_Angeles.with_ymd_and_hms(2022, 3, 13, 3, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_no_panic_on_leap_day_time_after() {
        let dt = chrono::DateTime::parse_from_rfc3339("2024-02-29T10:00:00.000+08:00").unwrap();
        let schedule = Schedule::from_str("0 0 0 * * * 2099").unwrap();
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

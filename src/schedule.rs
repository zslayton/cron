use std::fmt::{Display, Formatter, Result as FmtResult};

use jiff::{tz::TimeZone, RoundMode, ToSpan as _, Unit, Zoned, ZonedRound};
#[cfg(feature = "serde")]
use serde::{
    de::{self, Visitor},
    Deserialize, Serialize, Serializer,
};

use crate::{ordinal::*, time_unit::*};

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

    // Get the set of ordinals for the given unit.
    fn ordinals(&self, unit: Unit) -> Option<&OrdinalSet> {
        Some(match unit {
            Unit::Second => self.fields.seconds.ordinals(),
            Unit::Minute => self.fields.minutes.ordinals(),
            Unit::Hour => self.fields.hours.ordinals(),
            Unit::Day => self.fields.days_of_month.ordinals(),
            Unit::Month => self.fields.months.ordinals(),
            Unit::Year => self.fields.years.ordinals(),
            _ => return None,
        })
    }

    // Get the current value for the corresponding unit in the given timestamp with
    // associated timezone.
    fn current(zoned: &Zoned, unit: Unit) -> Option<u32> {
        Some(match unit {
            Unit::Second => zoned.second() as u32,
            Unit::Minute => zoned.minute() as u32,
            Unit::Hour => zoned.hour() as u32,
            Unit::Day => zoned.day() as u32,
            Unit::Month => zoned.month() as u32,
            Unit::Year => zoned.year() as u32,
            _ => return None,
        })
    }

    fn adjust_next(&self, mut zoned: Zoned, unit: Unit) -> Option<Zoned> {
        let ordinals = self.ordinals(unit)?;
        let current = Self::current(&zoned, unit)?;

        // Determine the next ordinal to use for the given unit and timestamp.
        let interval = ordinals
            .range(current + 1..)
            .next()
            .filter(|next| match unit {
                Unit::Day => **next <= zoned.days_in_month() as u32,
                _ => true,
            })
            .map(|next| (next - current) as i32)?;

        // Calculate the span we have to add to the timestamp.
        let interval = match unit {
            Unit::Second => interval.seconds(),
            Unit::Minute => interval.minutes(),
            Unit::Hour => interval.hours(),
            Unit::Day => interval.days(),
            Unit::Month => interval.months(),
            Unit::Year => interval.years(),
            _ => return None,
        };

        zoned = zoned.checked_add(interval).ok()?;

        Some(zoned)
    }

    fn adjust_prev(&self, mut zoned: Zoned, unit: Unit) -> Option<Zoned> {
        let ordinals = self.ordinals(unit)?;
        let current = Self::current(&zoned, unit)?;

        // Determine the next ordinal to use for the given unit and timestamp.
        let interval = ordinals
            .range(..current)
            .next_back()
            .map(|prev| (current - prev) as i32)?;

        // Calculate the span we have to subtract from the timestamp.
        let interval = match unit {
            Unit::Second => interval.seconds(),
            Unit::Minute => interval.minutes(),
            Unit::Hour => interval.hours(),
            Unit::Day => interval.days(),
            Unit::Month => interval.months(),
            Unit::Year => interval.years(),
            _ => return None,
        };

        zoned = zoned.checked_sub(interval).ok()?;

        Some(zoned)
    }

    fn reset_next(&self, zoned: Zoned, unit: Unit) -> Option<Zoned> {
        // Pick the first available ordinal for the given unit.
        let ordinals = self.ordinals(unit)?;
        let first = *ordinals.first()?;

        // Simply replace the value of the corresponding unit.
        Some(match unit {
            Unit::Second => zoned.with().second(first as i8).build().ok()?,
            Unit::Minute => zoned.with().minute(first as i8).build().ok()?,
            Unit::Hour => zoned.with().hour(first as i8).build().ok()?,
            Unit::Day => zoned.with().day(first as i8).build().ok()?,
            Unit::Month => zoned.with().month(first as i8).build().ok()?,
            Unit::Year => zoned.with().year(first as i16).build().ok()?,
            _ => return None,
        })
    }

    fn reset_prev(&self, zoned: Zoned, unit: Unit) -> Option<Zoned> {
        // Pick the last available ordinal for the given unit. Ensure that if we are
        // working with days, that we do not pick ordinals greater than the
        // number of days in the current month.
        let ordinals = self.ordinals(unit)?;

        let last = *ordinals.iter().rev().find(|next| match unit {
            Unit::Day => **next <= zoned.days_in_month() as u32,
            _ => true,
        })?;

        // Simply replace the value of the corresponding unit.
        Some(match unit {
            Unit::Second => zoned.with().second(last as i8).build().ok()?,
            Unit::Minute => zoned.with().minute(last as i8).build().ok()?,
            Unit::Hour => zoned.with().hour(last as i8).build().ok()?,
            Unit::Day => zoned.with().day(last as i8).build().ok()?,
            Unit::Month => zoned.with().month(last as i8).build().ok()?,
            Unit::Year => zoned.with().year(last as i16).build().ok()?,
            _ => return None,
        })
    }

    fn next_after(&self, after: &Zoned) -> Option<Zoned> {
        let units = [
            Unit::Second,
            Unit::Minute,
            Unit::Hour,
            Unit::Day,
            Unit::Month,
            Unit::Year,
        ];
        let mut candidate = after.clone();

        // First try rounding up the candidate to the nearest second.
        let rounded = candidate
            .round(
                ZonedRound::new()
                    .smallest(Unit::Second)
                    .mode(RoundMode::Ceil),
            )
            .ok()?;

        // If all fields have valid ordinals, return the rounded timestamp.
        if rounded != candidate {
            let mut valid = true;

            for unit in &units {
                let unit = *unit;
                let ordinals = self.ordinals(unit)?;
                let current = Self::current(&rounded, unit)?;

                valid &= ordinals.contains(&current);
            }

            if valid {
                return Some(rounded);
            }
        }

        candidate = rounded;

        loop {
            'outer: for (i, unit) in units.iter().enumerate() {
                // Determine the smallest possible unit for which we can simply pick the next
                // ordinal without wrapping around.
                let Some(new_candidate) = self.adjust_next(candidate.clone(), *unit) else {
                    continue;
                };

                // Check if all larger units have valid ordinals. Otherwise we have to try the
                // next smallest possible unit.
                for unit in &units[i..] {
                    let ordinals = self.ordinals(*unit)?;
                    let current = Self::current(&new_candidate, *unit)?;

                    if !ordinals.contains(&current) {
                        continue 'outer;
                    }
                }

                // At this point we found a suitable candidate for which we have to reset the
                // values corresponding to the units smaller than the unit we found. We simply
                // pick the smallest possible ordinal for each.
                candidate = new_candidate;

                for unit in units[..i].iter().rev() {
                    candidate = self.reset_next(candidate, *unit)?;
                }

                break;
            }

            // Keep going until the weekday is valid for this schedule.
            if !self
                .fields
                .days_of_week
                .ordinals()
                .contains(&(candidate.weekday().to_sunday_one_offset() as u32))
            {
                continue;
            }

            // This is the next possible candidate that adheres to the schedule.
            return Some(candidate);
        }
    }

    fn prev_from(&self, before: &Zoned) -> Option<Zoned> {
        let units = [
            Unit::Second,
            Unit::Minute,
            Unit::Hour,
            Unit::Day,
            Unit::Month,
            Unit::Year,
        ];
        let mut candidate = before.clone();

        // First try rounding up the candidate to the nearest second.
        let rounded = candidate
            .round(
                ZonedRound::new()
                    .smallest(Unit::Second)
                    .mode(RoundMode::Floor),
            )
            .ok()?;

        // If all fields have valid ordinals, return the rounded timestamp.
        if rounded != candidate {
            let mut valid = true;

            for unit in &units {
                let unit = *unit;
                let ordinals = self.ordinals(unit)?;
                let current = Self::current(&rounded, unit)?;

                valid &= ordinals.contains(&current);
            }

            if valid {
                return Some(rounded);
            }
        }

        candidate = rounded;

        loop {
            'outer: for (i, unit) in units.iter().enumerate() {
                // Determine the smallest possible unit for which we can simply pick the next
                // ordinal without wrapping around.
                let Some(new_candidate) = self.adjust_prev(candidate.clone(), *unit) else {
                    continue;
                };

                // Check if all larger units have valid ordinals. Otherwise we have to try the
                // next smallest possible unit.
                for unit in &units[i..] {
                    let ordinals = self.ordinals(*unit)?;
                    let current = Self::current(&new_candidate, *unit)?;

                    if !ordinals.contains(&current) {
                        continue 'outer;
                    }
                }

                // At this point we found a suitable candidate for which we have to reset the
                // values corresponding to the units smaller than the unit we found. We simply
                // pick the greatest possible ordinal for each.
                candidate = new_candidate;

                for unit in units[..i].iter().rev() {
                    candidate = self.reset_prev(candidate, *unit)?;
                }

                break;
            }

            // Keep going until the weekday is valid for this schedule.
            if !self
                .fields
                .days_of_week
                .ordinals()
                .contains(&(candidate.weekday().to_sunday_one_offset() as u32))
            {
                continue;
            }

            // This is the next possible candidate that adheres to the schedule.
            return Some(candidate);
        }
    }

    /// Provides an iterator which will return each [`jiff::Zoned`] that matches
    /// the schedule starting with the current time if applicable.
    pub fn upcoming(&self, timezone: TimeZone) -> ScheduleIterator<'_> {
        let after = Zoned::now().with_time_zone(timezone);
        self.after(&after)
    }

    /// The same, but with an iterator with a static ownership
    pub fn upcoming_owned(&self, timezone: TimeZone) -> OwnedScheduleIterator {
        let after = Zoned::now().with_time_zone(timezone);
        self.after_owned(after)
    }

    /// Like the `upcoming` method, but allows you to specify a start time other
    /// than the present.
    pub fn after(&self, after: &Zoned) -> ScheduleIterator<'_> {
        ScheduleIterator::new(self, after)
    }

    /// The same, but with a static ownership.
    pub fn after_owned(&self, after: Zoned) -> OwnedScheduleIterator {
        OwnedScheduleIterator::new(self.clone(), after)
    }

    pub fn includes(&self, date_time: Zoned) -> bool {
        self.fields.years.includes(date_time.year() as Ordinal)
            && self.fields.months.includes(date_time.month() as Ordinal)
            && self
                .fields
                .days_of_week
                .includes(date_time.weekday().to_sunday_one_offset() as u32)
            && self
                .fields
                .days_of_month
                .includes(date_time.day() as Ordinal)
            && self.fields.hours.includes(date_time.hour() as Ordinal)
            && self.fields.minutes.includes(date_time.minute() as Ordinal)
            && self.fields.seconds.includes(date_time.second() as Ordinal)
    }

    /// Returns a [`TimeUnitSpec`] describing the years included in this
    /// [`Schedule`].
    pub fn years(&self) -> &impl TimeUnitSpec {
        &self.fields.years
    }

    /// Returns a [`TimeUnitSpec`] describing the months of the year included in
    /// this [`Schedule`].
    pub fn months(&self) -> &impl TimeUnitSpec {
        &self.fields.months
    }

    /// Returns a [`TimeUnitSpec`] describing the days of the month included in
    /// this [`Schedule`].
    pub fn days_of_month(&self) -> &impl TimeUnitSpec {
        &self.fields.days_of_month
    }

    /// Returns a [`TimeUnitSpec`] describing the days of the week included in
    /// this [`Schedule`].
    pub fn days_of_week(&self) -> &impl TimeUnitSpec {
        &self.fields.days_of_week
    }

    /// Returns a [`TimeUnitSpec`] describing the hours of the day included in
    /// this [`Schedule`].
    pub fn hours(&self) -> &impl TimeUnitSpec {
        &self.fields.hours
    }

    /// Returns a [`TimeUnitSpec`] describing the minutes of the hour included
    /// in this [`Schedule`].
    pub fn minutes(&self) -> &impl TimeUnitSpec {
        &self.fields.minutes
    }

    /// Returns a [`TimeUnitSpec`] describing the seconds of the minute included
    /// in this [`Schedule`].
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
        self.timeunitspec_eq(other)
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

pub struct ScheduleIterator<'a> {
    schedule: &'a Schedule,
    previous_datetime: Option<Zoned>,
}
//TODO: Cutoff datetime?

impl<'a> ScheduleIterator<'a> {
    fn new(schedule: &'a Schedule, starting_datetime: &Zoned) -> Self {
        ScheduleIterator {
            schedule,
            previous_datetime: Some(starting_datetime.clone()),
        }
    }
}

impl Iterator for ScheduleIterator<'_> {
    type Item = Zoned;

    fn next(&mut self) -> Option<Zoned> {
        let previous = self.previous_datetime.take()?;

        if let Some(next) = self.schedule.next_after(&previous) {
            self.previous_datetime = Some(next.clone());
            Some(next)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for ScheduleIterator<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        if let Some(prev) = self.schedule.prev_from(&previous) {
            self.previous_datetime = Some(prev.clone());
            Some(prev)
        } else {
            None
        }
    }
}

/// A `ScheduleIterator` with a static lifetime.
pub struct OwnedScheduleIterator {
    schedule: Schedule,
    previous_datetime: Option<Zoned>,
}

impl OwnedScheduleIterator {
    pub fn new(schedule: Schedule, starting_datetime: Zoned) -> Self {
        Self {
            schedule,
            previous_datetime: Some(starting_datetime),
        }
    }
}

impl Iterator for OwnedScheduleIterator {
    type Item = Zoned;

    fn next(&mut self) -> Option<Zoned> {
        let previous = self.previous_datetime.take()?;

        if let Some(next) = self.schedule.next_after(&previous) {
            self.previous_datetime = Some(next.clone());
            Some(next)
        } else {
            None
        }
    }
}

impl DoubleEndedIterator for OwnedScheduleIterator {
    fn next_back(&mut self) -> Option<Self::Item> {
        let previous = self.previous_datetime.take()?;

        if let Some(prev) = self.schedule.prev_from(&previous) {
            self.previous_datetime = Some(prev.clone());
            Some(prev)
        } else {
            None
        }
    }
}

#[cfg(feature = "serde")]
struct ScheduleVisitor;

#[cfg(feature = "serde")]
impl Visitor<'_> for ScheduleVisitor {
    type Value = Schedule;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> FmtResult {
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
    use std::str::FromStr;

    use jiff::{civil::DateTime, SignedDuration, Span};
    #[cfg(feature = "serde")]
    use serde_test::{assert_tokens, Token};

    use super::*;

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
            "definitively an invalid value for a cron schedule!\n^\nThe 'Seconds' field does not \
             support using names. 'definitively' specified.",
        );
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ser_de_schedule_shorthand() {
        use jiff::civil::date;

        let serialized = postcard::to_stdvec(&Schedule::try_from("@hourly").expect("valid format"))
            .expect("serializable schedule");

        let schedule: Schedule =
            postcard::from_bytes(&serialized).expect("deserializable schedule");

        let starting_date = date(2017, 2, 25)
            .at(22, 29, 36, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap();
        assert!([
            date(2017, 2, 25)
                .at(23, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2017, 2, 26)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2017, 2, 26)
                .at(1, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
        ]
        .into_iter()
        .eq(schedule.after(&starting_date).take(3)));
    }

    #[cfg(feature = "serde")]
    #[test]
    fn test_ser_de_schedule_period_values_range() {
        use jiff::civil::date;

        let serialized =
            postcard::to_stdvec(&Schedule::try_from("0 0 0 1-31/10 * ?").expect("valid format"))
                .expect("serializable schedule");

        let schedule: Schedule =
            postcard::from_bytes(&serialized).expect("deserializable schedule");

        let starting_date = date(2020, 1, 1)
            .at(0, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap();
        assert!([
            date(2020, 1, 11)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2020, 1, 21)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2020, 1, 31)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2020, 2, 1)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2020, 2, 11)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2020, 2, 21)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
            date(2020, 3, 1)
                .at(0, 0, 0, 0)
                .to_zoned(TimeZone::UTC)
                .unwrap(),
        ]
        .into_iter()
        .eq(schedule.after(&starting_date).take(7)));
    }

    #[test]
    fn test_next_and_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();

        let utc_now = Zoned::now().with_time_zone(TimeZone::UTC);
        let next = schedule.next_after(&utc_now);
        println!("NEXT AFTER for {} {:?}", expression, &next);
        assert!(next.is_some());

        let next2 = schedule.next_after(next.as_ref().unwrap());
        println!("NEXT2 AFTER for {expression} {next2:?}");
        assert!(next2.is_some());

        let prev = schedule.prev_from(next2.as_ref().unwrap());
        println!("PREV FROM for {expression} {prev:?}");
        assert!(prev.is_some());
        assert_eq!(prev, next);

        let prev2 = schedule.prev_from(
            &next2
                .as_ref()
                .map(|next2| next2.saturating_add(SignedDuration::from_millis(100)))
                .unwrap(),
        );
        println!("PREV2 FROM for {expression} {prev2:?}");
        assert!(prev2.is_some());
        assert_eq!(prev2, next2);
    }

    #[test]
    fn test_next_after_past_date_next_year() {
        // Schedule after 2021-10-27
        let starting_point = jiff::civil::date(2021, 10, 27)
            .at(0, 0, 0, 0)
            .to_zoned(TimeZone::UTC)
            .unwrap();

        // Triggers on 2022-06-01. Note that the month and day are smaller than
        // the month and day in `starting_point`.
        let expression = "0 5 17 1 6 ? 2022".to_string();
        let schedule = Schedule::from_str(&expression).unwrap();
        let next = schedule.next_after(&starting_point);
        println!("NEXT AFTER for {expression} {next:?}");
        assert!(next.is_some());
    }

    #[test]
    fn test_prev_from() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let utc_now = Zoned::now().with_time_zone(TimeZone::UTC);
        let prev = schedule.prev_from(&utc_now);
        println!("PREV FROM for {expression} {prev:?}");
        assert!(prev.is_some());
    }

    #[test]
    fn test_next_after() {
        let expression = "0 5,13,40-42 17 1 Jan *";
        let schedule = Schedule::from_str(expression).unwrap();
        let utc_now = Zoned::now().with_time_zone(TimeZone::UTC);
        let next = schedule.next_after(&utc_now);
        println!("NEXT AFTER for {expression} {next:?}");
        assert!(next.is_some());
    }

    #[test]
    fn test_upcoming_utc() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming(TimeZone::UTC);
        let next1 = upcoming.next();
        assert!(next1.is_some());
        let next2 = upcoming.next();
        assert!(next2.is_some());
        let next3 = upcoming.next();
        assert!(next3.is_some());
        println!("Upcoming 1 for {expression} {next1:?}");
        println!("Upcoming 2 for {expression} {next2:?}");
        println!("Upcoming 3 for {expression} {next3:?}");
    }

    #[test]
    fn test_upcoming_utc_owned() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming_owned(TimeZone::UTC);
        let next1 = upcoming.next();
        assert!(next1.is_some());
        let next2 = upcoming.next();
        assert!(next2.is_some());
        let next3 = upcoming.next();
        assert!(next3.is_some());
        println!("Upcoming 1 for {expression} {next1:?}");
        println!("Upcoming 2 for {expression} {next2:?}");
        println!("Upcoming 3 for {expression} {next3:?}");
    }

    #[test]
    fn test_upcoming_rev_utc() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming(TimeZone::UTC).rev();
        let prev1 = upcoming.next();
        assert!(prev1.is_some());
        let prev2 = upcoming.next();
        assert!(prev2.is_some());
        let prev3 = upcoming.next();
        assert!(prev3.is_some());
        println!("Prev Upcoming 1 for {expression} {prev1:?}");
        println!("Prev Upcoming 2 for {expression} {prev2:?}");
        println!("Prev Upcoming 3 for {expression} {prev3:?}");
    }

    #[test]
    fn test_upcoming_rev_utc_owned() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming_owned(TimeZone::UTC).rev();
        let prev1 = upcoming.next();
        assert!(prev1.is_some());
        let prev2 = upcoming.next();
        assert!(prev2.is_some());
        let prev3 = upcoming.next();
        assert!(prev3.is_some());
        println!("Prev Upcoming 1 for {expression} {prev1:?}");
        println!("Prev Upcoming 2 for {expression} {prev2:?}");
        println!("Prev Upcoming 3 for {expression} {prev3:?}");
    }

    #[test]
    fn test_upcoming_local() {
        let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
        let schedule = Schedule::from_str(expression).unwrap();
        let mut upcoming = schedule.upcoming(TimeZone::system());
        let next1 = upcoming.next();
        assert!(next1.is_some());
        let next2 = upcoming.next();
        assert!(next2.is_some());
        let next3 = upcoming.next();
        assert!(next3.is_some());
        println!("Upcoming 1 for {expression} {next1:?}");
        println!("Upcoming 2 for {expression} {next2:?}");
        println!("Upcoming 3 for {expression} {next3:?}");
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
        write!(result, "{schedule}").unwrap();
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
        let schedule_tz: TimeZone = TimeZone::get("Europe/London").unwrap();
        let dt = DateTime::new(2019, 10, 27, 0, 3, 29, 0)
            .unwrap()
            .to_zoned(schedule_tz)
            .unwrap()
            .checked_add(Span::new().hours(1))
            .unwrap(); // puts it in the middle of the DST transition

        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let next = schedule.after(&dt).next().unwrap();
        assert!(next > dt); // test is ensuring line above does not panic
    }

    #[test]
    fn test_no_panic_on_nonexistent_time_before() {
        let schedule_tz: TimeZone = TimeZone::get("Europe/London").unwrap();
        let dt = DateTime::new(2019, 10, 27, 0, 3, 29, 0)
            .unwrap()
            .to_zoned(schedule_tz)
            .unwrap()
            .checked_add(Span::new().hours(1))
            .unwrap(); // puts it in the middle of the DST transition

        let schedule = Schedule::from_str("* * * * * Sat,Sun *").unwrap();
        let prev = schedule.after(&dt).next_back().unwrap();
        assert!(prev < dt); // test is ensuring line above does not panic
    }

    #[test]
    fn test_no_panic_on_leap_day_time_after() {
        let dt = "2024-02-29T10:00:00.000+08:00[Asia/Singapore]" // N.B. TZ inferred from original
            .parse()
            .unwrap();
        let schedule = Schedule::from_str("0 0 0 * * * 2100").unwrap();
        let next = schedule.after(&dt).next().unwrap();
        assert!(next > dt); // test is ensuring line above does not panic
    }

    #[test]
    fn test_time_unit_spec_equality() {
        // Every week
        let schedule_1 = Schedule::from_str("@weekly").unwrap();
        let schedule_2 = Schedule::from_str("0 0 0 * * 1 *").unwrap();

        // Every day
        let schedule_3 = Schedule::from_str("0 0 0 * * 1-7 *").unwrap();
        let schedule_4 = Schedule::from_str("0 0 0 * * * *").unwrap();
        let schedule_5 = Schedule::from_str("0  0  0  *  *  *  *").unwrap();

        // Every second of every day's first minute
        let schedule_6 = Schedule::from_str("* 0 0 * * * *").unwrap();

        // Schedules defined as "@weekly" and "0 0 0 * * 1 *"
        // yield the same events and are therefore considered equal.
        assert_eq!(schedule_1, schedule_2);
        assert!(schedule_1.timeunitspec_eq(&schedule_2));

        // Schedules defined as "0 0 0 * * 1-7 *" and "0 0 0 * * * *"
        // yield the same events and are therefore considered equal.
        assert_eq!(schedule_3, schedule_4);
        assert!(schedule_3.timeunitspec_eq(&schedule_4));

        // Whitespace in the source string is irrelevant to equality.
        assert!(schedule_4.timeunitspec_eq(&schedule_5));

        // But schedules yielding different events are not equal.
        assert_ne!(schedule_4, schedule_6);
    }

    #[test]
    fn test_dst_ambiguous_time_after() {
        let schedule_tz = TimeZone::get("America/Chicago").unwrap();
        let dt = DateTime::new(2022, 11, 5, 23, 30, 0, 0)
            .unwrap()
            .to_zoned(schedule_tz)
            .unwrap();
        let schedule = Schedule::from_str("0 0 * * * * *").unwrap();
        let times = schedule
            .after(&dt)
            .map(|x| x.to_string())
            .take(5)
            .collect::<Vec<_>>();
        let expected_times = [
            "2022-11-06T00:00:00-05:00[America/Chicago]".to_string(),
            "2022-11-06T01:00:00-05:00[America/Chicago]".to_string(),
            "2022-11-06T01:00:00-06:00[America/Chicago]".to_string(), // 1 AM happens again
            "2022-11-06T02:00:00-06:00[America/Chicago]".to_string(),
            "2022-11-06T03:00:00-06:00[America/Chicago]".to_string(),
        ];

        assert_eq!(times.as_slice(), expected_times.as_slice());
    }

    #[test]
    fn test_dst_ambiguous_time_before() {
        let schedule_tz = TimeZone::get("America/Chicago").unwrap();
        let dt = DateTime::new(2022, 11, 6, 3, 30, 0, 0)
            .unwrap()
            .to_zoned(schedule_tz)
            .unwrap();
        let schedule = Schedule::from_str("0 0 * * * * *").unwrap();
        let times = schedule
            .after(&dt)
            .map(|x| x.to_string())
            .rev()
            .take(5)
            .collect::<Vec<_>>();
        let expected_times = [
            "2022-11-06T03:00:00-06:00[America/Chicago]".to_string(),
            "2022-11-06T02:00:00-06:00[America/Chicago]".to_string(),
            "2022-11-06T01:00:00-06:00[America/Chicago]".to_string(),
            "2022-11-06T01:00:00-05:00[America/Chicago]".to_string(), // 1 AM happens again
            "2022-11-06T00:00:00-05:00[America/Chicago]".to_string(),
        ];

        assert_eq!(times.as_slice(), expected_times.as_slice());
    }
}

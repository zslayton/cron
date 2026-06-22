use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::{days_in_month, TimeUnitField};
use chrono::{Datelike, NaiveDate, Weekday};
use std::borrow::Cow;
use std::ops::RangeInclusive;

#[derive(Clone, Debug, Eq)]
pub struct DaysOfMonth {
    ordinals: OrdinalSet,
    last_day_of_month: bool,
    nearest_weekdays: OrdinalSet,
}

impl TimeUnitField for DaysOfMonth {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfMonth {
            ordinals: ordinal_set.unwrap_or_else(Self::supported_ordinals),
            last_day_of_month: false,
            nearest_weekdays: OrdinalSet::empty(Self::inclusive_min(), Self::inclusive_max()),
        }
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Days of Month")
    }
    fn inclusive_min() -> Ordinal {
        1
    }
    fn inclusive_max() -> Ordinal {
        31
    }
    fn ordinal_from_name(name: &str) -> Result<Ordinal, Error> {
        if name.eq_ignore_ascii_case("l") {
            return Ok(Self::inclusive_max());
        }
        Err(ErrorKind::Expression(format!(
            "The '{}' field does not support using names. '{}' specified.",
            Self::name(),
            name
        ))
        .into())
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.ordinals
    }
}

impl PartialEq for DaysOfMonth {
    fn eq(&self, other: &DaysOfMonth) -> bool {
        self.ordinals() == other.ordinals()
            && self.last_day_of_month == other.last_day_of_month
            && self.nearest_weekdays == other.nearest_weekdays
    }
}

impl DaysOfMonth {
    pub(crate) fn from_parts(
        ordinals: OrdinalSet,
        last_day_of_month: bool,
        nearest_weekdays: OrdinalSet,
    ) -> Self {
        Self {
            ordinals,
            last_day_of_month,
            nearest_weekdays,
        }
    }

    pub(crate) fn has_special_specifiers(&self) -> bool {
        self.last_day_of_month || !self.nearest_weekdays.is_empty()
    }

    pub(crate) fn is_all(&self) -> bool {
        !self.has_special_specifiers() && self.ordinals().is_all()
    }

    pub(crate) fn ordinals_for_month(
        &self,
        year: Ordinal,
        month: Ordinal,
        last_day: Ordinal,
        range: RangeInclusive<Ordinal>,
    ) -> Vec<Ordinal> {
        let start = *range.start();
        let end = (*range.end()).min(last_day);
        if start > end {
            return Vec::new();
        }

        let mut days = Vec::with_capacity(
            self.ordinals().len().min((end - start + 1) as usize)
                + usize::from(self.last_day_of_month)
                + self.nearest_weekdays.len(),
        );
        days.extend(self.ordinals().range(start..=end));

        if self.last_day_of_month && (start..=end).contains(&last_day) {
            days.push(last_day);
        }

        for nearest_weekday in &self.nearest_weekdays {
            let day = nearest_weekday_for_month(year, month, nearest_weekday, last_day);
            if (start..=end).contains(&day) {
                days.push(day);
            }
        }

        days.sort_unstable();
        days.dedup();
        days
    }

    pub(crate) fn matches(&self, year: Ordinal, month: Ordinal, day: Ordinal) -> bool {
        if !self.has_special_specifiers() {
            return self.ordinals().contains(&day);
        }

        let last_day = days_in_month(month, year);
        if day > last_day {
            return false;
        }

        // `L` and `W` are month-relative, so scan predicates resolve them
        // against the candidate date's month when DOM/DOW OR semantics apply.
        self.ordinals().contains(&day)
            || (self.last_day_of_month && day == last_day)
            || self.nearest_weekdays.iter().any(|nearest_weekday| {
                day == nearest_weekday_for_month(year, month, nearest_weekday, last_day)
            })
    }
}

fn nearest_weekday_for_month(
    year: Ordinal,
    month: Ordinal,
    day: Ordinal,
    last_day: Ordinal,
) -> Ordinal {
    // Croniter clamps out-of-range `nW` requests before moving to a weekday.
    let day = day.min(last_day);
    let weekday = NaiveDate::from_ymd_opt(year as i32, month, day)
        .expect("day of month must be valid")
        .weekday();

    match weekday {
        Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri => day,
        Weekday::Sat if day == 1 => day + 2,
        Weekday::Sat => day - 1,
        Weekday::Sun if day == last_day => day - 2,
        Weekday::Sun => day + 1,
    }
}

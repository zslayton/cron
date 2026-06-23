use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::{days_in_month, TimeUnitField};
use phf::phf_map;
use std::borrow::Cow;
use std::collections::BTreeMap;

static DAY_OF_WEEK_MAP: phf::Map<&'static str, Ordinal> = phf_map! {
    "sun" => 1,
    "sunday" => 1,
    "mon" => 2,
    "monday" => 2,
    "tue" => 3,
    "tues" => 3,
    "tuesday" => 3,
    "wed" => 4,
    "wednesday" => 4,
    "thu" => 5,
    "thurs" => 5,
    "thursday" => 5,
    "fri" => 6,
    "friday" => 6,
    "sat" => 7,
    "saturday" => 7,
};

#[derive(Clone, Debug, Eq)]
pub struct DaysOfWeek {
    ordinals: OrdinalSet,
    last_weekdays_of_month: OrdinalSet,
    nth_weekdays_of_month: BTreeMap<Ordinal, OrdinalSet>,
}

impl TimeUnitField for DaysOfWeek {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfWeek {
            ordinals: ordinal_set.unwrap_or_else(Self::supported_ordinals),
            last_weekdays_of_month: OrdinalSet::empty(Self::inclusive_min(), Self::inclusive_max()),
            nth_weekdays_of_month: BTreeMap::new(),
        }
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Days of Week")
    }
    fn inclusive_min() -> Ordinal {
        1
    }
    fn inclusive_max() -> Ordinal {
        7
    }
    fn ordinal_from_name(name: &str) -> Result<Ordinal, Error> {
        DAY_OF_WEEK_MAP
            .get(name.to_lowercase().as_ref())
            .copied()
            .ok_or_else(|| {
                ErrorKind::Expression(format!("'{}' is not a valid day of the week.", name)).into()
            })
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.ordinals
    }
}

impl PartialEq for DaysOfWeek {
    fn eq(&self, other: &DaysOfWeek) -> bool {
        self.ordinals() == other.ordinals()
            && self.last_weekdays_of_month == other.last_weekdays_of_month
            && self.nth_weekdays_of_month == other.nth_weekdays_of_month
    }
}

impl DaysOfWeek {
    pub(crate) fn from_parts(
        ordinals: OrdinalSet,
        last_weekdays_of_month: OrdinalSet,
        nth_weekdays_of_month: BTreeMap<Ordinal, OrdinalSet>,
    ) -> Self {
        Self {
            ordinals,
            last_weekdays_of_month,
            nth_weekdays_of_month,
        }
    }

    pub(crate) fn has_special_specifiers(&self) -> bool {
        !self.last_weekdays_of_month.is_empty() || !self.nth_weekdays_of_month.is_empty()
    }

    pub(crate) fn is_all(&self) -> bool {
        !self.has_special_specifiers() && self.ordinals().is_all()
    }

    pub(crate) fn matches(
        &self,
        year: Ordinal,
        month: Ordinal,
        day: Ordinal,
        day_of_week: Ordinal,
    ) -> bool {
        if self.ordinals().contains(&day_of_week) {
            return true;
        }
        if !self.has_special_specifiers() {
            return false;
        }

        let occurrence = ((day - 1) / 7) + 1;
        self.last_weekdays_of_month.contains(&day_of_week) && day > days_in_month(month, year) - 7
            || self
                .nth_weekdays_of_month
                .get(&day_of_week)
                .map_or(false, |occurrences| occurrences.contains(&occurrence))
    }
}

use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::{days_in_month, TimeUnitField};
use once_cell::sync::Lazy;
use phf::phf_map;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

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

static ALL: Lazy<OrdinalSet> = Lazy::new(DaysOfWeek::supported_ordinals);

#[derive(Clone, Debug, Eq)]
pub struct DaysOfWeek {
    ordinals: Option<OrdinalSet>,
    last_weekdays_of_month: OrdinalSet,
    nth_weekdays_of_month: BTreeMap<Ordinal, BTreeSet<Ordinal>>,
}

impl TimeUnitField for DaysOfWeek {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfWeek {
            ordinals: ordinal_set,
            last_weekdays_of_month: OrdinalSet::new(),
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
        match &self.ordinals {
            Some(ordinal_set) => ordinal_set,
            None => &ALL,
        }
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
        ordinals: Option<OrdinalSet>,
        last_weekdays_of_month: OrdinalSet,
        nth_weekdays_of_month: BTreeMap<Ordinal, BTreeSet<Ordinal>>,
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
        !self.has_special_specifiers()
            && self.ordinals().len() == (Self::inclusive_max() - Self::inclusive_min() + 1) as usize
    }

    pub(crate) fn matches(
        &self,
        year: Ordinal,
        month: Ordinal,
        day: Ordinal,
        day_of_week: Ordinal,
    ) -> bool {
        self.ordinals().contains(&day_of_week)
            || self.matches_last_weekday(year, month, day, day_of_week)
            || self.matches_nth_weekday(day, day_of_week)
    }

    fn matches_last_weekday(
        &self,
        year: Ordinal,
        month: Ordinal,
        day: Ordinal,
        day_of_week: Ordinal,
    ) -> bool {
        self.last_weekdays_of_month.contains(&day_of_week) && day > days_in_month(month, year) - 7
    }

    fn matches_nth_weekday(&self, day: Ordinal, day_of_week: Ordinal) -> bool {
        let Some(occurrences) = self.nth_weekdays_of_month.get(&day_of_week) else {
            return false;
        };
        let occurrence = ((day - 1) / 7) + 1;
        occurrences.contains(&occurrence)
    }
}

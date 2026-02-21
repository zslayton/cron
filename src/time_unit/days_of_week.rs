use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use once_cell::sync::Lazy;
use phf::phf_map;
use std::borrow::Cow;

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
}

impl TimeUnitField for DaysOfWeek {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfWeek {
            ordinals: ordinal_set,
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
            .get(name.to_lowercase().as_str())
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
    }
}

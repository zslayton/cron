use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use phf::phf_map;

static ALL: Lazy<OrdinalSet> = Lazy::new(DaysOfWeek::supported_ordinals);

#[derive(Clone, Debug, Eq)]
pub struct DaysOfWeek {
    ordinals: Option<OrdinalSet>,
}

#[cfg(not(feature = "vixie"))]
static DAY_NAME_TO_ORDINAL: phf::Map<&'static str, u32> = phf_map! {
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

#[cfg(feature = "vixie")]
static DAY_NAME_TO_ORDINAL: phf::Map<&'static str, u32> = phf_map! {
    "sun" => 0,
    "sunday" => 0,
    "mon" => 1,
    "monday" => 1,
    "tue" => 2,
    "tues" => 2,
    "tuesday" => 2,
    "wed" => 3,
    "wednesday" => 3,
    "thu" => 4,
    "thurs" => 4,
    "thursday" => 4,
    "fri" => 5,
    "friday" => 5,
    "sat" => 6,
    "saturday" => 6,
};

impl TimeUnitField for DaysOfWeek {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfWeek {
            ordinals: ordinal_set,
        }
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Days of Week")
    }

    #[cfg(not(feature = "vixie"))]
    fn inclusive_min() -> Ordinal {
        1
    }

    #[cfg(not(feature = "vixie"))]
    fn inclusive_max() -> Ordinal {
        7
    }

    #[cfg(feature = "vixie")]
    fn inclusive_min() -> Ordinal {
        0
    }

    #[cfg(feature = "vixie")]
    fn inclusive_max() -> Ordinal {
        6
    }

    fn ordinal_from_name(name: &str) -> Result<Ordinal, Error> {
        match DAY_NAME_TO_ORDINAL.get(name.to_lowercase().as_str()) {
            Some(&ordinal) => Ok(ordinal),
            _ => {
                return Err(ErrorKind::Expression(format!(
                    "'{}' is not a valid day of the week.",
                    name
                ))
                .into())
            }
        }
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

use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;
use phf::{Map, phf_map};

#[derive(Clone, Debug)]
pub struct DaysOfWeek(OrdinalSet);

impl TimeUnitField for DaysOfWeek {
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        DaysOfWeek(ordinal_set)
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
        static WEEKDAY_MAP : Map<&'static str, Ordinal> = phf_map! {
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

        WEEKDAY_MAP.get::<str>(name.to_lowercase().as_ref())
        .copied()
        .ok_or(Error::from( ErrorKind::Expression(
            format!( 
                "'{}' is not a valid day of the week.",
                name
            )
        )))
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.0
    }
}

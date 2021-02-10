use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;
use lazy_static::lazy_static;

lazy_static!{
    static ref ALL: OrdinalSet = DaysOfWeek::supported_ordinals();
}

#[derive(Clone, Debug)]
pub struct DaysOfWeek{
    ordinals: Option<OrdinalSet>
}

impl TimeUnitField for DaysOfWeek {
    fn from_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfWeek{
            ordinals: ordinal_set
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
        //TODO: Use phf crate
        let ordinal = match name.to_lowercase().as_ref() {
            "sun" | "sunday" => 1,
            "mon" | "monday" => 2,
            "tue" | "tues" | "tuesday" => 3,
            "wed" | "wednesday" => 4,
            "thu" | "thurs" | "thursday" => 5,
            "fri" | "friday" => 6,
            "sat" | "saturday" => 7,
            _ => {
                return Err(ErrorKind::Expression(format!(
                    "'{}' is not a valid day of the week.",
                    name
                ))
                .into())
            }
        };
        Ok(ordinal)
    }
    fn ordinals(&self) -> &OrdinalSet {
        match &self.ordinals {
            Some(ordinal_set) => &ordinal_set,
            None => &ALL
        }
    }
    fn is_specified(&self) -> bool {
        self.ordinals.is_some()
    }
}

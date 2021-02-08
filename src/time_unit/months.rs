use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;
use phf::{Map, phf_map};

#[derive(Clone, Debug)]
pub struct Months(OrdinalSet);

impl TimeUnitField for Months {
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        Months(ordinal_set)
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Months")
    }
    fn inclusive_min() -> Ordinal {
        1
    }
    fn inclusive_max() -> Ordinal {
        12
    }
    fn ordinal_from_name(name: &str) -> Result<Ordinal, Error> {
        static MONTH_MAP : Map<&'static str, Ordinal> = phf_map! {
            "jan" => 1,
            "january" => 1,
            "feb" => 2,
            "february" => 2,
            "mar" => 3,
            "march" => 3,
            "apr" => 4,
            "april" => 4,
            "may" => 5,
            "jun" => 6,
            "june" => 6,
            "jul" => 7,
            "july" => 7,
            "aug" => 8,
            "august" => 8,
            "sep" => 9,
            "september" => 9,
            "oct" => 10,
            "october" => 10,
            "nov" => 11,
            "november" => 11,
            "dec" => 12,
            "december" => 12,
        };

        MONTH_MAP.get::<str>(name.to_lowercase().as_ref())
        .copied()
        .ok_or(Error::from( ErrorKind::Expression(
            format!( 
                "'{}' is not a valid day of the month name.",
                name
            )
        )))
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.0
    }
}

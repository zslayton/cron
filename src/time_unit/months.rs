use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use once_cell::sync::Lazy;
use phf::phf_map;
use std::borrow::Cow;

static ALL: Lazy<OrdinalSet> = Lazy::new(Months::supported_ordinals);

static MONTH_NAME_TO_ORDINAL: phf::Map<&'static str, u32> = phf_map! {
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

#[derive(Clone, Debug, Eq)]
pub struct Months {
    ordinals: Option<OrdinalSet>,
}

impl TimeUnitField for Months {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        Months {
            ordinals: ordinal_set,
        }
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
        match MONTH_NAME_TO_ORDINAL.get(name.to_lowercase().as_ref()) {
            Some(&ordinal) => Ok(ordinal),
            _ => {
                return Err(
                    ErrorKind::Expression(format!("'{}' is not a valid month name.", name)).into(),
                )
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

impl PartialEq for Months {
    fn eq(&self, other: &Months) -> bool {
        self.ordinals() == other.ordinals()
    }
}

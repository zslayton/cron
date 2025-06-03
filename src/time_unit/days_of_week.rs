use crate::error::*;
use crate::ordinal::{
    Ordinal, OrdinalSet, IS_1ST_OCCURRENCE, IS_2ND_OCCURRENCE, IS_3RD_OCCURRENCE,
    IS_4TH_OCCURRENCE, IS_5TH_OCCURRENCE, IS_LAST_OCCURRENCE, IS_NTH_OCCURRENCE,
};
use crate::specifier::{RootSpecifier, SingleSpecifier, Specifier};
use crate::time_unit::TimeUnitField;
use chrono::{DateTime, Datelike, TimeZone};
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::iter;

use super::days_in_month;

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
            Some(ordinal_set) => ordinal_set,
            None => &ALL,
        }
    }

    fn validate_ordinal(ordinal: Ordinal) -> Result<Ordinal, Error> {
        //println!("validate_ordinal for {} => {}", Self::name(), ordinal);
        match ordinal & !IS_NTH_OCCURRENCE & !IS_LAST_OCCURRENCE {
            i if i > 5 && (ordinal & IS_NTH_OCCURRENCE != 0) => {
                Err(ErrorKind::Expression(format!(
                    "Occurrence of a weekday must be between 1 and 5 inclusive. ('{}' specified.)",
                    i
                ))
                .into())
            }
            i if i < Self::inclusive_min() => Err(ErrorKind::Expression(format!(
                "{} must be greater than or equal to {}. ('{}' \
                 specified.)",
                Self::name(),
                Self::inclusive_min(),
                i
            ))
            .into()),
            i if i > Self::inclusive_max() => Err(ErrorKind::Expression(format!(
                "{} must be less than {}. ('{}' specified.)",
                Self::name(),
                Self::inclusive_max(),
                i
            ))
            .into()),
            _ => Ok(ordinal),
        }
    }

    fn ordinals_from_root_specifier(root_specifier: &RootSpecifier) -> Result<OrdinalSet, Error> {
        let ordinals = match root_specifier {
            RootSpecifier::Specifier(specifier) => Self::ordinals_from_specifier(specifier)?,
            RootSpecifier::Period(_, 0) => Err(ErrorKind::Expression(
                "range step cannot be zero".to_string(),
            ))?,
            RootSpecifier::Period(start, step) => {
                if *step < 1 || *step > Self::inclusive_max() {
                    return Err(ErrorKind::Expression(format!(
                        "{} must be between 1 and {}. ('{}' specified.)",
                        Self::name(),
                        Self::inclusive_max(),
                        step,
                    ))
                    .into());
                }

                let base_set = match start {
                    // A point prior to a period implies a range whose start is the specified
                    // point and terminating inclusively with the inclusive max
                    Specifier::Point(start) => {
                        let start = Self::validate_ordinal(*start)?;
                        (start..=Self::inclusive_max()).collect()
                    }
                    specifier => Self::ordinals_from_specifier(specifier)?,
                };
                base_set.into_iter().step_by(*step as usize).collect()
            }
            RootSpecifier::NamedPoint(ref name) => ([Self::ordinal_from_name(name)?])
                .iter()
                .cloned()
                .collect::<OrdinalSet>(),

            RootSpecifier::LastPoint(single_specifier) => {
                if let SingleSpecifier::Point(0) = single_specifier {
                    OrdinalSet::from_iter(iter::once(Self::inclusive_max()))
                } else {
                    let ordinal = match single_specifier {
                        SingleSpecifier::Point(ordinal) => Self::validate_ordinal(*ordinal)?,
                        SingleSpecifier::NamedPoint(name) => Self::ordinal_from_name(name)?,
                    };
                    OrdinalSet::from_iter(iter::once(ordinal | IS_LAST_OCCURRENCE))
                }
            }
            RootSpecifier::NthOfMonth(single_specifier, ordinal) => {
                let day_of_month_ordinal = match single_specifier {
                    SingleSpecifier::Point(ordinal) => Self::validate_ordinal(*ordinal)?,
                    SingleSpecifier::NamedPoint(name) => Self::ordinal_from_name(name)?,
                };

                let occurrence = match ordinal {
                    1 => IS_1ST_OCCURRENCE,
                    2 => IS_2ND_OCCURRENCE,
                    3 => IS_3RD_OCCURRENCE,
                    4 => IS_4TH_OCCURRENCE,
                    5 => IS_5TH_OCCURRENCE,
                    ordinal => return Err(ErrorKind::Expression(format!(
                        "Occurrence of a weekday must be between 1 and 5 inclusive. ('{}' specified.)",
                        ordinal
                    ))
                    .into())
                };

                OrdinalSet::from_iter(iter::once(day_of_month_ordinal | occurrence))
            }
            RootSpecifier::Weekday(_) => panic!(
                "W not supported for field {}, got: {:?}",
                Self::name(),
                root_specifier
            ),
        };
        Ok(ordinals)
    }
}

impl PartialEq for DaysOfWeek {
    fn eq(&self, other: &DaysOfWeek) -> bool {
        self.ordinals() == other.ordinals()
    }
}

impl DaysOfWeek {
    pub fn match_day_of<Z>(&self, date: &DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.ordinals()
            .iter()
            .copied()
            .filter_map(|ordinal| {
                // Only consider ordinals that match the weekday.
                if ordinal & !IS_NTH_OCCURRENCE & !IS_LAST_OCCURRENCE
                    == date.weekday().number_from_sunday()
                {
                    Some(ordinal)
                } else {
                    None
                }
            })
            .any(|ordinal| {
                if ordinal & IS_NTH_OCCURRENCE == 0 && ordinal & IS_LAST_OCCURRENCE == 0 {
                    return true;
                }

                let day_of_month = date.day();

                if ordinal & IS_LAST_OCCURRENCE != 0 {
                    let month_ordinal = date.month();
                    let year = date.year() as Ordinal;
                    // Previous check ensures weekday is already matching.
                    // We only check if date is in the last seven days of the month.
                    return day_of_month > days_in_month(month_ordinal, year) - 7;
                }

                if ordinal & IS_NTH_OCCURRENCE != 0 {
                    let week_occurrence = (day_of_month - 1) / 7;
                    return match week_occurrence {
                        0 => ordinal & IS_1ST_OCCURRENCE != 0,
                        1 => ordinal & IS_2ND_OCCURRENCE != 0,
                        2 => ordinal & IS_3RD_OCCURRENCE != 0,
                        3 => ordinal & IS_4TH_OCCURRENCE != 0,
                        4 => ordinal & IS_5TH_OCCURRENCE != 0,
                        _ => false,
                    };
                }

                false
            })
    }
}

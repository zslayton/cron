use crate::error::*;
use crate::ordinal::{
    Ordinal, OrdinalSet, IS_1ST_OCCURRENCE, IS_2ND_OCCURRENCE, IS_3RD_OCCURRENCE,
    IS_4TH_OCCURRENCE, IS_5TH_OCCURRENCE, IS_LAST_OCCURRENCE, IS_NTH_OCCURRENCE,
};
use crate::specifier::{RootSpecifier, SingleSpecifier};
use crate::time_unit::{
    ordinals_from_root_specifier_default, validate_ordinal_default, TimeUnitField,
};
use chrono::{DateTime, Datelike, TimeZone};
use once_cell::sync::Lazy;
use std::borrow::Cow;

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
            nth_of_month_day_of_week if ordinal & IS_NTH_OCCURRENCE != 0 => {
                // There are strictly less than 5 weeks in any month
                if nth_of_month_day_of_week > 5 {
                    Err(ErrorKind::Expression(format!(
                        "Occurrence of a weekday must be between 1 and 5 inclusive. ('{}' specified.)",
                        nth_of_month_day_of_week
                    ))
                    .into())
                } else {
                    Ok(ordinal)
                }
            }
            // Normal case where the ordinal is a day of week
            day_of_week => {
                validate_ordinal_default::<Self>(day_of_week)?;
                Ok(ordinal)
            }
        }
    }

    fn ordinals_from_root_specifier(root_specifier: &RootSpecifier) -> Result<OrdinalSet, Error> {
        let ordinals = match root_specifier {
            RootSpecifier::LastPoint(single_specifier) => {
                // If point value is 0, then we are asked for the last day of the week, which is always Saturday
                OrdinalSet::from_iter([if let SingleSpecifier::Point(0) = single_specifier {
                    Self::inclusive_max()
                } else {
                    // Otherwise we are asked for the last occurrence of a day of week in the month
                    let day_of_week = match single_specifier {
                        SingleSpecifier::Point(ordinal) => Self::validate_ordinal(*ordinal)?,
                        SingleSpecifier::NamedPoint(name) => Self::ordinal_from_name(name)?,
                    };
                    day_of_week | IS_LAST_OCCURRENCE
                }])
            }
            RootSpecifier::NthOfMonth(single_specifier, occurrence_number) => {
                let day_of_week = match single_specifier {
                    SingleSpecifier::Point(ordinal) => Self::validate_ordinal(*ordinal)?,
                    SingleSpecifier::NamedPoint(name) => Self::ordinal_from_name(name)?,
                };

                let occurrence_flag = match occurrence_number {
                    1 => IS_1ST_OCCURRENCE,
                    2 => IS_2ND_OCCURRENCE,
                    3 => IS_3RD_OCCURRENCE,
                    4 => IS_4TH_OCCURRENCE,
                    5 => IS_5TH_OCCURRENCE,
                    i => return Err(ErrorKind::Expression(format!(
                        "Occurrence of a weekday must be between 1 and 5 inclusive. ('{}' specified.)",
                        i
                    ))
                    .into())
                };

                OrdinalSet::from_iter([day_of_week | occurrence_flag])
            }
            // Use default implementation for other root specifiers (Weekday variant must not happen here)
            root_specifier => ordinals_from_root_specifier_default::<Self>(root_specifier)?,
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
    /// Given a date, return true if the date matches a day of week of the specifier,
    /// taking into account the nth occurrence and last occurrence constraints
    pub fn match_day_of<Z>(&self, date: &DateTime<Z>) -> bool
    where
        Z: TimeZone,
    {
        self.ordinals().iter().copied().any(|ordinal| {
            // If day of week does not match without constraint, we know it does not match
            if ordinal & !IS_NTH_OCCURRENCE & !IS_LAST_OCCURRENCE
                != date.weekday().number_from_sunday()
            {
                return false;
            }

            match (ordinal & IS_NTH_OCCURRENCE, ordinal & IS_LAST_OCCURRENCE) {
                // No constraint case, we already verified it matches so we return true
                (0, 0) => true,
                // Last day of week occurrence case
                (_, 1..) => {
                    let month_ordinal = date.month();
                    let year = date.year() as Ordinal;
                    // Previous check ensures weekday is already matching.
                    // We only check if date is in the last seven days of the month.
                    date.day() > super::days_in_month(month_ordinal, year) - 7
                }
                // Nth day of week occurrence case
                // We already checked day of week matches, we can deduce the occurrence
                // using euclidean division of the month day (but starting from 0) by 7
                (1.., 0) => match (date.day() - 1) / 7 {
                    0 => ordinal & IS_1ST_OCCURRENCE != 0,
                    1 => ordinal & IS_2ND_OCCURRENCE != 0,
                    2 => ordinal & IS_3RD_OCCURRENCE != 0,
                    3 => ordinal & IS_4TH_OCCURRENCE != 0,
                    4 => ordinal & IS_5TH_OCCURRENCE != 0,
                    _ => false,
                },
            }
        })
    }
}

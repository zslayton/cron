use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet, IS_LAST_OCCURRENCE, IS_WEEKDAY};
use crate::specifier::{RootSpecifier, SingleSpecifier};
use crate::time_unit::{
    ordinals_from_root_specifier_default, validate_ordinal_default, TimeUnitField,
};
use chrono::Datelike;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::iter;

use super::days_in_month;

static ALL: Lazy<OrdinalSet> = Lazy::new(DaysOfMonth::supported_ordinals);

#[derive(Clone, Debug, Eq)]
pub struct DaysOfMonth {
    ordinals: Option<OrdinalSet>,
}

impl TimeUnitField for DaysOfMonth {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfMonth {
            ordinals: ordinal_set,
        }
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Days of Month")
    }
    fn inclusive_min() -> Ordinal {
        1
    }
    fn inclusive_max() -> Ordinal {
        31
    }
    fn ordinals(&self) -> &OrdinalSet {
        match &self.ordinals {
            Some(ordinal_set) => ordinal_set,
            None => &ALL,
        }
    }
    fn validate_ordinal(ordinal: Ordinal) -> Result<Ordinal, Error> {
        //println!("validate_ordinal for {} => {}", Self::name(), ordinal);
        match ordinal & !IS_LAST_OCCURRENCE & !IS_WEEKDAY {
            i if ordinal & IS_LAST_OCCURRENCE != 0 => {
                if i > 28 {
                    Err(ErrorKind::Expression(format!(
                    "A day of month more than 28 days before the last day of the month for {} may not exist, got {}",
                    Self::name(),
                    i
                ))
                .into())
                } else {
                    Ok(ordinal)
                }
            }
            i => {
                validate_ordinal_default::<Self>(i)?;
                Ok(ordinal)
            }
        }
    }

    fn ordinals_from_root_specifier(root_specifier: &RootSpecifier) -> Result<OrdinalSet, Error> {
        let ordinals = match root_specifier {
            RootSpecifier::LastPoint(single_specifier) => {
                let ordinal = match single_specifier {
                    SingleSpecifier::Point(ordinal) => {
                        if *ordinal > 28 {
                            return Err(ErrorKind::Expression(format!(
                                "A day of month more than 28 days before the last day of the month for {} may not exist",
                                Self::name()
                            ))
                            .into());
                        }
                        *ordinal
                    }
                    SingleSpecifier::NamedPoint(name) => Self::ordinal_from_name(name)?,
                };
                OrdinalSet::from_iter(iter::once(ordinal | IS_LAST_OCCURRENCE))
            }
            RootSpecifier::Weekday(ordinal) => {
                OrdinalSet::from_iter(iter::once(Self::validate_ordinal(*ordinal)? | IS_WEEKDAY))
            }
            root_specifier => ordinals_from_root_specifier_default::<Self>(root_specifier)?,
        };
        Ok(ordinals)
    }
}

impl PartialEq for DaysOfMonth {
    fn eq(&self, other: &DaysOfMonth) -> bool {
        self.ordinals() == other.ordinals()
    }
}

impl DaysOfMonth {
    pub fn days_in_month(&self, month_ordinal: Ordinal, year: Ordinal) -> OrdinalSet {
        let days_in_month = days_in_month(month_ordinal, year);
        self.ordinals()
            .iter()
            .copied()
            .map(|ordinal| {
                if ordinal & IS_WEEKDAY == 0 {
                    if ordinal & IS_LAST_OCCURRENCE == 0 {
                        // Classical day of month without specifier
                        ordinal
                    } else {
                        // Nth day of month before last day of month
                        // (obtain N by switching off IS_LAST_OCCURRENCE bit)
                        days_in_month - (ordinal & !IS_LAST_OCCURRENCE)
                    }
                } else {
                    let day_of_month = if ordinal & IS_LAST_OCCURRENCE == 0 {
                        ordinal & !IS_WEEKDAY
                    } else {
                        days_in_month
                    };
                    let date =
                        chrono::NaiveDate::from_ymd_opt(year as i32, month_ordinal, day_of_month)
                            .expect("day of month must be valid");

                    match date.weekday().number_from_sunday() {
                        1 => {
                            if day_of_month == days_in_month {
                                day_of_month - 2
                            } else {
                                day_of_month + 1
                            }
                        }
                        7 => {
                            if day_of_month == 1 {
                                3
                            } else {
                                day_of_month - 1
                            }
                        }
                        _ => day_of_month,
                    }
                }
            })
            .collect::<OrdinalSet>()
    }
}

use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet, IS_LAST_OCCURRENCE, IS_WEEKDAY};
use crate::specifier::{RootSpecifier, SingleSpecifier};
use crate::time_unit::{
    ordinals_from_root_specifier_default, validate_ordinal_default, TimeUnitField,
};
use chrono::Datelike;
use once_cell::sync::Lazy;
use std::borrow::Cow;

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
            days_count_before_end_of_month if ordinal & IS_LAST_OCCURRENCE != 0 => {
                if days_count_before_end_of_month > 28 {
                    Err(ErrorKind::Expression(format!(
                    "A day of month more than 28 days before the last day of the month for {} may not exist, got {}",
                    Self::name(),
                    days_count_before_end_of_month
                ))
                .into())
                } else {
                    Ok(ordinal)
                }
            }
            // Normal case where ordinal is a day of month
            day_of_month => {
                validate_ordinal_default::<Self>(day_of_month)?;
                Ok(ordinal)
            }
        }
    }

    fn ordinals_from_root_specifier(root_specifier: &RootSpecifier) -> Result<OrdinalSet, Error> {
        let ordinals = match root_specifier {
            RootSpecifier::LastPoint(single_specifier) => {
                let ordinal = match single_specifier {
                    SingleSpecifier::Point(days_count_before_end_of_month) => {
                        if *days_count_before_end_of_month > 28 {
                            return Err(ErrorKind::Expression(format!(
                                "A day of month more than 28 days before the last day of the month for {} may not exist",
                                Self::name()
                            ))
                            .into());
                        }
                        *days_count_before_end_of_month
                    }
                    SingleSpecifier::NamedPoint(name) => Self::ordinal_from_name(name)?,
                };
                // Set the last occurrence flag on resulting ordinal
                OrdinalSet::from_iter([ordinal | IS_LAST_OCCURRENCE])
            }
            RootSpecifier::Weekday(day_of_week) => {
                // Set the weekday flag on resulting ordinal
                OrdinalSet::from_iter([Self::validate_ordinal(*day_of_week)? | IS_WEEKDAY])
            }
            // Use default implementation for other root specifiers (NthOfMonth variant must not happen here)
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
    /// Given a specified month of a specific year, return the days of month that match the specifier
    /// for this month, taking into account the weekday and last occurrence constraints of the specifier.
    pub fn days_in_month(&self, month_ordinal: Ordinal, year: Ordinal) -> OrdinalSet {
        let days_in_month = super::days_in_month(month_ordinal, year);
        self.ordinals()
            .iter()
            .copied()
            .map(|ordinal| {
                // Case where ordinal is not a weekday
                if ordinal & IS_WEEKDAY == 0 {
                    if ordinal & IS_LAST_OCCURRENCE == 0 {
                        // Classical day of month without special constraints
                        ordinal
                    } else {
                        // Nth day of month before last day of month
                        // (obtain N by switching off IS_LAST_OCCURRENCE bit)
                        days_in_month - (ordinal & !IS_LAST_OCCURRENCE)
                    }
                // Case where ordinal must be a weekday
                } else {
                    // Extract the day of month without the weekday constraint
                    let day_of_month = if ordinal & IS_LAST_OCCURRENCE == 0 {
                        ordinal & !IS_WEEKDAY
                    } else {
                        // Case where we are asked for the last week day of the month
                        // we return the number of day in the current month
                        days_in_month
                    };
                    // Get the day of week of the specified day of the current month
                    let day_of_week =
                        chrono::NaiveDate::from_ymd_opt(year as i32, month_ordinal, day_of_month)
                            .expect("day of month must be valid")
                            .weekday()
                            .number_from_sunday();

                    // If the day of the week is not a weekday,
                    // we transform the day to the closest weekday of the same month
                    match day_of_week {
                        // Sunday case
                        1 => {
                            // If this sunday is the last day of the month,
                            // then we return the last friday of the month
                            // as the following monday will not be in the same month
                            if day_of_month == days_in_month {
                                day_of_month - 2
                            // Otherwise, we return the next monday which is in the same month
                            } else {
                                day_of_month + 1
                            }
                        }
                        // Saturday case
                        7 => {
                            // If this saturday is the first day of the month,
                            // then we return the first monday of the month
                            // as the previous friday will not be in the same month
                            if day_of_month == 1 {
                                3
                            // Otherwise, we return the previous friday which is in the same month
                            } else {
                                day_of_month - 1
                            }
                        }
                        // Already a weekday case, nothing to do
                        _ => day_of_month,
                    }
                }
            })
            .collect::<OrdinalSet>()
    }
}

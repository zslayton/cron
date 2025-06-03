use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet, IS_LAST_OCCURRENCE, IS_WEEKDAY};
use crate::specifier::{RootSpecifier, SingleSpecifier, Specifier};
use crate::time_unit::TimeUnitField;
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
            i if i < 6 && (ordinal & IS_WEEKDAY != 0) => Ok(ordinal),
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
                if *ordinal == 0 {
                    OrdinalSet::from_iter(iter::once(IS_LAST_OCCURRENCE | IS_WEEKDAY))
                } else {
                    let ordinal = Self::validate_ordinal(*ordinal)?;
                    OrdinalSet::from_iter(iter::once(ordinal | IS_WEEKDAY))
                }
            }
            RootSpecifier::NthOfMonth(_, _) => {
                panic!(
                    "# not supported for field {}, got: {:?}",
                    Self::name(),
                    root_specifier
                )
            }
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
                    match ordinal & IS_LAST_OCCURRENCE {
                        // Classical day of month without specifier
                        0 => ordinal,
                        // Nth day of month before last day of month
                        // (obtain N by xor of IS_LAST_OCCURRENCE bit)
                        _ => days_in_month - (ordinal ^ IS_LAST_OCCURRENCE),
                    }
                } else {
                    let ordinal = ordinal ^ IS_WEEKDAY;
                    match ordinal & IS_LAST_OCCURRENCE {
                        // Weekday close to a given day of month
                        0 => {
                            let date = chrono::NaiveDate::from_ymd_opt(
                                year as i32,
                                month_ordinal,
                                ordinal,
                            )
                            .expect("day of month must be valid");
                            let weekday = date.weekday().number_from_sunday();
                            println!("WEEKDAY {:?}", weekday);
                            match weekday {
                                1 => {
                                    if ordinal == days_in_month {
                                        ordinal - 2
                                    } else {
                                        ordinal + 1
                                    }
                                }
                                7 => {
                                    if ordinal == 1 {
                                        3
                                    } else {
                                        ordinal - 1
                                    }
                                }
                                _ => ordinal,
                            }
                        }
                        // Find last week day of month and return its ordinal
                        _ => {
                            let date = chrono::NaiveDate::from_ymd_opt(
                                year as i32,
                                month_ordinal,
                                days_in_month,
                            )
                            .expect("last day of month must be valid");
                            let weekday = date.weekday().number_from_sunday();
                            match weekday {
                                1 => days_in_month - 2,
                                7 => days_in_month - 1,
                                _ => days_in_month,
                            }
                        }
                    }
                }
            })
            .collect::<OrdinalSet>()
    }
}

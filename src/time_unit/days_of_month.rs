use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::specifier::{RangeEndpoint, RootSpecifier, Specifier};
use crate::time_unit::{
    days_in_month, ordinal_range_values, ordinal_range_values_with_step, TimeUnitField,
};
use chrono::{Datelike, NaiveDate};
use once_cell::sync::Lazy;
use std::borrow::Cow;

static ALL: Lazy<OrdinalSet> = Lazy::new(DaysOfMonth::supported_ordinals);

#[derive(Clone, Debug, Eq)]
pub struct DaysOfMonth {
    ordinals: Option<OrdinalSet>,
    last_day_of_month: bool,
    nearest_weekdays: OrdinalSet,
}

impl TimeUnitField for DaysOfMonth {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        DaysOfMonth {
            ordinals: ordinal_set,
            last_day_of_month: false,
            nearest_weekdays: OrdinalSet::new(),
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
}

impl PartialEq for DaysOfMonth {
    fn eq(&self, other: &DaysOfMonth) -> bool {
        self.ordinals() == other.ordinals()
            && self.last_day_of_month == other.last_day_of_month
            && self.nearest_weekdays == other.nearest_weekdays
    }
}

impl DaysOfMonth {
    pub(crate) fn from_parts(
        ordinals: Option<OrdinalSet>,
        last_day_of_month: bool,
        nearest_weekdays: OrdinalSet,
    ) -> Self {
        Self {
            ordinals,
            last_day_of_month,
            nearest_weekdays,
        }
    }

    pub(crate) fn has_special_specifiers(&self) -> bool {
        self.last_day_of_month || !self.nearest_weekdays.is_empty()
    }

    pub(crate) fn is_all(&self) -> bool {
        !self.has_special_specifiers()
            && self.ordinals().len() == (Self::inclusive_max() - Self::inclusive_min() + 1) as usize
    }

    pub(crate) fn days_for_month(&self, year: Ordinal, month: Ordinal) -> OrdinalSet {
        let last_day = days_in_month(month, year);
        let mut days = self
            .ordinals()
            .iter()
            .copied()
            .filter(|day| *day <= last_day)
            .collect::<OrdinalSet>();

        if self.last_day_of_month {
            days.insert(last_day);
        }

        for nearest_weekday in &self.nearest_weekdays {
            days.insert(nearest_weekday_for_month(
                year,
                month,
                (*nearest_weekday).min(last_day),
            ));
        }

        days
    }

    pub(crate) fn matches(&self, year: Ordinal, month: Ordinal, day: Ordinal) -> bool {
        self.days_for_month(year, month).contains(&day)
    }

    pub(crate) fn ordinals_from_root_specifier_with_options(
        root_specifier: &RootSpecifier,
        wraparound_ranges: bool,
        last_day_of_month: bool,
    ) -> Result<OrdinalSet, Error> {
        let ordinals = match root_specifier {
            RootSpecifier::Specifier(specifier) => Self::ordinals_from_specifier_with_options(
                specifier,
                wraparound_ranges,
                last_day_of_month,
            )?,
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

                let base_values = match start {
                    Specifier::Point(start) => {
                        let start = Self::validate_ordinal(*start)?;
                        (start..=Self::inclusive_max()).collect()
                    }
                    Specifier::Range(start, end) => {
                        let start_ordinal = Self::ordinal_from_range_endpoint_with_options(
                            start,
                            last_day_of_month,
                        )?;
                        let end_ordinal =
                            Self::ordinal_from_range_endpoint_with_options(end, last_day_of_month)?;
                        return ordinal_range_values_with_step(
                            start_ordinal,
                            end_ordinal,
                            Self::inclusive_min(),
                            Self::inclusive_max(),
                            wraparound_ranges,
                            *step,
                        )
                        .map(|ordinals| ordinals.into_iter().collect())
                        .ok_or_else(|| {
                            ErrorKind::Expression(format!(
                                "Invalid range for {}: {}-{}",
                                Self::name(),
                                start,
                                end
                            ))
                            .into()
                        });
                    }
                    specifier => Self::ordinal_values_from_specifier_with_options(
                        specifier,
                        wraparound_ranges,
                        last_day_of_month,
                    )?,
                };
                base_values.into_iter().step_by(*step as usize).collect()
            }
            RootSpecifier::NamedPoint(name) => {
                if last_day_of_month && name.eq_ignore_ascii_case("l") {
                    return Ok(([Self::inclusive_max()]).into_iter().collect());
                }
                ([Self::ordinal_from_name(name)?]).into_iter().collect()
            }
            _ => {
                return Err(ErrorKind::Expression(format!(
                    "Root specifier not supported for {}: {:?}",
                    Self::name(),
                    root_specifier
                ))
                .into())
            }
        };
        Ok(ordinals)
    }

    fn ordinals_from_specifier_with_options(
        specifier: &Specifier,
        wraparound_ranges: bool,
        last_day_of_month: bool,
    ) -> Result<OrdinalSet, Error> {
        Ok(Self::ordinal_values_from_specifier_with_options(
            specifier,
            wraparound_ranges,
            last_day_of_month,
        )?
        .into_iter()
        .collect())
    }

    fn ordinal_values_from_specifier_with_options(
        specifier: &Specifier,
        wraparound_ranges: bool,
        last_day_of_month: bool,
    ) -> Result<Vec<Ordinal>, Error> {
        match specifier {
            Specifier::All => Ok((Self::inclusive_min()..=Self::inclusive_max()).collect()),
            Specifier::Point(ordinal) => Ok(vec![Self::validate_ordinal(*ordinal)?]),
            Specifier::Range(start, end) => {
                let start_ordinal =
                    Self::ordinal_from_range_endpoint_with_options(start, last_day_of_month)?;
                let end_ordinal =
                    Self::ordinal_from_range_endpoint_with_options(end, last_day_of_month)?;
                ordinal_range_values(
                    start_ordinal,
                    end_ordinal,
                    Self::inclusive_min(),
                    Self::inclusive_max(),
                    wraparound_ranges,
                )
                .ok_or_else(|| {
                    ErrorKind::Expression(format!(
                        "Invalid range for {}: {}-{}",
                        Self::name(),
                        start,
                        end
                    ))
                    .into()
                })
            }
        }
    }

    fn ordinal_from_range_endpoint_with_options(
        endpoint: &RangeEndpoint,
        last_day_of_month: bool,
    ) -> Result<Ordinal, Error> {
        match endpoint {
            RangeEndpoint::Ordinal(ordinal) => Self::validate_ordinal(*ordinal),
            RangeEndpoint::Name(name) if last_day_of_month && name.eq_ignore_ascii_case("l") => {
                Ok(Self::inclusive_max())
            }
            RangeEndpoint::Name(name) => Self::ordinal_from_name(name),
        }
    }
}

fn nearest_weekday_for_month(year: Ordinal, month: Ordinal, day: Ordinal) -> Ordinal {
    let last_day = days_in_month(month, year);
    let day = day.min(last_day);
    let weekday = NaiveDate::from_ymd_opt(year as i32, month, day)
        .expect("day of month must be valid")
        .weekday()
        .number_from_monday();

    match weekday {
        1..=5 => day,
        6 if day == 1 => day + 2,
        6 => day - 1,
        7 if day == last_day => day - 2,
        7 => day + 1,
        _ => unreachable!("chrono weekday is always in 1..=7"),
    }
}

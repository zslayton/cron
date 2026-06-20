mod days_of_month;
mod days_of_week;
mod hours;
mod minutes;
mod months;
mod seconds;
mod years;

pub use self::days_of_month::DaysOfMonth;
pub use self::days_of_week::DaysOfWeek;
pub use self::hours::Hours;
pub use self::minutes::Minutes;
pub use self::months::Months;
pub use self::seconds::Seconds;
pub use self::years::Years;

use crate::error::*;
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::specifier::{RangeEndpoint, RootSpecifier, Specifier};
use std::borrow::Cow;
use std::collections::btree_set;
use std::iter;
use std::ops::RangeBounds;

pub struct OrdinalIter<'a> {
    set_iter: btree_set::Iter<'a, Ordinal>,
}

impl Iterator for OrdinalIter<'_> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
        self.set_iter.next().copied()
    }
}

impl DoubleEndedIterator for OrdinalIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.set_iter.next_back().copied()
    }
}

pub struct OrdinalRangeIter<'a> {
    range_iter: btree_set::Range<'a, Ordinal>,
}

impl Iterator for OrdinalRangeIter<'_> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
        self.range_iter.next().copied()
    }
}

impl DoubleEndedIterator for OrdinalRangeIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range_iter.next_back().copied()
    }
}

/// Methods exposing a schedule's configured ordinals for each individual unit of time.
/// # Example
/// ```
/// use cron::{Schedule,TimeUnitSpec};
/// use std::ops::Bound::{Included,Excluded};
/// use std::str::FromStr;
///
/// let expression = "* * * * * * 2015-2044";
/// let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
///
/// // Membership
/// assert_eq!(true, schedule.years().includes(2031));
/// assert_eq!(false, schedule.years().includes(1969));
///
/// // Number of years specified
/// assert_eq!(30, schedule.years().count());
///
/// // Iterator
/// let mut years_iter = schedule.years().iter();
/// assert_eq!(Some(2015), years_iter.next());
/// assert_eq!(Some(2016), years_iter.next());
/// // ...
///
/// // Range Iterator
/// let mut five_year_plan = schedule.years().range((Included(2017), Excluded(2017 + 5)));
/// assert_eq!(Some(2017), five_year_plan.next());
/// assert_eq!(Some(2018), five_year_plan.next());
/// assert_eq!(Some(2019), five_year_plan.next());
/// assert_eq!(Some(2020), five_year_plan.next());
/// assert_eq!(Some(2021), five_year_plan.next());
/// assert_eq!(None, five_year_plan.next());
/// ```
pub trait TimeUnitSpec {
    /// Returns true if the provided ordinal was included in the schedule spec for the unit of time
    /// being described.
    /// # Example
    /// ```
    /// use cron::{Schedule,TimeUnitSpec};
    /// use std::str::FromStr;
    ///
    /// let expression = "* * * * * * 2015-2044";
    /// let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
    ///
    /// // Membership
    /// assert_eq!(true, schedule.years().includes(2031));
    /// assert_eq!(false, schedule.years().includes(2004));
    /// ```
    fn includes(&self, ordinal: Ordinal) -> bool;

    /// Provides an iterator which will return each included ordinal for this schedule in order from
    /// lowest to highest.
    /// # Example
    /// ```
    /// use cron::{Schedule,TimeUnitSpec};
    /// use std::str::FromStr;
    ///
    /// let expression = "* * * * 5-8 * *";
    /// let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
    ///
    /// // Iterator
    /// let mut summer = schedule.months().iter();
    /// assert_eq!(Some(5), summer.next());
    /// assert_eq!(Some(6), summer.next());
    /// assert_eq!(Some(7), summer.next());
    /// assert_eq!(Some(8), summer.next());
    /// assert_eq!(None, summer.next());
    /// ```
    fn iter(&self) -> OrdinalIter<'_>;

    /// Provides an iterator which will return each included ordinal within the specified range.
    /// # Example
    /// ```
    /// use cron::{Schedule,TimeUnitSpec};
    /// use std::ops::Bound::{Included,Excluded};
    /// use std::str::FromStr;
    ///
    /// let expression = "* * * 1,15 * * *";
    /// let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
    ///
    /// // Range Iterator
    /// let mut mid_month_paydays = schedule.days_of_month().range((Included(10), Included(20)));
    /// assert_eq!(Some(15), mid_month_paydays.next());
    /// assert_eq!(None, mid_month_paydays.next());
    /// ```
    fn range<R>(&self, range: R) -> OrdinalRangeIter<'_>
    where
        R: RangeBounds<Ordinal>;

    /// Returns the number of ordinals included in the associated schedule
    /// # Example
    /// ```
    /// use cron::{Schedule,TimeUnitSpec};
    /// use std::str::FromStr;
    ///
    /// let expression = "* * * 1,15 * * *";
    /// let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
    ///
    /// assert_eq!(2, schedule.days_of_month().count());
    /// ```
    fn count(&self) -> u32;

    /// Checks if this TimeUnitSpec is defined as all possibilities (thus created with a '*', '?' or in the case of weekdays '1-7')
    /// # Example
    /// ```
    /// use cron::{Schedule,TimeUnitSpec};
    /// use std::str::FromStr;
    ///
    /// let expression = "* * * 1,15 * * *";
    /// let schedule = Schedule::from_str(expression).expect("Failed to parse expression.");
    ///
    /// assert_eq!(false, schedule.days_of_month().is_all());
    /// assert_eq!(true, schedule.months().is_all());
    /// ```
    fn is_all(&self) -> bool;
}

impl<T> TimeUnitSpec for T
where
    T: TimeUnitField,
{
    fn includes(&self, ordinal: Ordinal) -> bool {
        self.contains_ordinal(ordinal)
    }
    fn iter(&self) -> OrdinalIter<'_> {
        OrdinalIter {
            set_iter: TimeUnitField::ordinals(self).iter(),
        }
    }
    fn range<R>(&'_ self, range: R) -> OrdinalRangeIter<'_>
    where
        R: RangeBounds<Ordinal>,
    {
        OrdinalRangeIter {
            range_iter: TimeUnitField::ordinals(self).range(range),
        }
    }
    fn count(&self) -> u32 {
        self.ordinals().len() as u32
    }

    fn is_all(&self) -> bool {
        self.has_all_ordinals()
    }
}

pub(crate) fn ordinal_range_values(
    start: Ordinal,
    end: Ordinal,
    inclusive_min: Ordinal,
    inclusive_max: Ordinal,
    wraparound_ranges: bool,
) -> Option<Vec<Ordinal>> {
    ordinal_range_values_with_step(
        start,
        end,
        inclusive_min,
        inclusive_max,
        wraparound_ranges,
        1,
    )
}

pub(crate) fn ordinal_range_values_with_step(
    start: Ordinal,
    end: Ordinal,
    inclusive_min: Ordinal,
    inclusive_max: Ordinal,
    wraparound_ranges: bool,
    step: Ordinal,
) -> Option<Vec<Ordinal>> {
    let step = step as usize;

    if wraparound_ranges && start == end {
        Some((inclusive_min..=inclusive_max).step_by(step).collect())
    } else if start <= end {
        Some((start..=end).step_by(step).collect())
    } else if wraparound_ranges {
        let first_segment = (start..=inclusive_max).step_by(step).collect::<Vec<_>>();
        // Croniter preserves the original range step across the max-to-min boundary by
        // skipping initial wrapped values when the first segment lands near the field max.
        let to_skip = first_segment
            .last()
            .map(|last| {
                let step = step as Ordinal;
                let already_skipped = inclusive_max - last;
                let current_position = last - inclusive_min;
                let field_len = inclusive_max - inclusive_min + 1;
                if current_position + step > field_len && already_skipped < step {
                    step - already_skipped
                } else {
                    0
                }
            })
            .unwrap_or(0);
        Some(
            first_segment
                .into_iter()
                .chain(((inclusive_min + to_skip)..=end).step_by(step))
                .collect(),
        )
    } else {
        None
    }
}

fn is_leap_year(year: Ordinal) -> bool {
    let by_four = year.is_multiple_of(4);
    let by_hundred = year.is_multiple_of(100);
    let by_four_hundred = year.is_multiple_of(400);
    by_four && ((!by_hundred) || by_four_hundred)
}

pub(crate) fn days_in_month(month: Ordinal, year: Ordinal) -> Ordinal {
    let is_leap_year = is_leap_year(year);
    match month {
        9 | 4 | 6 | 11 => 30,
        2 if is_leap_year => 29,
        2 => 28,
        _ => 31,
    }
}

pub trait TimeUnitField
where
    Self: Sized,
{
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self;
    fn name() -> Cow<'static, str>;
    fn inclusive_min() -> Ordinal;
    fn inclusive_max() -> Ordinal;
    fn ordinals(&self) -> &OrdinalSet;

    fn contains_ordinal(&self, ordinal: Ordinal) -> bool {
        self.ordinals().contains(&ordinal)
    }

    fn has_all_ordinals(&self) -> bool {
        let max_supported_ordinals = Self::inclusive_max() - Self::inclusive_min() + 1;
        self.ordinals().len() == max_supported_ordinals as usize
    }

    fn from_ordinal(ordinal: Ordinal) -> Self {
        Self::from_ordinal_set(iter::once(ordinal).collect())
    }

    fn supported_ordinals() -> OrdinalSet {
        (Self::inclusive_min()..Self::inclusive_max() + 1).collect()
    }

    fn all() -> Self {
        Self::from_optional_ordinal_set(None)
    }

    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        Self::from_optional_ordinal_set(Some(ordinal_set))
    }

    fn ordinal_from_name(name: &str) -> Result<Ordinal, Error> {
        Err(ErrorKind::Expression(format!(
            "The '{}' field does not support using names. '{}' \
             specified.",
            Self::name(),
            name
        ))
        .into())
    }

    fn ordinal_from_range_endpoint(endpoint: &RangeEndpoint) -> Result<Ordinal, Error> {
        match endpoint {
            RangeEndpoint::Ordinal(ordinal) => Ok(*ordinal),
            RangeEndpoint::Name(name) => Self::ordinal_from_name(name),
        }
    }

    fn validate_ordinal(ordinal: Ordinal) -> Result<Ordinal, Error> {
        //println!("validate_ordinal for {} => {}", Self::name(), ordinal);
        match ordinal {
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
            i => Ok(i),
        }
    }

    fn ordinals_from_specifier(specifier: &Specifier) -> Result<OrdinalSet, Error> {
        Self::ordinals_from_specifier_with_options(specifier, false)
    }

    fn ordinal_values_from_specifier_with_options(
        specifier: &Specifier,
        wraparound_ranges: bool,
    ) -> Result<Vec<Ordinal>, Error> {
        use self::Specifier::*;
        //println!("ordinals_from_specifier for {} => {:?}", Self::name(), specifier);
        match specifier {
            All => Ok((Self::inclusive_min()..=Self::inclusive_max()).collect()),
            Point(ordinal) => Ok(vec![Self::validate_ordinal(*ordinal)?]),
            Range(start, end) => {
                let start_ordinal =
                    Self::validate_ordinal(Self::ordinal_from_range_endpoint(start)?)?;
                let end_ordinal = Self::validate_ordinal(Self::ordinal_from_range_endpoint(end)?)?;
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

    fn ordinals_from_specifier_with_options(
        specifier: &Specifier,
        wraparound_ranges: bool,
    ) -> Result<OrdinalSet, Error> {
        Ok(
            Self::ordinal_values_from_specifier_with_options(specifier, wraparound_ranges)?
                .into_iter()
                .collect(),
        )
    }

    fn ordinals_from_root_specifier(root_specifier: &RootSpecifier) -> Result<OrdinalSet, Error> {
        Self::ordinals_from_root_specifier_with_options(root_specifier, false)
    }

    fn ordinals_from_root_specifier_with_options(
        root_specifier: &RootSpecifier,
        wraparound_ranges: bool,
    ) -> Result<OrdinalSet, Error> {
        let ordinals = match root_specifier {
            RootSpecifier::Specifier(specifier) => {
                Self::ordinals_from_specifier_with_options(specifier, wraparound_ranges)?
            }
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
                    // A point prior to a period implies a range whose start is the specified
                    // point and terminating inclusively with the inclusive max
                    Specifier::Point(start) => {
                        let start = Self::validate_ordinal(*start)?;
                        (start..=Self::inclusive_max()).collect()
                    }
                    Specifier::Range(start, end) => {
                        let start_ordinal =
                            Self::validate_ordinal(Self::ordinal_from_range_endpoint(start)?)?;
                        let end_ordinal =
                            Self::validate_ordinal(Self::ordinal_from_range_endpoint(end)?)?;
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
                    )?,
                };
                base_values.into_iter().step_by(*step as usize).collect()
            }
            RootSpecifier::NamedPoint(ref name) => ([Self::ordinal_from_name(name)?])
                .iter()
                .cloned()
                .collect::<OrdinalSet>(),
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
}

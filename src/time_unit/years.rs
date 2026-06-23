use crate::error::{Error, ErrorKind};
use crate::ordinal::{Ordinal, OrdinalSet, OrdinalSetIter};
use crate::specifier::{RangeEndpoint, RootSpecifier, Specifier};
use crate::time_unit::TimeUnitField;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::ops::Bound;

const FIRST_YEAR: Ordinal = 0;
const LAST_YEAR: Ordinal = i32::MAX as Ordinal;
const MAX_MATERIALIZED_YEARS: Ordinal = 10_000;

static EMPTY: Lazy<OrdinalSet> = Lazy::new(|| OrdinalSet::empty(FIRST_YEAR, FIRST_YEAR));

#[derive(Clone, Debug, Eq, PartialEq)]
enum YearsSpec {
    All,
    Ordinals(OrdinalSet),
    Predicates {
        specifiers: Vec<RootSpecifier>,
        wraparound_ranges: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Years {
    spec: YearsSpec,
}

impl TimeUnitField for Years {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        Years {
            spec: ordinal_set.map_or(YearsSpec::All, YearsSpec::Ordinals),
        }
    }

    fn name() -> Cow<'static, str> {
        Cow::from("Years")
    }

    fn inclusive_min() -> Ordinal {
        FIRST_YEAR
    }

    fn inclusive_max() -> Ordinal {
        LAST_YEAR
    }

    fn ordinals(&self) -> &OrdinalSet {
        match &self.spec {
            YearsSpec::Ordinals(ordinals) => ordinals,
            YearsSpec::All | YearsSpec::Predicates { .. } => &EMPTY,
        }
    }

    fn contains_ordinal(&self, ordinal: Ordinal) -> bool {
        match &self.spec {
            YearsSpec::All => ordinal <= LAST_YEAR,
            YearsSpec::Ordinals(ordinals) => ordinals.contains(&ordinal),
            YearsSpec::Predicates {
                specifiers,
                wraparound_ranges,
            } => {
                ordinal <= LAST_YEAR
                    && specifiers
                        .iter()
                        .any(|specifier| root_contains(specifier, ordinal, *wraparound_ranges))
            }
        }
    }

    fn has_all_ordinals(&self) -> bool {
        matches!(self.spec, YearsSpec::All)
    }

    fn iter_ordinals(&self) -> crate::time_unit::OrdinalIter<'_> {
        match &self.spec {
            YearsSpec::All => crate::time_unit::OrdinalIter {
                iter: Box::new(FIRST_YEAR..=LAST_YEAR),
            },
            YearsSpec::Ordinals(ordinals) => crate::time_unit::OrdinalIter {
                iter: Box::new(ordinals.iter()),
            },
            YearsSpec::Predicates { .. } => crate::time_unit::OrdinalIter {
                iter: Box::new(
                    (FIRST_YEAR..=LAST_YEAR).filter(move |year| self.contains_ordinal(*year)),
                ),
            },
        }
    }

    fn range_ordinals(
        &self,
        range: (Bound<Ordinal>, Bound<Ordinal>),
    ) -> crate::time_unit::OrdinalRangeIter<'_> {
        let Some((start, end)) = inclusive_bounds(range) else {
            return crate::time_unit::OrdinalRangeIter {
                iter: Box::new(std::iter::empty()),
            };
        };

        crate::time_unit::OrdinalRangeIter {
            iter: Box::new(self.ordinals_between(start, end)),
        }
    }

    fn count_ordinals(&self) -> u32 {
        match &self.spec {
            YearsSpec::All => LAST_YEAR - FIRST_YEAR + 1,
            YearsSpec::Ordinals(ordinals) => ordinals.len() as u32,
            YearsSpec::Predicates {
                specifiers,
                wraparound_ranges,
            } if specifiers.len() == 1 => root_specifier_count(&specifiers[0], *wraparound_ranges),
            YearsSpec::Predicates { .. } => self.iter_ordinals().count() as u32,
        }
    }

    fn from_ordinal(ordinal: Ordinal) -> Self {
        let ordinal =
            Self::validate_ordinal(ordinal).expect("ordinal outside supported Years range");
        Self::from_ordinal_set(OrdinalSet::from_values(ordinal, ordinal, [ordinal]))
    }
}

impl Years {
    pub(crate) fn from_root_specifiers(
        specifiers: Vec<RootSpecifier>,
        wraparound_ranges: bool,
    ) -> Result<Self, Error> {
        validate_root_specifiers(&specifiers, wraparound_ranges)?;

        if specifiers
            .iter()
            .any(|specifier| matches!(specifier, RootSpecifier::Specifier(Specifier::All)))
        {
            return Ok(Self::all());
        }

        if let Some(ordinals) = materialize_ordinals(&specifiers, wraparound_ranges) {
            return Ok(Self::from_ordinal_set(ordinals));
        }

        Ok(Years {
            spec: YearsSpec::Predicates {
                specifiers,
                wraparound_ranges,
            },
        })
    }

    pub(crate) fn is_unrestricted(&self) -> bool {
        matches!(self.spec, YearsSpec::All)
    }

    pub(crate) fn ordinals_between(&self, start: Ordinal, end: Ordinal) -> YearRangeIter<'_> {
        let end = end.min(LAST_YEAR);
        if start > end {
            return YearRangeIter::empty();
        }

        match &self.spec {
            YearsSpec::All => YearRangeIter::range(start, end),
            YearsSpec::Ordinals(ordinals) => YearRangeIter::Set(ordinals.range(start..=end)),
            YearsSpec::Predicates { .. } => YearRangeIter::Predicates {
                years: self,
                front: start,
                back: end,
                exhausted: false,
            },
        }
    }
}

pub(crate) enum YearRangeIter<'a> {
    Empty,
    Range {
        front: Ordinal,
        back: Ordinal,
        exhausted: bool,
    },
    Set(OrdinalSetIter<'a>),
    Predicates {
        years: &'a Years,
        front: Ordinal,
        back: Ordinal,
        exhausted: bool,
    },
}

impl YearRangeIter<'_> {
    fn empty() -> Self {
        Self::Empty
    }

    fn range(start: Ordinal, end: Ordinal) -> Self {
        Self::Range {
            front: start,
            back: end,
            exhausted: false,
        }
    }
}

impl Iterator for YearRangeIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Range {
                front,
                back,
                exhausted,
            } => {
                if *exhausted {
                    return None;
                }

                let candidate = *front;
                if front == back {
                    *exhausted = true;
                } else {
                    *front += 1;
                }
                Some(candidate)
            }
            Self::Set(iter) => iter.next(),
            Self::Predicates {
                years,
                front,
                back,
                exhausted,
            } => {
                if *exhausted {
                    return None;
                }

                while *front <= *back {
                    let candidate = *front;
                    if front == back {
                        *exhausted = true;
                    } else {
                        *front += 1;
                    }
                    if years.contains_ordinal(candidate) {
                        return Some(candidate);
                    }
                }

                *exhausted = true;
                None
            }
        }
    }
}

impl DoubleEndedIterator for YearRangeIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match self {
            Self::Empty => None,
            Self::Range {
                front,
                back,
                exhausted,
            } => {
                if *exhausted {
                    return None;
                }

                let candidate = *back;
                if front == back {
                    *exhausted = true;
                } else {
                    *back -= 1;
                }
                Some(candidate)
            }
            Self::Set(iter) => iter.next_back(),
            Self::Predicates {
                years,
                front,
                back,
                exhausted,
            } => {
                if *exhausted {
                    return None;
                }

                while *front <= *back {
                    let candidate = *back;
                    if front == back {
                        *exhausted = true;
                    } else {
                        *back -= 1;
                    }
                    if years.contains_ordinal(candidate) {
                        return Some(candidate);
                    }
                }

                *exhausted = true;
                None
            }
        }
    }
}

fn inclusive_bounds(range: (Bound<Ordinal>, Bound<Ordinal>)) -> Option<(Ordinal, Ordinal)> {
    let start = match range.0 {
        Bound::Included(ordinal) => ordinal,
        Bound::Excluded(ordinal) => ordinal.checked_add(1)?,
        Bound::Unbounded => FIRST_YEAR,
    };
    let end = match range.1 {
        Bound::Included(ordinal) => ordinal,
        Bound::Excluded(ordinal) => ordinal.checked_sub(1)?,
        Bound::Unbounded => LAST_YEAR,
    };
    Some((start, end))
}

fn validate_root_specifiers(
    specifiers: &[RootSpecifier],
    wraparound_ranges: bool,
) -> Result<(), Error> {
    for specifier in specifiers {
        match specifier {
            RootSpecifier::Specifier(specifier) => {
                validate_specifier(specifier, wraparound_ranges)?
            }
            RootSpecifier::Period(_, 0) => {
                return Err(ErrorKind::Expression("range step cannot be zero".to_string()).into());
            }
            RootSpecifier::Period(specifier, _) => {
                validate_specifier(specifier, wraparound_ranges)?
            }
            RootSpecifier::NamedPoint(name) => {
                Years::ordinal_from_name(name)?;
            }
            specifier => {
                return Err(ErrorKind::Expression(format!(
                    "Root specifier not supported for Years: {:?}",
                    specifier
                ))
                .into());
            }
        }
    }
    Ok(())
}

fn validate_specifier(specifier: &Specifier, wraparound_ranges: bool) -> Result<(), Error> {
    match specifier {
        Specifier::All => Ok(()),
        Specifier::Point(ordinal) => {
            Years::validate_ordinal(*ordinal)?;
            Ok(())
        }
        Specifier::Range(start, end) => {
            let start = Years::validate_ordinal(ordinal_from_endpoint(start)?)?;
            let end = Years::validate_ordinal(ordinal_from_endpoint(end)?)?;
            if start > end && !wraparound_ranges {
                return Err(ErrorKind::Expression(format!(
                    "Invalid range for Years: {}-{}",
                    start, end
                ))
                .into());
            }
            Ok(())
        }
    }
}

fn ordinal_from_endpoint(endpoint: &RangeEndpoint) -> Result<Ordinal, Error> {
    match endpoint {
        RangeEndpoint::Ordinal(ordinal) => Ok(*ordinal),
        RangeEndpoint::Name(name) => Years::ordinal_from_name(name),
    }
}

fn materialize_ordinals(
    specifiers: &[RootSpecifier],
    wraparound_ranges: bool,
) -> Option<OrdinalSet> {
    let mut values = Vec::new();
    for specifier in specifiers {
        values.extend(materialize_root_specifier(specifier, wraparound_ranges)?);
    }
    let min = values.iter().copied().min()?;
    let max = values.iter().copied().max()?;
    OrdinalSet::try_from_values(min, max, values)
}

fn materialize_root_specifier(
    root_specifier: &RootSpecifier,
    wraparound_ranges: bool,
) -> Option<Vec<Ordinal>> {
    match root_specifier {
        RootSpecifier::Specifier(specifier) => materialize_specifier(specifier, wraparound_ranges),
        RootSpecifier::Period(Specifier::Range(start, end), step) => {
            let start = ordinal_from_endpoint(start).ok()?;
            let end = ordinal_from_endpoint(end).ok()?;
            let len = years_in_range(start, end);
            if (wraparound_ranges && start >= end) || len > MAX_MATERIALIZED_YEARS {
                return None;
            }
            Some((start..=end).step_by(*step as usize).collect())
        }
        RootSpecifier::Period(_, _)
        | RootSpecifier::NamedPoint(_)
        | RootSpecifier::LastDayOfMonth
        | RootSpecifier::NearestWeekday(_)
        | RootSpecifier::LastWeekdayOfMonth(_)
        | RootSpecifier::NthWeekdayOfMonth(_, _)
        | RootSpecifier::NthWeekdayRangeOfMonth(_, _, _)
        | RootSpecifier::Random(_) => None,
    }
}

fn materialize_specifier(specifier: &Specifier, wraparound_ranges: bool) -> Option<Vec<Ordinal>> {
    match specifier {
        Specifier::All => None,
        Specifier::Point(ordinal) => Some(vec![*ordinal]),
        Specifier::Range(start, end) => {
            let start = ordinal_from_endpoint(start).ok()?;
            let end = ordinal_from_endpoint(end).ok()?;
            let len = years_in_range(start, end);
            if (wraparound_ranges && start >= end) || len > MAX_MATERIALIZED_YEARS {
                return None;
            }
            Some((start..=end).collect())
        }
    }
}

fn years_in_range(start: Ordinal, end: Ordinal) -> Ordinal {
    end.saturating_sub(start).saturating_add(1)
}

fn root_specifier_count(root_specifier: &RootSpecifier, wraparound_ranges: bool) -> u32 {
    match root_specifier {
        RootSpecifier::Specifier(Specifier::All) => LAST_YEAR - FIRST_YEAR + 1,
        RootSpecifier::Specifier(Specifier::Point(_)) => 1,
        RootSpecifier::Specifier(Specifier::Range(start, end)) => {
            let (Ok(start), Ok(end)) = (ordinal_from_endpoint(start), ordinal_from_endpoint(end))
            else {
                return 0;
            };
            if wraparound_ranges && start == end {
                return LAST_YEAR - FIRST_YEAR + 1;
            }
            if start <= end {
                years_in_range(start, end)
            } else {
                years_in_range(start, LAST_YEAR) + years_in_range(FIRST_YEAR, end)
            }
        }
        RootSpecifier::Period(specifier, step) if *step > 0 => {
            period_count(specifier, *step, wraparound_ranges)
        }
        RootSpecifier::Period(_, _)
        | RootSpecifier::NamedPoint(_)
        | RootSpecifier::LastDayOfMonth
        | RootSpecifier::NearestWeekday(_)
        | RootSpecifier::LastWeekdayOfMonth(_)
        | RootSpecifier::NthWeekdayOfMonth(_, _)
        | RootSpecifier::NthWeekdayRangeOfMonth(_, _, _)
        | RootSpecifier::Random(_) => 0,
    }
}

fn period_count(specifier: &Specifier, step: Ordinal, wraparound_ranges: bool) -> u32 {
    match specifier {
        Specifier::All => ((LAST_YEAR - FIRST_YEAR) / step) + 1,
        Specifier::Point(start) => ((LAST_YEAR - start) / step) + 1,
        Specifier::Range(start, end) => {
            let (Ok(start), Ok(end)) = (ordinal_from_endpoint(start), ordinal_from_endpoint(end))
            else {
                return 0;
            };
            if wraparound_ranges && start == end {
                return ((LAST_YEAR - FIRST_YEAR) / step) + 1;
            }
            if start <= end {
                ((end - start) / step) + 1
            } else {
                let second_start = wrapped_second_segment_start(start, step);
                let second_segment = if second_start <= end {
                    ((end - second_start) / step) + 1
                } else {
                    0
                };
                ((LAST_YEAR - start) / step) + 1 + second_segment
            }
        }
    }
}

fn root_contains(
    root_specifier: &RootSpecifier,
    ordinal: Ordinal,
    wraparound_ranges: bool,
) -> bool {
    match root_specifier {
        RootSpecifier::Specifier(specifier) => {
            specifier_contains(specifier, ordinal, wraparound_ranges)
        }
        RootSpecifier::Period(specifier, step) => {
            period_contains(specifier, ordinal, *step, wraparound_ranges)
        }
        RootSpecifier::NamedPoint(_)
        | RootSpecifier::LastDayOfMonth
        | RootSpecifier::NearestWeekday(_)
        | RootSpecifier::LastWeekdayOfMonth(_)
        | RootSpecifier::NthWeekdayOfMonth(_, _)
        | RootSpecifier::NthWeekdayRangeOfMonth(_, _, _)
        | RootSpecifier::Random(_) => false,
    }
}

fn specifier_contains(specifier: &Specifier, ordinal: Ordinal, wraparound_ranges: bool) -> bool {
    match specifier {
        Specifier::All => true,
        Specifier::Point(point) => ordinal == *point,
        Specifier::Range(start, end) => {
            let (Ok(start), Ok(end)) = (ordinal_from_endpoint(start), ordinal_from_endpoint(end))
            else {
                return false;
            };
            range_contains(ordinal, start, end, 1, wraparound_ranges)
        }
    }
}

fn period_contains(
    specifier: &Specifier,
    ordinal: Ordinal,
    step: Ordinal,
    wraparound_ranges: bool,
) -> bool {
    if step == 0 {
        return false;
    }

    match specifier {
        Specifier::All => (ordinal - FIRST_YEAR).is_multiple_of(step),
        Specifier::Point(start) => ordinal >= *start && (ordinal - start).is_multiple_of(step),
        Specifier::Range(start, end) => {
            let (Ok(start), Ok(end)) = (ordinal_from_endpoint(start), ordinal_from_endpoint(end))
            else {
                return false;
            };
            range_contains(ordinal, start, end, step, wraparound_ranges)
        }
    }
}

fn range_contains(
    ordinal: Ordinal,
    start: Ordinal,
    end: Ordinal,
    step: Ordinal,
    wraparound_ranges: bool,
) -> bool {
    if wraparound_ranges && start == end {
        return (ordinal - FIRST_YEAR).is_multiple_of(step);
    }
    if start <= end {
        return ordinal >= start && ordinal <= end && (ordinal - start).is_multiple_of(step);
    }
    if !wraparound_ranges {
        return false;
    }

    ordinal >= start && (ordinal - start).is_multiple_of(step)
        || wrapped_segment_contains(ordinal, start, end, step)
}

fn wrapped_segment_contains(ordinal: Ordinal, start: Ordinal, end: Ordinal, step: Ordinal) -> bool {
    let ordinal = u64::from(ordinal);
    let second_start = u64::from(wrapped_second_segment_start(start, step));

    ordinal >= second_start
        && ordinal <= u64::from(end)
        && (ordinal - second_start).is_multiple_of(u64::from(step))
}

fn wrapped_second_segment_start(start: Ordinal, step: Ordinal) -> Ordinal {
    let max = u64::from(LAST_YEAR);
    let start = u64::from(start);
    let step = u64::from(step);
    let last = start + ((max - start) / step) * step;
    let already_skipped = max - last;
    let field_len = max - u64::from(FIRST_YEAR) + 1;
    let to_skip = if last - u64::from(FIRST_YEAR) + step > field_len && already_skipped < step {
        step - already_skipped
    } else {
        0
    };
    (u64::from(FIRST_YEAR) + to_skip) as Ordinal
}

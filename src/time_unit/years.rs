use crate::error::{Error, ErrorKind};
use crate::ordinal::{Ordinal, OrdinalSet};
use crate::specifier::{RangeEndpoint, RootSpecifier, Specifier};
use crate::time_unit::TimeUnitField;
use once_cell::sync::Lazy;
use std::borrow::Cow;

const FIRST_YEAR: Ordinal = 0;
const LAST_YEAR: Ordinal = i32::MAX as Ordinal;
const MAX_MATERIALIZED_YEARS: Ordinal = 10_000;

static EMPTY: Lazy<OrdinalSet> = Lazy::new(OrdinalSet::new);

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

    pub(crate) fn ordinals_between(
        &self,
        start: Ordinal,
        end: Ordinal,
    ) -> Box<dyn DoubleEndedIterator<Item = Ordinal> + '_> {
        let end = end.min(LAST_YEAR);
        if start > end {
            return Box::new(std::iter::empty());
        }

        match &self.spec {
            YearsSpec::All => Box::new(start..=end),
            YearsSpec::Ordinals(ordinals) => Box::new(ordinals.range(start..=end).copied()),
            YearsSpec::Predicates { .. } => {
                Box::new((start..=end).filter(move |year| self.contains_ordinal(*year)))
            }
        }
    }
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
    let mut ordinals = OrdinalSet::new();
    for specifier in specifiers {
        ordinals.extend(materialize_root_specifier(specifier, wraparound_ranges)?);
    }
    Some(ordinals)
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
    let ordinal = u64::from(ordinal);
    let second_start = u64::from(FIRST_YEAR) + to_skip;

    ordinal >= second_start
        && ordinal <= u64::from(end)
        && (ordinal - second_start).is_multiple_of(step)
}

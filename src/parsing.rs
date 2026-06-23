use winnow::ascii::{alpha1, digit1, multispace0};
use winnow::combinator::{
    alt, delimited, eof, opt, preceded, separated, separated_pair, terminated,
};
use winnow::prelude::*;

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::str::{self, FromStr};

use crate::config::{CronScheduleParts, DayOfWeekNumbering, DowDomOperand};
use crate::error::{Error, ErrorKind};
use crate::ordinal::*;
use crate::schedule::{Schedule, ScheduleFields};
use crate::specifier::*;
use crate::time_unit::*;
use crate::ScheduleConfig;

impl TryFrom<Cow<'_, str>> for Schedule {
    type Error = Error;

    fn try_from(expression: Cow<'_, str>) -> Result<Self, Self::Error> {
        Self::from_str_with_config(expression.as_ref(), ScheduleConfig::default())
    }
}

impl Schedule {
    /// Parse a cron expression using the supplied [ScheduleConfig].
    pub fn from_str_with_config(expression: &str, config: ScheduleConfig) -> Result<Self, Error> {
        match schedule_with_config(expression, config) {
            Ok(schedule_fields) => Ok(Schedule::new(
                expression.to_owned(),
                schedule_fields,
                config,
            )),
            Err(parse_error) => Err(ErrorKind::Expression(parse_error.to_string()).into()),
        }
    }
}

impl TryFrom<String> for Schedule {
    type Error = Error;

    fn try_from(expression: String) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Owned(expression))
    }
}

impl TryFrom<&str> for Schedule {
    type Error = Error;

    fn try_from(expression: &str) -> Result<Self, Self::Error> {
        Self::try_from(Cow::Borrowed(expression))
    }
}

impl FromStr for Schedule {
    type Err = Error;

    fn from_str(expression: &str) -> Result<Self, Self::Err> {
        Self::try_from(Cow::Borrowed(expression))
    }
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub specifiers: Vec<RootSpecifier>, // TODO: expose iterator?
}

fn ordinal(i: &mut &str) -> winnow::Result<u32> {
    delimited(multispace0, digit1, multispace0)
        .try_map(u32::from_str)
        .parse_next(i)
}

fn name(i: &mut &str) -> winnow::Result<String> {
    delimited(multispace0, alpha1, multispace0)
        .map(ToOwned::to_owned)
        .parse_next(i)
}

fn point(i: &mut &str) -> winnow::Result<Specifier> {
    ordinal.map(Specifier::Point).parse_next(i)
}

fn range_endpoint(i: &mut &str) -> winnow::Result<RangeEndpoint> {
    alt((
        ordinal.map(RangeEndpoint::Ordinal),
        name.map(RangeEndpoint::Name),
    ))
    .parse_next(i)
}

fn named_point(i: &mut &str) -> winnow::Result<RootSpecifier> {
    name.map(RootSpecifier::NamedPoint).parse_next(i)
}

fn literal_l(i: &mut &str) -> winnow::Result<()> {
    alt(("L", "l")).map(|_| ()).parse_next(i)
}

fn raw_ordinal(i: &mut &str) -> winnow::Result<Ordinal> {
    digit1.try_map(u32::from_str).parse_next(i)
}

fn raw_name(i: &mut &str) -> winnow::Result<String> {
    alpha1.map(ToOwned::to_owned).parse_next(i)
}

fn raw_range_endpoint(i: &mut &str) -> winnow::Result<RangeEndpoint> {
    alt((
        raw_ordinal.map(RangeEndpoint::Ordinal),
        raw_name.map(RangeEndpoint::Name),
    ))
    .parse_next(i)
}

fn raw_range_endpoint_pair(i: &mut &str) -> winnow::Result<(RangeEndpoint, RangeEndpoint)> {
    separated_pair(raw_range_endpoint, "-", raw_range_endpoint).parse_next(i)
}

fn nearest_weekday(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        terminated(raw_ordinal, alt(("W", "w"))),
        preceded(alt(("W", "w")), raw_ordinal),
    ))
    .map(RootSpecifier::NearestWeekday)
    .parse_next(i)
}

fn last_weekday_of_month(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        preceded(literal_l, raw_range_endpoint),
        terminated(raw_range_endpoint, literal_l),
        literal_l.map(|_| RangeEndpoint::Name("sat".to_owned())),
    ))
    .map(RootSpecifier::LastWeekdayOfMonth)
    .parse_next(i)
}

fn nth_weekday_of_month(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        separated_pair(raw_range_endpoint_pair, "#", raw_ordinal)
            .map(|((start, end), nth)| RootSpecifier::NthWeekdayRangeOfMonth(start, end, nth)),
        separated_pair(raw_range_endpoint, "#", raw_ordinal)
            .map(|(day, nth)| RootSpecifier::NthWeekdayOfMonth(day, nth)),
    ))
    .parse_next(i)
}

fn random_specifier(i: &mut &str) -> winnow::Result<RootSpecifier> {
    (
        alt(("R", "r")),
        opt(delimited(
            "(",
            separated_pair(raw_ordinal, "-", raw_ordinal),
            ")",
        )),
        opt(preceded("/", raw_ordinal)),
    )
        .map(|(_, range, step)| RootSpecifier::Random(RandomSpecifier { range, step }))
        .parse_next(i)
}

fn period(i: &mut &str) -> winnow::Result<RootSpecifier> {
    separated_pair(specifier, "/", ordinal)
        .map(|(start, step)| RootSpecifier::Period(start, step))
        .parse_next(i)
}

fn period_with_any(i: &mut &str) -> winnow::Result<RootSpecifier> {
    separated_pair(specifier_with_any, "/", ordinal)
        .map(|(start, step)| RootSpecifier::Period(start, step))
        .parse_next(i)
}

fn range(i: &mut &str) -> winnow::Result<Specifier> {
    separated_pair(range_endpoint, "-", range_endpoint)
        .map(|(start, end)| Specifier::Range(start, end))
        .parse_next(i)
}

#[cfg(test)]
fn named_range(i: &mut &str) -> winnow::Result<Specifier> {
    separated_pair(name, "-", name)
        .map(|(start, end)| Specifier::Range(RangeEndpoint::Name(start), RangeEndpoint::Name(end)))
        .parse_next(i)
}

fn all(i: &mut &str) -> winnow::Result<Specifier> {
    "*".map(|_| Specifier::All).parse_next(i)
}

fn any(i: &mut &str) -> winnow::Result<Specifier> {
    "?".map(|_| Specifier::All).parse_next(i)
}

fn specifier(i: &mut &str) -> winnow::Result<Specifier> {
    alt((all, range, point)).parse_next(i)
}

fn specifier_with_any(i: &mut &str) -> winnow::Result<Specifier> {
    alt((any, specifier)).parse_next(i)
}

fn root_specifier(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        period,
        random_specifier,
        specifier.map(RootSpecifier::from),
        named_point,
    ))
    .parse_next(i)
}

fn dom_root_specifier_with_any(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        nearest_weekday,
        literal_l.map(|_| RootSpecifier::LastDayOfMonth),
        period_with_any,
        random_specifier,
        specifier_with_any.map(RootSpecifier::from),
        named_point,
    ))
    .parse_next(i)
}

fn dow_root_specifier_with_any(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        last_weekday_of_month,
        nth_weekday_of_month,
        period_with_any,
        random_specifier,
        specifier_with_any.map(RootSpecifier::from),
        named_point,
    ))
    .parse_next(i)
}

fn root_specifier_list(i: &mut &str) -> winnow::Result<Vec<RootSpecifier>> {
    let list = separated(1.., root_specifier, ",");
    let single_item = root_specifier.map(|spec| vec![spec]);
    delimited(multispace0, alt((list, single_item)), multispace0).parse_next(i)
}

fn dom_root_specifier_list_with_any(i: &mut &str) -> winnow::Result<Vec<RootSpecifier>> {
    let list = separated(1.., dom_root_specifier_with_any, ",");
    let single_item = dom_root_specifier_with_any.map(|spec| vec![spec]);
    delimited(multispace0, alt((list, single_item)), multispace0).parse_next(i)
}

fn dow_root_specifier_list_with_any(i: &mut &str) -> winnow::Result<Vec<RootSpecifier>> {
    let list = separated(1.., dow_root_specifier_with_any, ",");
    let single_item = dow_root_specifier_with_any.map(|spec| vec![spec]);
    delimited(multispace0, alt((list, single_item)), multispace0).parse_next(i)
}

fn field(i: &mut &str) -> winnow::Result<Field> {
    let specifiers = root_specifier_list.parse_next(i)?;
    Ok(Field { specifiers })
}

fn dom_field_with_any(i: &mut &str) -> winnow::Result<Field> {
    let specifiers = dom_root_specifier_list_with_any.parse_next(i)?;
    Ok(Field { specifiers })
}

fn dow_field_with_any(i: &mut &str) -> winnow::Result<Field> {
    let specifiers = dow_root_specifier_list_with_any.parse_next(i)?;
    Ok(Field { specifiers })
}

fn shorthand_yearly(i: &mut &str) -> winnow::Result<ScheduleFields> {
    "@yearly".parse_next(i)?;
    let fields = ScheduleFields::new(
        Seconds::from_ordinal(0),
        Minutes::from_ordinal(0),
        Hours::from_ordinal(0),
        DaysOfMonth::from_ordinal(1),
        Months::from_ordinal(1),
        DaysOfWeek::all(),
        Years::all(),
    );
    Ok(fields)
}

fn shorthand_monthly(i: &mut &str) -> winnow::Result<ScheduleFields> {
    "@monthly".parse_next(i)?;
    let fields = ScheduleFields::new(
        Seconds::from_ordinal(0),
        Minutes::from_ordinal(0),
        Hours::from_ordinal(0),
        DaysOfMonth::from_ordinal(1),
        Months::all(),
        DaysOfWeek::all(),
        Years::all(),
    );
    Ok(fields)
}

fn shorthand_weekly(i: &mut &str) -> winnow::Result<ScheduleFields> {
    "@weekly".parse_next(i)?;
    let fields = ScheduleFields::new(
        Seconds::from_ordinal(0),
        Minutes::from_ordinal(0),
        Hours::from_ordinal(0),
        DaysOfMonth::all(),
        Months::all(),
        DaysOfWeek::from_ordinal(1),
        Years::all(),
    );
    Ok(fields)
}

fn shorthand_daily(i: &mut &str) -> winnow::Result<ScheduleFields> {
    "@daily".parse_next(i)?;
    let fields = ScheduleFields::new(
        Seconds::from_ordinal(0),
        Minutes::from_ordinal(0),
        Hours::from_ordinal(0),
        DaysOfMonth::all(),
        Months::all(),
        DaysOfWeek::all(),
        Years::all(),
    );
    Ok(fields)
}

fn shorthand_hourly(i: &mut &str) -> winnow::Result<ScheduleFields> {
    "@hourly".parse_next(i)?;
    let fields = ScheduleFields::new(
        Seconds::from_ordinal(0),
        Minutes::from_ordinal(0),
        Hours::all(),
        DaysOfMonth::all(),
        Months::all(),
        DaysOfWeek::all(),
        Years::all(),
    );
    Ok(fields)
}

fn shorthand(i: &mut &str) -> winnow::Result<ScheduleFields> {
    let keywords = alt((
        shorthand_yearly,
        shorthand_monthly,
        shorthand_weekly,
        shorthand_daily,
        shorthand_hourly,
    ));
    delimited(multispace0, keywords, multispace0).parse_next(i)
}

fn parse_field_token(token: &str) -> Result<Field, String> {
    terminated(field, eof)
        .parse(token)
        .map_err(|parse_error| format!("{parse_error}"))
}

fn parse_dom_field_with_any_token(token: &str) -> Result<Field, String> {
    terminated(dom_field_with_any, eof)
        .parse(token)
        .map_err(|parse_error| format!("{parse_error}"))
}

fn parse_dow_field_with_any_token(token: &str) -> Result<Field, String> {
    terminated(dow_field_with_any, eof)
        .parse(token)
        .map_err(|parse_error| format!("{parse_error}"))
}

type RandomFieldBounds = (Ordinal, Ordinal);

fn day_of_week_random_bounds(config: ScheduleConfig) -> RandomFieldBounds {
    match config.day_of_week_numbering {
        DayOfWeekNumbering::OneIndexed => random_bounds::<DaysOfWeek>(),
        DayOfWeekNumbering::ZeroIndexed => (0, 6),
    }
}

fn random_bounds<T>() -> RandomFieldBounds
where
    T: TimeUnitField,
{
    (T::inclusive_min(), T::inclusive_max())
}

fn resolve_random_specifier(
    random: RandomSpecifier,
    config: ScheduleConfig,
    bounds: RandomFieldBounds,
) -> Result<RootSpecifier, Error> {
    ensure_enabled(config.random_fields, "random field")?;

    let range = random.range.unwrap_or(bounds);
    if range.0 >= range.1 {
        return Err(ErrorKind::Expression(
            "random range end must be greater than range begin".into(),
        )
        .into());
    }
    if range.0 < bounds.0 || range.1 > bounds.1 {
        return Err(ErrorKind::Expression(format!(
            "random range must be between {} and {}",
            bounds.0, bounds.1
        ))
        .into());
    }

    match random.step {
        Some(0) => Err(ErrorKind::Expression("Bad random expression".into()).into()),
        Some(step) => {
            let random_end = range.0.checked_add(step - 1).ok_or_else(|| {
                Error::from(ErrorKind::Expression("Bad random expression".into()))
            })?;
            if random_end > range.1 {
                return Err(ErrorKind::Expression("Bad random expression".into()).into());
            }
            let start = fastrand::u32(range.0..=random_end);
            Ok(RootSpecifier::Period(
                Specifier::Range(
                    RangeEndpoint::Ordinal(start),
                    RangeEndpoint::Ordinal(range.1),
                ),
                step,
            ))
        }
        None => Ok(RootSpecifier::from(Specifier::Point(fastrand::u32(
            range.0..=range.1,
        )))),
    }
}

fn resolve_random_root_specifier(
    specifier: RootSpecifier,
    config: ScheduleConfig,
    bounds: RandomFieldBounds,
) -> Result<RootSpecifier, Error> {
    match specifier {
        RootSpecifier::Random(random) => resolve_random_specifier(random, config, bounds),
        specifier => Ok(specifier),
    }
}

fn from_field_with_options<T>(
    field: Field,
    config: ScheduleConfig,
    bounds: RandomFieldBounds,
) -> Result<T, Error>
where
    T: TimeUnitField,
{
    if field.specifiers.len() == 1
        && field.specifiers.first().unwrap() == &RootSpecifier::from(Specifier::All)
    {
        return Ok(T::all());
    }

    let mut ordinals = OrdinalSet::empty(T::inclusive_min(), T::inclusive_max());
    for specifier in field.specifiers {
        let specifier = resolve_random_root_specifier(specifier, config, bounds)?;
        let specifier_ordinals =
            T::ordinals_from_root_specifier_with_options(&specifier, config.wraparound_ranges)?;
        for ordinal in &specifier_ordinals {
            ordinals.insert(T::validate_ordinal(ordinal)?);
        }
    }
    Ok(T::from_ordinal_set(ordinals))
}

fn parse_field_as<T>(
    token: &str,
    config: ScheduleConfig,
    bounds: RandomFieldBounds,
) -> Result<T, String>
where
    T: TimeUnitField,
{
    from_field_with_options(parse_field_token(token)?, config, bounds)
        .map_err(|parse_error| format!("{parse_error}"))
}

fn ensure_enabled(enabled: bool, feature: &str) -> Result<(), Error> {
    if enabled {
        Ok(())
    } else {
        Err(ErrorKind::Expression(format!("{feature} specifiers are not enabled")).into())
    }
}

fn root_specifier_uses_last_day(specifier: &RootSpecifier) -> bool {
    let is_last = |endpoint: &RangeEndpoint| matches!(endpoint, RangeEndpoint::Name(name) if name.eq_ignore_ascii_case("l"));
    match specifier {
        RootSpecifier::Specifier(Specifier::Range(start, end))
        | RootSpecifier::Period(Specifier::Range(start, end), _) => is_last(start) || is_last(end),
        RootSpecifier::NamedPoint(name) => name.eq_ignore_ascii_case("l"),
        _ => false,
    }
}

fn parse_days_of_month_as(token: &str, config: ScheduleConfig) -> Result<DaysOfMonth, String> {
    days_of_month_from_field(parse_dom_field_with_any_token(token)?, config)
        .map_err(|e| e.to_string())
}

fn parse_days_of_week_as(token: &str, config: ScheduleConfig) -> Result<DaysOfWeek, String> {
    days_of_week_from_field(parse_dow_field_with_any_token(token)?, config)
        .map_err(|parse_error| format!("{parse_error}"))
}

fn build_schedule_fields(
    [seconds, minutes, hours, days_of_month, months, days_of_week, years]: [&str; 7],
    config: ScheduleConfig,
) -> Result<ScheduleFields, String> {
    Ok(ScheduleFields::new(
        parse_field_as(seconds, config, random_bounds::<Seconds>())?,
        parse_field_as(minutes, config, random_bounds::<Minutes>())?,
        parse_field_as(hours, config, random_bounds::<Hours>())?,
        parse_days_of_month_as(days_of_month, config)?,
        parse_field_as(months, config, random_bounds::<Months>())?,
        parse_days_of_week_as(days_of_week, config)?,
        parse_years_as(years, config)?,
    ))
}

fn build_schedule_fields_from_six_part_tokens(
    tokens: &[&str],
    config: ScheduleConfig,
) -> Result<ScheduleFields, String> {
    match tokens {
        [seconds, minutes, hours, days_of_month, months, days_of_week] => build_schedule_fields(
            [
                seconds,
                minutes,
                hours,
                days_of_month,
                months,
                days_of_week,
                "*",
            ],
            config,
        ),
        _ => Err("a valid cron expression".to_owned()),
    }
}

fn build_schedule_fields_from_seven_part_tokens(
    tokens: &[&str],
    config: ScheduleConfig,
) -> Result<ScheduleFields, String> {
    match tokens {
        [seconds, minutes, hours, days_of_month, months, days_of_week, years] => {
            build_schedule_fields(
                [
                    seconds,
                    minutes,
                    hours,
                    days_of_month,
                    months,
                    days_of_week,
                    years,
                ],
                config,
            )
        }
        _ => Err("a valid cron expression".to_owned()),
    }
}

fn days_of_month_from_field(field: Field, config: ScheduleConfig) -> Result<DaysOfMonth, Error> {
    if field.specifiers.len() == 1
        && field.specifiers.first().unwrap() == &RootSpecifier::from(Specifier::All)
    {
        return Ok(DaysOfMonth::all());
    }

    let mut ordinals =
        OrdinalSet::empty(DaysOfMonth::inclusive_min(), DaysOfMonth::inclusive_max());
    let mut last_day_of_month = false;
    let mut nearest_weekdays =
        OrdinalSet::empty(DaysOfMonth::inclusive_min(), DaysOfMonth::inclusive_max());

    for specifier in field.specifiers {
        let specifier =
            resolve_random_root_specifier(specifier, config, random_bounds::<DaysOfMonth>())?;
        match specifier {
            RootSpecifier::LastDayOfMonth => {
                ensure_enabled(config.last_specifiers, "last day-of-month")?;
                last_day_of_month = true;
            }
            RootSpecifier::NearestWeekday(day) => {
                ensure_enabled(config.nearest_weekday, "nearest weekday")?;
                nearest_weekdays.insert(DaysOfMonth::validate_ordinal(day)?);
            }
            specifier => {
                if root_specifier_uses_last_day(&specifier) {
                    ensure_enabled(config.last_specifiers, "last day-of-month")?;
                }
                let specifier_ordinals = DaysOfMonth::ordinals_from_root_specifier_with_options(
                    &specifier,
                    config.wraparound_ranges,
                )?;
                for ordinal in &specifier_ordinals {
                    ordinals.insert(DaysOfMonth::validate_ordinal(ordinal)?);
                }
            }
        }
    }

    Ok(DaysOfMonth::from_parts(
        ordinals,
        last_day_of_month,
        nearest_weekdays,
    ))
}

fn zero_indexed_day_of_week_from_numeric(ordinal: Ordinal) -> Result<Ordinal, Error> {
    match ordinal {
        0 | 7 => Ok(0),
        1..=6 => Ok(ordinal),
        _ => Err(ErrorKind::Expression(format!(
            "Days of Week must be between 0 and 7. ('{}' specified.)",
            ordinal
        ))
        .into()),
    }
}

fn zero_indexed_day_of_week_to_internal_ordinal(ordinal: Ordinal) -> Ordinal {
    debug_assert!(ordinal <= 6);
    ordinal + 1
}

fn zero_indexed_day_of_week_from_name(name: &str) -> Result<Ordinal, Error> {
    let internal_ordinal = DaysOfWeek::ordinal_from_name(name)?;
    debug_assert!((1..=7).contains(&internal_ordinal));
    // The shared day-name map uses the crate's Sunday=1 internal ordinals.
    // Vixie range expansion uses Sunday=0, so decrement named weekdays into that space.
    Ok(internal_ordinal - 1)
}

fn zero_indexed_day_of_week_from_endpoint(endpoint: &RangeEndpoint) -> Result<Ordinal, Error> {
    match endpoint {
        RangeEndpoint::Ordinal(ordinal) => zero_indexed_day_of_week_from_numeric(*ordinal),
        RangeEndpoint::Name(name) => zero_indexed_day_of_week_from_name(name),
    }
}

fn zero_indexed_day_of_week_values_from_specifier(
    specifier: &Specifier,
    wraparound_ranges: bool,
) -> Result<Vec<Ordinal>, Error> {
    match specifier {
        Specifier::All => Ok((0..=6).collect()),
        Specifier::Point(ordinal) => Ok(vec![zero_indexed_day_of_week_from_numeric(*ordinal)?]),
        Specifier::Range(start, end) => {
            let start_ordinal = zero_indexed_day_of_week_from_endpoint(start)?;
            let end_ordinal = zero_indexed_day_of_week_from_endpoint(end)?;
            ordinal_range_values(start_ordinal, end_ordinal, 0, 6, wraparound_ranges).ok_or_else(
                || {
                    ErrorKind::Expression(format!(
                        "Invalid range for Days of Week: {}-{}",
                        start, end
                    ))
                    .into()
                },
            )
        }
    }
}

fn zero_indexed_day_of_week_internal_ordinals_from_root_specifier(
    root_specifier: &RootSpecifier,
    wraparound_ranges: bool,
) -> Result<OrdinalSet, Error> {
    let ordinals = match root_specifier {
        RootSpecifier::Specifier(specifier) => {
            zero_indexed_day_of_week_values_from_specifier(specifier, wraparound_ranges)?
        }
        RootSpecifier::Period(_, 0) => Err(ErrorKind::Expression(
            "range step cannot be zero".to_string(),
        ))?,
        RootSpecifier::Period(start, step) => {
            if *step < 1 || *step > 7 {
                return Err(ErrorKind::Expression(format!(
                    "Days of Week must be between 1 and 7. ('{}' specified.)",
                    step,
                ))
                .into());
            }

            let base_values = match start {
                Specifier::Point(start) => {
                    let start = zero_indexed_day_of_week_from_numeric(*start)?;
                    (start..=6).collect()
                }
                Specifier::Range(start, end) => {
                    let start_ordinal = zero_indexed_day_of_week_from_endpoint(start)?;
                    let end_ordinal = zero_indexed_day_of_week_from_endpoint(end)?;
                    return ordinal_range_values_with_step(
                        start_ordinal,
                        end_ordinal,
                        0,
                        6,
                        wraparound_ranges,
                        *step,
                    )
                    .map(|ordinals| {
                        OrdinalSet::from_values(
                            DaysOfWeek::inclusive_min(),
                            DaysOfWeek::inclusive_max(),
                            ordinals
                                .into_iter()
                                .map(zero_indexed_day_of_week_to_internal_ordinal),
                        )
                    })
                    .ok_or_else(|| {
                        Error::from(ErrorKind::Expression(format!(
                            "Invalid range for Days of Week: {}-{}",
                            start, end
                        )))
                    });
                }
                specifier => {
                    zero_indexed_day_of_week_values_from_specifier(specifier, wraparound_ranges)?
                }
            };
            base_values.into_iter().step_by(*step as usize).collect()
        }
        RootSpecifier::NamedPoint(name) => vec![zero_indexed_day_of_week_from_name(name)?],
        _ => {
            return Err(ErrorKind::Expression(format!(
                "Root specifier not supported for Days of Week: {:?}",
                root_specifier
            ))
            .into())
        }
    };

    Ok(OrdinalSet::from_values(
        DaysOfWeek::inclusive_min(),
        DaysOfWeek::inclusive_max(),
        ordinals
            .into_iter()
            .map(zero_indexed_day_of_week_to_internal_ordinal),
    ))
}

fn day_of_week_from_endpoint(
    endpoint: &RangeEndpoint,
    config: ScheduleConfig,
) -> Result<Ordinal, Error> {
    match config.day_of_week_numbering {
        DayOfWeekNumbering::OneIndexed => {
            DaysOfWeek::validate_ordinal(DaysOfWeek::ordinal_from_range_endpoint(endpoint)?)
        }
        DayOfWeekNumbering::ZeroIndexed => Ok(zero_indexed_day_of_week_to_internal_ordinal(
            zero_indexed_day_of_week_from_endpoint(endpoint)?,
        )),
    }
}

fn day_of_week_values_from_range(
    start: &RangeEndpoint,
    end: &RangeEndpoint,
    config: ScheduleConfig,
) -> Result<Vec<Ordinal>, Error> {
    match config.day_of_week_numbering {
        DayOfWeekNumbering::OneIndexed => {
            let start_ordinal =
                DaysOfWeek::validate_ordinal(DaysOfWeek::ordinal_from_range_endpoint(start)?)?;
            let end_ordinal =
                DaysOfWeek::validate_ordinal(DaysOfWeek::ordinal_from_range_endpoint(end)?)?;
            ordinal_range_values(
                start_ordinal,
                end_ordinal,
                DaysOfWeek::inclusive_min(),
                DaysOfWeek::inclusive_max(),
                config.wraparound_ranges,
            )
            .ok_or_else(|| {
                ErrorKind::Expression(format!("Invalid range for Days of Week: {start}-{end}"))
                    .into()
            })
        }
        DayOfWeekNumbering::ZeroIndexed => {
            let start_ordinal = zero_indexed_day_of_week_from_endpoint(start)?;
            let end_ordinal = zero_indexed_day_of_week_from_endpoint(end)?;
            ordinal_range_values(start_ordinal, end_ordinal, 0, 6, config.wraparound_ranges)
                .map(|ordinals| {
                    ordinals
                        .into_iter()
                        .map(zero_indexed_day_of_week_to_internal_ordinal)
                        .collect()
                })
                .ok_or_else(|| {
                    ErrorKind::Expression(format!("Invalid range for Days of Week: {start}-{end}"))
                        .into()
                })
        }
    }
}

fn insert_nth_weekday(
    nth_weekdays: &mut BTreeMap<Ordinal, OrdinalSet>,
    day_of_week: Ordinal,
    occurrence: Ordinal,
) -> Result<(), Error> {
    if !(1..=5).contains(&occurrence) {
        return Err(ErrorKind::Expression(format!(
            "Occurrence of a weekday must be between 1 and 5 inclusive. ('{}' specified.)",
            occurrence
        ))
        .into());
    }
    nth_weekdays
        .entry(day_of_week)
        .or_insert_with(|| OrdinalSet::empty(1, 5))
        .insert(occurrence);
    Ok(())
}

fn days_of_week_from_field(field: Field, config: ScheduleConfig) -> Result<DaysOfWeek, Error> {
    if field.specifiers.len() == 1
        && field.specifiers.first().unwrap() == &RootSpecifier::from(Specifier::All)
    {
        return Ok(DaysOfWeek::all());
    }

    let mut ordinals = OrdinalSet::empty(DaysOfWeek::inclusive_min(), DaysOfWeek::inclusive_max());
    let mut last_weekdays =
        OrdinalSet::empty(DaysOfWeek::inclusive_min(), DaysOfWeek::inclusive_max());
    let mut nth_weekdays = BTreeMap::new();

    for specifier in field.specifiers {
        let specifier =
            resolve_random_root_specifier(specifier, config, day_of_week_random_bounds(config))?;
        match specifier {
            RootSpecifier::LastWeekdayOfMonth(day_of_week) => {
                ensure_enabled(config.last_specifiers, "last weekday-of-month")?;
                last_weekdays.insert(day_of_week_from_endpoint(&day_of_week, config)?);
            }
            RootSpecifier::NthWeekdayOfMonth(day_of_week, occurrence) => {
                ensure_enabled(config.nth_weekday_of_month, "nth weekday-of-month")?;
                let day_of_week = day_of_week_from_endpoint(&day_of_week, config)?;
                insert_nth_weekday(&mut nth_weekdays, day_of_week, occurrence)?;
            }
            RootSpecifier::NthWeekdayRangeOfMonth(start, end, occurrence) => {
                ensure_enabled(config.nth_weekday_of_month, "nth weekday-of-month")?;
                for day_of_week in day_of_week_values_from_range(&start, &end, config)? {
                    insert_nth_weekday(&mut nth_weekdays, day_of_week, occurrence)?;
                }
            }
            specifier => {
                let specifier_ordinals =
                    if config.day_of_week_numbering == DayOfWeekNumbering::OneIndexed {
                        DaysOfWeek::ordinals_from_root_specifier_with_options(
                            &specifier,
                            config.wraparound_ranges,
                        )?
                    } else {
                        zero_indexed_day_of_week_internal_ordinals_from_root_specifier(
                            &specifier,
                            config.wraparound_ranges,
                        )?
                    };
                for ordinal in &specifier_ordinals {
                    ordinals.insert(DaysOfWeek::validate_ordinal(ordinal)?);
                }
            }
        }
    }

    Ok(DaysOfWeek::from_parts(
        ordinals,
        last_weekdays,
        nth_weekdays,
    ))
}

fn years_from_field(
    field: Field,
    config: ScheduleConfig,
    bounds: RandomFieldBounds,
) -> Result<Years, Error> {
    let specifiers = field
        .specifiers
        .into_iter()
        .map(|specifier| resolve_random_root_specifier(specifier, config, bounds))
        .collect::<Result<Vec<_>, _>>()?;
    Years::from_root_specifiers(specifiers, config.wraparound_ranges)
}

fn parse_years_as(token: &str, config: ScheduleConfig) -> Result<Years, String> {
    years_from_field(parse_field_token(token)?, config, random_bounds::<Years>())
        .map_err(|parse_error| format!("{parse_error}"))
}

fn build_schedule_fields_from_five_part_tokens(
    tokens: &[&str],
    config: ScheduleConfig,
) -> Result<ScheduleFields, String> {
    match tokens {
        [minutes, hours, days_of_month, months, days_of_week] => build_schedule_fields(
            [
                "0",
                minutes,
                hours,
                days_of_month,
                months,
                days_of_week,
                "*",
            ],
            config,
        ),
        _ => Err("a valid cron expression".to_owned()),
    }
}

fn validate_day_of_month(fields: &ScheduleFields, config: ScheduleConfig) -> Result<(), String> {
    if fields.days_of_month_is_all() || fields.days_of_month_has_special_specifiers() {
        return Ok(());
    }
    if config.dow_dom_operand == DowDomOperand::Or && !fields.days_of_week_is_all() {
        return Ok(());
    }

    let has_valid_date = fields.months_ordinals().iter().any(|month| {
        // Leap-year baseline allows Feb 29 without checking whether selected years contain one.
        fields
            .days_of_month_ordinals()
            .iter()
            .any(|day| day <= days_in_month(month, 2024))
    });

    if has_valid_date {
        Ok(())
    } else {
        Err("day-of-month values do not occur in the selected month/year set".to_owned())
    }
}

fn schedule_with_config(
    expression: &str,
    config: ScheduleConfig,
) -> Result<ScheduleFields, String> {
    if let Ok(fields) = terminated(shorthand, eof).parse(expression) {
        return Ok(fields);
    }

    let tokens = expression.split_whitespace().collect::<Vec<_>>();
    if tokens.is_empty() {
        return Err("a valid cron expression".to_owned());
    }

    let fields = match (tokens.len(), config.cron_schedule_parts) {
        (5, CronScheduleParts::Five | CronScheduleParts::FiveOrSix | CronScheduleParts::All) => {
            build_schedule_fields_from_five_part_tokens(tokens.as_slice(), config)
        }
        (
            6,
            CronScheduleParts::Six
            | CronScheduleParts::FiveOrSix
            | CronScheduleParts::SixOrSeven
            | CronScheduleParts::All,
        ) => build_schedule_fields_from_six_part_tokens(tokens.as_slice(), config),
        (7, CronScheduleParts::Seven | CronScheduleParts::SixOrSeven | CronScheduleParts::All) => {
            build_schedule_fields_from_seven_part_tokens(tokens.as_slice(), config)
        }
        _ => Err("a valid cron expression".to_owned()),
    }?;
    validate_day_of_month(&fields, config)?;
    Ok(fields)
}

#[cfg(test)]
mod test {
    use super::*;

    fn parse_schedule(expression: &str) -> Result<Schedule, Error> {
        Schedule::from_str(expression)
    }

    #[test]
    fn test_nom_valid_number() {
        let expression = "1997";
        point.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_point() {
        let expression = "a";
        assert!(point.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_named_point() {
        let expression = "WED";
        named_point.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_point() {
        let expression = "8";
        assert!(named_point.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_period() {
        let expression = "1/2";
        period.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_period() {
        let expression = "Wed/4";
        assert!(period.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_number_list() {
        let expression = "1,2";
        field.parse(expression).unwrap();
        dom_field_with_any.parse(expression).unwrap();
        dow_field_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_number_list() {
        let expression = ",1,2";
        assert!(field.parse(expression).is_err());
        assert!(dom_field_with_any.parse(expression).is_err());
        assert!(dow_field_with_any.parse(expression).is_err());
    }

    #[test]
    fn test_nom_dom_and_dow_fields_valid_any() {
        let expression = "?";
        dom_field_with_any.parse(expression).unwrap();
        dow_field_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_field_invalid_any() {
        let expression = "?";
        assert!(field.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_range_field() {
        let expression = "1-4";
        range.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_all() {
        let expression = "*/2";
        period.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_range() {
        let expression = "10-20/2";
        period.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_named_range() {
        let expression = "Mon-Thurs/2";
        period.parse(expression).unwrap();

        let expression = "February-November/2";
        period.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_point() {
        let expression = "10/2";
        period.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_period_any() {
        let expression = "?/2";
        assert!(period.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_named_point() {
        let expression = "Tues/2";
        assert!(period.parse(expression).is_err());

        let expression = "February/2";
        assert!(period.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_specifier_range() {
        let expression = "10-12/*";
        assert!(period.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_period_with_any_all() {
        let expression = "*/2";
        period_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_range() {
        let expression = "10-20/2";
        period_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_named_range() {
        let expression = "Mon-Thurs/2";
        period_with_any.parse(expression).unwrap();

        let expression = "February-November/2";
        period_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_point() {
        let expression = "10/2";
        period_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_any() {
        let expression = "?/2";
        period_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_period_with_any_named_point() {
        let expression = "Tues/2";
        assert!(period_with_any.parse(expression).is_err());

        let expression = "February/2";
        assert!(period_with_any.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_any_specifier_range() {
        let expression = "10-12/*";
        assert!(period_with_any.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_range_field() {
        let expression = "-4";
        assert!(range.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_named_range_field() {
        let expression = "TUES-THURS";
        named_range.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_range_field() {
        let expression = "3-THURS";
        assert!(named_range.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_schedule() {
        let expression = "* * * * * *";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_schedule() {
        let expression = "* * * *";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_seconds_list() {
        let expression = "0,20,40 * * * * *";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_range() {
        let expression = "0-40 * * * * *";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_mix() {
        let expression = "0-5,58 * * * * *";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_seconds_range() {
        let expression = "0-65 * * * * *";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_list() {
        let expression = "103,12 * * * * *";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_mix() {
        let expression = "0-5,102 * * * * *";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_list() {
        let expression = "* * * * * MON,WED,FRI";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_list() {
        let expression = "* * * * * MON,TURTLE";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_range() {
        let expression = "* * * * * MON-FRI";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_range() {
        let expression = "* * * * * BEAR-OWL";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_range_specifier() {
        let expression = "10-12/10-12 * * * * ?";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_month_any() {
        let expression = "* * * ? * *";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any() {
        let expression = "* * * * * ?";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_month_any_days_of_week_specific() {
        let expression = "* * * ? * Mon,Thu";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any_days_of_month_specific() {
        let expression = "* * * 1,2 * ?";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_dom_and_dow_any() {
        let expression = "* * * ? * ?";
        parse_schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_other_fields_any() {
        let expression = "? * * * * *";
        assert!(parse_schedule(expression).is_err());

        let expression = "* ? * * * *";
        assert!(parse_schedule(expression).is_err());

        let expression = "* * ? * * *";
        assert!(parse_schedule(expression).is_err());

        let expression = "* * * * ? *";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_trailing_characters() {
        let expression = "* * * * * *foo *";
        assert!(parse_schedule(expression).is_err());

        let expression = "* * * * * * * foo";
        assert!(parse_schedule(expression).is_err());
    }

    /// Issue #86
    #[test]
    fn shorthand_must_match_whole_input() {
        let expression = "@dailyBla";
        assert!(parse_schedule(expression).is_err());
        let expression = " @dailyBla ";
        assert!(parse_schedule(expression).is_err());
    }

    #[test]
    fn test_try_from_cow_str_owned() {
        let expression = Cow::Owned(String::from("* * * ? * ?"));
        Schedule::try_from(expression).unwrap();
    }

    #[test]
    fn test_try_from_cow_str_borrowed() {
        let expression = Cow::Borrowed("* * * ? * ?");
        Schedule::try_from(expression).unwrap();
    }

    #[test]
    fn test_try_from_string() {
        let expression = String::from("* * * ? * ?");
        Schedule::try_from(expression).unwrap();
    }

    #[test]
    fn test_try_from_str() {
        let expression = "* * * ? * ?";
        Schedule::try_from(expression).unwrap();
    }

    #[test]
    fn test_from_str() {
        let expression = "* * * ? * ?";
        Schedule::from_str(expression).unwrap();
    }

    /// Issue #59
    #[test]
    fn test_reject_invalid_interval() {
        for invalid_expression in [
            "1-5/61 * * * * *",
            "*/61 2 3 4 5 6",
            "* */61 * * * *",
            "* * */25 * * *",
            "* * * */32 * *",
            "* * * * */13 *",
            "1,2,3/60 * * * * *",
        ] {
            assert!(parse_schedule(invalid_expression).is_err());
        }

        for valid_expression in [
            "1-5/59 * * * * *",
            "*/10 2 3 4 5 6",
            "* */30 * * * *",
            "* * */23 * * *",
            "* * * */30 * *",
            "* * * * */10 *",
            "1,2,3/5 * * * * *",
            "0 0 0 1 1 ? 2020-2040/10",
            "0 0 0 1 1 ? 2020-2040/2200",
        ] {
            assert!(parse_schedule(valid_expression).is_ok());
        }
    }
}

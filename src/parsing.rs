use winnow::ascii::{alpha1, digit1, multispace0};
use winnow::combinator::{
    alt, delimited, eof, opt, preceded, separated, separated_pair, terminated,
};
use winnow::prelude::*;
use winnow::stream::AsChar;
use winnow::token::take_while;

use std::borrow::Cow;
use std::convert::TryFrom;
use std::str::{self, FromStr};

use crate::error::{Error, ErrorKind};
use crate::ordinal::*;
use crate::schedule::{Schedule, ScheduleFields};
use crate::specifier::*;
use crate::time_unit::*;

impl TryFrom<Cow<'_, str>> for Schedule {
    type Error = Error;

    fn try_from(expression: Cow<'_, str>) -> Result<Self, Self::Error> {
        match schedule.parse(&expression) {
            Ok(schedule_fields) => Ok(Schedule::new(expression.into_owned(), schedule_fields)), // Extract from winnow tuple
            Err(parse_error) => Err(ErrorKind::Expression(format!("{parse_error}")).into()),
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

trait FromField
where
    Self: Sized,
{
    //TODO: Replace with std::convert::TryFrom when stable
    fn from_field(field: Field) -> Result<Self, Error>;
}

impl<T> FromField for T
where
    T: TimeUnitField,
{
    fn from_field(field: Field) -> Result<T, Error> {
        if field.specifiers.len() == 1
            && field.specifiers.first().unwrap() == &RootSpecifier::from(Specifier::All)
        {
            return Ok(T::all());
        }
        let mut ordinals = OrdinalSet::new();
        for specifier in field.specifiers {
            let specifier_ordinals: OrdinalSet = T::ordinals_from_root_specifier(&specifier)?;
            for ordinal in specifier_ordinals {
                ordinals.insert(T::validate_ordinal(ordinal)?);
            }
        }
        Ok(T::from_ordinal_set(ordinals))
    }
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

fn named_point(i: &mut &str) -> winnow::Result<RootSpecifier> {
    name.map(RootSpecifier::NamedPoint).parse_next(i)
}

fn last_named(i: &mut &str) -> winnow::Result<SingleSpecifier> {
    delimited(
        multispace0,
        terminated(take_while(3.., |c| AsChar::is_alpha(c) && c != 'L'), "L"),
        multispace0,
    )
    .map(|s: &str| SingleSpecifier::NamedPoint(s.to_owned()))
    .parse_next(i)
}

fn single_point(i: &mut &str) -> winnow::Result<SingleSpecifier> {
    digit1
        .try_map(u32::from_str)
        .map(SingleSpecifier::Point)
        .parse_next(i)
}

fn single_named_point(i: &mut &str) -> winnow::Result<SingleSpecifier> {
    alpha1
        .map(ToOwned::to_owned)
        .map(SingleSpecifier::NamedPoint)
        .parse_next(i)
}

fn single_specifier(i: &mut &str) -> winnow::Result<SingleSpecifier> {
    alt((single_point, single_named_point)).parse_next(i)
}

fn dom_last_point(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        preceded("L-", single_point),
        "L".map(|_| SingleSpecifier::Point(0)),
    ))
    .map(|specifier| RootSpecifier::LastPoint(specifier))
    .parse_next(i)
}

fn dow_last_point(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        terminated(single_specifier, "L"),
        last_named,
        "L".map(|_| SingleSpecifier::Point(0)),
    ))
    .map(|specifier| RootSpecifier::LastPoint(specifier))
    .parse_next(i)
}

fn weekday(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        "LW".map(|_| IS_LAST_OCCURRENCE | IS_WEEKDAY),
        "WL".map(|_| IS_LAST_OCCURRENCE | IS_WEEKDAY),
        terminated(ordinal, "W"),
    ))
    .map(|ordinal| RootSpecifier::Weekday(ordinal))
    .parse_next(i)
}

fn nth_of_month(i: &mut &str) -> winnow::Result<RootSpecifier> {
    separated_pair(single_specifier, "#", digit1.try_map(u32::from_str))
        .map(|(specifier, ordinal)| RootSpecifier::NthOfMonth(specifier, ordinal))
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
    separated_pair(ordinal, "-", ordinal)
        .map(|(start, end)| Specifier::Range(start, end))
        .parse_next(i)
}

fn named_range(i: &mut &str) -> winnow::Result<Specifier> {
    separated_pair(name, "-", name)
        .map(|(start, end)| Specifier::NamedRange(start, end))
        .parse_next(i)
}

fn all(i: &mut &str) -> winnow::Result<Specifier> {
    "*".map(|_| Specifier::All).parse_next(i)
}

fn any(i: &mut &str) -> winnow::Result<Specifier> {
    "?".map(|_| Specifier::All).parse_next(i)
}

fn specifier(i: &mut &str) -> winnow::Result<Specifier> {
    alt((all, range, point, named_range)).parse_next(i)
}

fn specifier_with_any(i: &mut &str) -> winnow::Result<Specifier> {
    alt((any, specifier)).parse_next(i)
}

fn root_specifier(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((period, specifier.map(RootSpecifier::from), named_point)).parse_next(i)
}

fn dow_root_specifier_with_any(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        period_with_any,
        dow_last_point,
        nth_of_month,
        specifier_with_any.map(RootSpecifier::from),
        named_point,
    ))
    .parse_next(i)
}

fn dom_root_specifier_with_any(i: &mut &str) -> winnow::Result<RootSpecifier> {
    alt((
        period_with_any,
        weekday,
        dom_last_point,
        specifier_with_any.map(RootSpecifier::from),
    ))
    .parse_next(i)
}

fn root_specifier_list(i: &mut &str) -> winnow::Result<Vec<RootSpecifier>> {
    let list = separated(1.., root_specifier, ",");
    let single_item = root_specifier.map(|spec| vec![spec]);
    delimited(multispace0, alt((list, single_item)), multispace0).parse_next(i)
}

fn dow_root_specifier_list_with_any(i: &mut &str) -> winnow::Result<Vec<RootSpecifier>> {
    let list = separated(1.., dow_root_specifier_with_any, ",");
    let single_item = dow_root_specifier_with_any.map(|spec| vec![spec]);
    delimited(multispace0, alt((list, single_item)), multispace0).parse_next(i)
}

fn dom_root_specifier_list_with_any(i: &mut &str) -> winnow::Result<Vec<RootSpecifier>> {
    let list = separated(1.., dom_root_specifier_with_any, ",");
    let single_item = dom_root_specifier_with_any.map(|spec| vec![spec]);
    delimited(multispace0, alt((list, single_item)), multispace0).parse_next(i)
}

fn field(i: &mut &str) -> winnow::Result<Field> {
    let specifiers = root_specifier_list.parse_next(i)?;
    Ok(Field { specifiers })
}

fn dow_field_with_any(i: &mut &str) -> winnow::Result<Field> {
    let specifiers = dow_root_specifier_list_with_any.parse_next(i)?;
    Ok(Field { specifiers })
}

fn dom_field_with_any(i: &mut &str) -> winnow::Result<Field> {
    let specifiers = dom_root_specifier_list_with_any.parse_next(i)?;
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

fn longhand(i: &mut &str) -> winnow::Result<ScheduleFields> {
    let seconds = field.try_map(Seconds::from_field);
    let minutes = field.try_map(Minutes::from_field);
    let hours = field.try_map(Hours::from_field);
    let days_of_month = dom_field_with_any.try_map(DaysOfMonth::from_field);
    let months = field.try_map(Months::from_field);
    let days_of_week = dow_field_with_any.try_map(DaysOfWeek::from_field);
    let years = opt(field.try_map(Years::from_field));
    let fields = (
        seconds,
        minutes,
        hours,
        days_of_month,
        months,
        days_of_week,
        years,
    );

    terminated(fields, eof)
        .map(
            |(seconds, minutes, hours, days_of_month, months, days_of_week, years)| {
                let years = years.unwrap_or_else(Years::all);
                ScheduleFields::new(
                    seconds,
                    minutes,
                    hours,
                    days_of_month,
                    months,
                    days_of_week,
                    years,
                )
            },
        )
        .parse_next(i)
}

fn schedule(i: &mut &str) -> winnow::Result<ScheduleFields> {
    alt((shorthand, longhand)).parse_next(i)
}

#[cfg(test)]
mod test {
    use super::*;

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
    fn test_nom_valid_last_point() {
        let expression = "L";
        let result = dom_last_point.parse(expression).unwrap();
        assert_eq!(result, RootSpecifier::LastPoint(SingleSpecifier::Point(0)));
    }

    #[test]
    fn test_nom_valid_to_last_point() {
        let expression = "L-5";
        dom_last_point.parse(expression).unwrap();

        let expression = "L-0";
        dom_last_point.parse(expression).unwrap();

        // "L-[number]" is invalid in day of week fields
        let expression = "L-0";
        assert!(dow_last_point.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_to_last_point() {
        let expression = "5-L";
        assert!(dom_last_point.parse(expression).is_err());

        let expression = "L-LW";
        assert!(dom_last_point.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_last_of_point_named() {
        let expression = "MONDAYL";
        dow_last_point.parse(expression).unwrap();
        let expression = "MONL";
        dow_last_point.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_last_of_point_num() {
        let expression = "1L";
        dow_last_point.parse(expression).unwrap();

        let expression = "0L";
        dow_last_point.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_last_of_point_num() {
        let expression = "1 L";
        assert!(dow_last_point.parse(expression).is_err());

        // Invalid because it can be ambiguous with a last specifier
        // (MON L could be "last MONDAY" or "MONDAY and SATURDAY")
        let expression = "MON L";
        assert!(dow_last_point.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_weekday() {
        let expression = "1W";
        weekday.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_last_weekday() {
        let expression = "LW";
        weekday.parse(expression).unwrap();

        let expression = "WL";
        weekday.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_weekday() {
        let expression = "W";
        assert!(weekday.parse(expression).is_err());

        let expression = "WEDW";
        assert!(weekday.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_nth_of_month() {
        let expression = "1#1";
        nth_of_month.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_named_nth_of_month() {
        let expression = "MON#1";
        nth_of_month.parse(expression).unwrap();

        // Parsing is valid but this will error when building schedule:
        let expression = "FRIDAY#0";
        nth_of_month.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_nth_of_month() {
        let expression = "MON# 1";
        assert!(nth_of_month.parse(expression).is_err());

        let expression = "MON #1";
        assert!(nth_of_month.parse(expression).is_err());
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
        dow_field_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_last_point_list() {
        let expression = "TUEL,FRIL";
        dow_field_with_any.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_number_list() {
        let expression = ",1,2";
        assert!(field.parse(expression).is_err());
        assert!(dow_field_with_any.parse(expression).is_err());
    }

    #[test]
    fn test_nom_field_with_any_valid_any() {
        let expression = "?";
        dom_field_with_any.parse(expression).unwrap();
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

        let expression = "1-4L";
        assert!(range.parse(expression).is_err());

        let expression = "2-5#3";
        assert!(range.parse(expression).is_err());

        let expression = "1-L-3";
        assert!(range.parse(expression).is_err());

        let expression = "1-5W";
        assert!(range.parse(expression).is_err());

        let expression = "1-LW";
        assert!(range.parse(expression).is_err());

        let expression = "1-WL";
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

        let expression = "Tue-Thu#3";
        assert!(range.parse(expression).is_err());

        let expression = "Tue-L";
        assert!(range.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_schedule() {
        let expression = "* * * * * *";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_schedule() {
        let expression = "* * * *";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_seconds_list() {
        let expression = "0,20,40 * * * * *";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_range() {
        let expression = "0-40 * * * * *";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_mix() {
        let expression = "0-5,58 * * * * *";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_seconds_range() {
        let expression = "0-65 * * * * *";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_list() {
        let expression = "103,12 * * * * *";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_mix() {
        let expression = "0-5,102 * * * * *";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_list() {
        let expression = "* * * * * MON,WED,FRI";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_list() {
        let expression = "* * * * * MON,TURTLE";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_range() {
        let expression = "* * * * * MON-FRI";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_range() {
        let expression = "* * * * * BEAR-OWL";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_range_specifier() {
        let expression = "10-12/10-12 * * * * ?";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_month_any() {
        let expression = "* * * ? * *";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any() {
        let expression = "* * * * * ?";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_month_any_days_of_week_specific() {
        let expression = "* * * ? * Mon,Thu";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any_days_of_month_specific() {
        let expression = "* * * 1,2 * ?";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_dom_and_dow_any() {
        let expression = "* * * ? * ?";
        schedule.parse(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_other_fields_any() {
        let expression = "? * * * * *";
        assert!(schedule.parse(expression).is_err());

        let expression = "* ? * * * *";
        assert!(schedule.parse(expression).is_err());

        let expression = "* * ? * * *";
        assert!(schedule.parse(expression).is_err());

        let expression = "* * * * ? *";
        assert!(schedule.parse(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_trailing_characters() {
        let expression = "* * * * * *foo *";
        assert!(schedule.parse(expression).is_err());

        let expression = "* * * * * * * foo";
        assert!(schedule.parse(expression).is_err());
    }

    /// Issue #86
    #[test]
    fn shorthand_must_match_whole_input() {
        let expression = "@dailyBla";
        assert!(schedule.parse(expression).is_err());
        let expression = " @dailyBla ";
        assert!(schedule.parse(expression).is_err());
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
            "0 0 0 1 1 ? 2020-2040/2200",
        ] {
            assert!(schedule.parse(invalid_expression).is_err());
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
        ] {
            assert!(schedule.parse(valid_expression).is_ok());
        }
    }
}

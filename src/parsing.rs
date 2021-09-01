use nom::*;
use nom::character::complete::{alpha1, digit1};
use std::iter::{Iterator};
use std::str::{self, FromStr};

use crate::error::{Error, ErrorKind};
use crate::schedule::{ScheduleFields, Schedule};
use crate::specifier::*;
use crate::time_unit::*;
use crate::ordinal::*;

impl FromStr for Schedule {
    type Err = Error;
    fn from_str(expression: &str) -> Result<Self, Self::Err> {
        match schedule(expression) {
            Ok((_, schedule_fields)) => {
                Ok(Schedule::new(String::from(expression), schedule_fields))
            } // Extract from nom tuple
            Err(_) => Err(ErrorKind::Expression("Invalid cron expression.".to_owned()).into()), //TODO: Details
        }
    }
}

impl ScheduleFields {
    fn from_field_list(fields: Vec<Field>) -> Result<ScheduleFields, Error> {
        let number_of_fields = fields.len();
        if number_of_fields != 6 && number_of_fields != 7 {
            return Err(ErrorKind::Expression(format!(
                "Expression has {} fields. Valid cron \
                 expressions have 6 or 7.",
                number_of_fields
            ))
            .into());
        }

        let mut iter = fields.into_iter();

        let seconds = Seconds::from_field(iter.next().unwrap())?;
        let minutes = Minutes::from_field(iter.next().unwrap())?;
        let hours = Hours::from_field(iter.next().unwrap())?;
        let days_of_month = DaysOfMonth::from_field(iter.next().unwrap())?;
        let months = Months::from_field(iter.next().unwrap())?;
        let days_of_week = DaysOfWeek::from_field(iter.next().unwrap())?;
        let years: Years = iter
            .next()
            .map(Years::from_field)
            .unwrap_or_else(|| Ok(Years::all()))?;

        Ok(ScheduleFields::new(
            seconds,
            minutes,
            hours,
            days_of_month,
            months,
            days_of_week,
            years,
        ))
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
        if field.specifiers.len() == 1 && 
            field.specifiers.get(0).unwrap() == &RootSpecifier::from(Specifier::All) 
            { return Ok(T::all()); }
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

named!(
    ordinal<&str, u32>,
    map_res!(ws!(digit1), |x: &str| x.parse())
);

named!(
    name<&str, String>,
    map!(ws!(alpha1), |x| x.to_owned())
);

named!(
    point<&str, Specifier>,
    do_parse!(o: ordinal >> (Specifier::Point(o)))
);

named!(
    named_point<&str, RootSpecifier>,
    do_parse!(n: name >> (RootSpecifier::NamedPoint(n)))
);

named!(
    period<&str, RootSpecifier>,
    complete!(do_parse!(
        start: specifier >> tag!("/") >> step: ordinal >> (RootSpecifier::Period(start, step))
    ))
);

named!(
    period_with_any<&str, RootSpecifier>,
    complete!(do_parse!(
        start: specifier_with_any >> tag!("/") >> step: ordinal >> (RootSpecifier::Period(start, step))
    ))
);

named!(
    range<&str, Specifier>,
    complete!(do_parse!(
        start: ordinal >> tag!("-") >> end: ordinal >> (Specifier::Range(start, end))
    ))
);

named!(
    named_range<&str, Specifier>,
    complete!(do_parse!(
        start: name >> tag!("-") >> end: name >> (Specifier::NamedRange(start, end))
    ))
);

named!(all<&str, Specifier>, do_parse!(tag!("*") >> (Specifier::All)));

named!(any<&str, Specifier>, do_parse!(tag!("?") >> (Specifier::All)));

named!(
    specifier<&str, Specifier>,
    alt!(all | range | point | named_range)
);

named!(
    specifier_with_any<&str, Specifier>,
    alt!(
        any |
        specifier
    )
);

named!(
    root_specifier<&str, RootSpecifier>,
    alt!(period | map!(specifier, RootSpecifier::from) | named_point)
);

named!(
    root_specifier_with_any<&str, RootSpecifier>,
    alt!(period_with_any | map!(specifier_with_any, RootSpecifier::from) | named_point)
);

named!(
    root_specifier_list<&str, Vec<RootSpecifier>>,
    ws!(alt!(
        do_parse!(list: separated_nonempty_list!(tag!(","), root_specifier) >> (list))
            | do_parse!(spec: root_specifier >> (vec![spec]))
    ))
);

named!(
    root_specifier_list_with_any<&str, Vec<RootSpecifier>>,
    ws!(alt!(
        do_parse!(list: separated_nonempty_list!(tag!(","), root_specifier_with_any) >> (list))
            | do_parse!(spec: root_specifier_with_any >> (vec![spec]))
    ))
);

named!(
    field<&str, Field>,
    do_parse!(specifiers: root_specifier_list >> (Field { specifiers }))
);

named!(
    field_with_any<&str, Field>,
    alt!(
        do_parse!(specifiers: root_specifier_list_with_any >> (Field { specifiers }))
    )
);

named!(
    shorthand_yearly<&str, ScheduleFields>,
    do_parse!(
        tag!("@yearly")
            >> (ScheduleFields::new(
                Seconds::from_ordinal(0),
                Minutes::from_ordinal(0),
                Hours::from_ordinal(0),
                DaysOfMonth::from_ordinal(1),
                Months::from_ordinal(1),
                DaysOfWeek::all(),
                Years::all()
            ))
    )
);

named!(
    shorthand_monthly<&str, ScheduleFields>,
    do_parse!(
        tag!("@monthly")
            >> (ScheduleFields::new(
                Seconds::from_ordinal(0),
                Minutes::from_ordinal(0),
                Hours::from_ordinal(0),
                DaysOfMonth::from_ordinal(1),
                Months::all(),
                DaysOfWeek::all(),
                Years::all()
            ))
    )
);

named!(
    shorthand_weekly<&str, ScheduleFields>,
    do_parse!(
        tag!("@weekly")
            >> (ScheduleFields::new(
                Seconds::from_ordinal(0),
                Minutes::from_ordinal(0),
                Hours::from_ordinal(0),
                DaysOfMonth::all(),
                Months::all(),
                DaysOfWeek::from_ordinal(1),
                Years::all()
            ))
    )
);

named!(
    shorthand_daily<&str, ScheduleFields>,
    do_parse!(
        tag!("@daily")
            >> (ScheduleFields::new(
                Seconds::from_ordinal(0),
                Minutes::from_ordinal(0),
                Hours::from_ordinal(0),
                DaysOfMonth::all(),
                Months::all(),
                DaysOfWeek::all(),
                Years::all()
            ))
    )
);

named!(
    shorthand_hourly<&str, ScheduleFields>,
    do_parse!(
        tag!("@hourly")
            >> (ScheduleFields::new(
                Seconds::from_ordinal(0),
                Minutes::from_ordinal(0),
                Hours::all(),
                DaysOfMonth::all(),
                Months::all(),
                DaysOfWeek::all(),
                Years::all()
            ))
    )
);

named!(
    shorthand<&str, ScheduleFields>,
    alt!(
        shorthand_yearly
            | shorthand_monthly
            | shorthand_weekly
            | shorthand_daily
            | shorthand_hourly
    )
);

named!(
    longhand<&str, ScheduleFields>,
    map_res!(
        complete!(do_parse!(
            seconds: field >>
            minutes: field >>
            hours: field >>
            days_of_month: field_with_any >>
            months: field >>
            days_of_week: field_with_any >>
            years: opt!(field) >>
            eof!() >>
            ({
                let mut fields = vec![
                    seconds,
                    minutes,
                    hours,
                    days_of_month,
                    months,
                    days_of_week,
                ];
                if let Some(years) = years {
                    fields.push(years);
                }
                fields
            })
        )),
        ScheduleFields::from_field_list
    )
);

named!(schedule<&str, ScheduleFields>, alt!(shorthand | longhand));
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nom_valid_number() {
        let expression = "1997";
        point(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_point() {
        let expression = "a";
        assert!(point(expression).is_err());
    }

    #[test]
    fn test_nom_valid_named_point() {
        let expression = "WED";
        named_point(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_point() {
        let expression = "8";
        assert!(named_point(expression).is_err());
    }

    #[test]
    fn test_nom_valid_period() {
        let expression = "1/2";
        period(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_period() {
        let expression = "Wed/4";
        assert!(period(expression).is_err());
    }

    #[test]
    fn test_nom_valid_number_list() {
        let expression = "1,2";
        field(expression).unwrap();
        field_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_number_list() {
        let expression = ",1,2";
        assert!(field(expression).is_err());
        assert!(field_with_any(expression).is_err());
    }

    #[test]
    fn test_nom_field_with_any_valid_any() {
        let expression = "?";
        field_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_field_invalid_any() {
        let expression = "?";
        assert!(field(expression).is_err());
    }

    #[test]
    fn test_nom_valid_range_field() {
        let expression = "1-4";
        range(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_all() {
        let expression = "*/2";
        period(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_range() {
        let expression = "10-20/2";
        period(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_named_range() {
        let expression = "Mon-Thurs/2";
        period(expression).unwrap();

        let expression = "February-November/2";
        period(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_point() {
        let expression = "10/2";
        period(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_period_any() {
        let expression = "?/2";
        assert!(period(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_named_point() {
        let expression = "Tues/2";
        assert!(period(expression).is_err());

        let expression = "February/2";
        assert!(period(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_specifier_range() {
        let expression = "10-12/*";
        assert!(period(expression).is_err());
    }

    #[test]
    fn test_nom_valid_period_with_any_all() {
        let expression = "*/2";
        period_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_range() {
        let expression = "10-20/2";
        period_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_named_range() {
        let expression = "Mon-Thurs/2";
        period_with_any(expression).unwrap();

        let expression = "February-November/2";
        period_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_point() {
        let expression = "10/2";
        period_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_any() {
        let expression = "?/2";
        period_with_any(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_period_with_any_named_point() {
        let expression = "Tues/2";
        assert!(period_with_any(expression).is_err());

        let expression = "February/2";
        assert!(period_with_any(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_any_specifier_range() {
        let expression = "10-12/*";
        assert!(period_with_any(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_range_field() {
        let expression = "-4";
        assert!(range(expression).is_err());
    }

    #[test]
    fn test_nom_valid_named_range_field() {
        let expression = "TUES-THURS";
        named_range(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_range_field() {
        let expression = "3-THURS";
        assert!(named_range(expression).is_err());
    }

    #[test]
    fn test_nom_valid_schedule() {
        let expression = "* * * * * *";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_schedule() {
        let expression = "* * * *";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_seconds_list() {
        let expression = "0,20,40 * * * * *";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_range() {
        let expression = "0-40 * * * * *";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_mix() {
        let expression = "0-5,58 * * * * *";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_seconds_range() {
        let expression = "0-65 * * * * *";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_list() {
        let expression = "103,12 * * * * *";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_mix() {
        let expression = "0-5,102 * * * * *";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_list() {
        let expression = "* * * * * MON,WED,FRI";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_list() {
        let expression = "* * * * * MON,TURTLE";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_range() {
        let expression = "* * * * * MON-FRI";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_range() {
        let expression = "* * * * * BEAR-OWL";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_range_specifier() {
        let expression = "10-12/10-12 * * * * ?";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_month_any() {
        let expression = "* * * ? * *";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any() {
        let expression = "* * * * * ?";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_month_any_days_of_week_specific() {
        let expression = "* * * ? * Mon,Thu";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any_days_of_month_specific() {
        let expression = "* * * 1,2 * ?";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_valid_dom_and_dow_any() {
        let expression = "* * * ? * ?";
        schedule(expression).unwrap();
    }

    #[test]
    fn test_nom_invalid_other_fields_any() {
        let expression = "? * * * * *";
        assert!(schedule(expression).is_err());

        let expression = "* ? * * * *";
        assert!(schedule(expression).is_err());

        let expression = "* * ? * * *";
        assert!(schedule(expression).is_err());

        let expression = "* * * * ? *";
        assert!(schedule(expression).is_err());
    }

    #[test]
    fn test_nom_invalid_trailing_characters() {
        let expression = "* * * * * *foo *";
        assert!(schedule(expression).is_err());

        let expression = "* * * * * * * foo";
        assert!(schedule(expression).is_err());
    }
}

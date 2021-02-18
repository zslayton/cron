use nom::{types::CompleteStr as Input, *};
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
        match schedule(Input(expression)) {
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
    ordinal<Input, u32>,
    map_res!(ws!(digit), |x: Input| x.0.parse())
);

named!(
    name<Input, String>,
    map!(ws!(alpha), |x| x.0.to_owned())
);

named!(
    point<Input, Specifier>,
    do_parse!(o: ordinal >> (Specifier::Point(o)))
);

named!(
    named_point<Input, RootSpecifier>,
    do_parse!(n: name >> (RootSpecifier::NamedPoint(n)))
);

named!(
    period<Input, RootSpecifier>,
    complete!(do_parse!(
        start: specifier >> tag!("/") >> step: ordinal >> (RootSpecifier::Period(start, step))
    ))
);

named!(
    period_with_any<Input, RootSpecifier>,
    complete!(do_parse!(
        start: specifier_with_any >> tag!("/") >> step: ordinal >> (RootSpecifier::Period(start, step))
    ))
);

named!(
    range<Input, Specifier>,
    complete!(do_parse!(
        start: ordinal >> tag!("-") >> end: ordinal >> (Specifier::Range(start, end))
    ))
);

named!(
    named_range<Input, Specifier>,
    complete!(do_parse!(
        start: name >> tag!("-") >> end: name >> (Specifier::NamedRange(start, end))
    ))
);

named!(all<Input, Specifier>, do_parse!(tag!("*") >> (Specifier::All)));

named!(any<Input, Specifier>, do_parse!(tag!("?") >> (Specifier::All)));

named!(
    specifier<Input, Specifier>,
    alt!(all | range | point | named_range)
);

named!(
    specifier_with_any<Input, Specifier>,
    alt!(
        any |
        specifier
    )
);

named!(
    root_specifier<Input, RootSpecifier>,
    alt!(period | map!(specifier, RootSpecifier::from) | named_point)
);

named!(
    root_specifier_with_any<Input, RootSpecifier>,
    alt!(period_with_any | map!(specifier_with_any, RootSpecifier::from) | named_point)
);

named!(
    root_specifier_list<Input, Vec<RootSpecifier>>,
    ws!(alt!(
        do_parse!(list: separated_nonempty_list!(tag!(","), root_specifier) >> (list))
            | do_parse!(spec: root_specifier >> (vec![spec]))
    ))
);

named!(
    root_specifier_list_with_any<Input, Vec<RootSpecifier>>,
    ws!(alt!(
        do_parse!(list: separated_nonempty_list!(tag!(","), root_specifier_with_any) >> (list))
            | do_parse!(spec: root_specifier_with_any >> (vec![spec]))
    ))
);

named!(
    field<Input, Field>,
    do_parse!(specifiers: root_specifier_list >> (Field { specifiers }))
);

named!(
    field_with_any<Input, Field>,
    alt!(
        do_parse!(specifiers: root_specifier_list_with_any >> (Field { specifiers }))
    )
);

named!(
    shorthand_yearly<Input, ScheduleFields>,
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
    shorthand_monthly<Input, ScheduleFields>,
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
    shorthand_weekly<Input, ScheduleFields>,
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
    shorthand_daily<Input, ScheduleFields>,
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
    shorthand_hourly<Input, ScheduleFields>,
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
    shorthand<Input, ScheduleFields>,
    alt!(
        shorthand_yearly
            | shorthand_monthly
            | shorthand_weekly
            | shorthand_daily
            | shorthand_hourly
    )
);

named!(
    longhand<Input, ScheduleFields>,
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

named!(schedule<Input, ScheduleFields>, alt!(shorthand | longhand));
#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_nom_valid_number() {
        let expression = "1997";
        point(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_point() {
        let expression = "a";
        assert!(point(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_named_point() {
        let expression = "WED";
        named_point(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_point() {
        let expression = "8";
        assert!(named_point(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_period() {
        let expression = "1/2";
        period(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_period() {
        let expression = "Wed/4";
        assert!(period(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_number_list() {
        let expression = "1,2";
        field(Input(expression)).unwrap();
        field_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_number_list() {
        let expression = ",1,2";
        assert!(field(Input(expression)).is_err());
        assert!(field_with_any(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_field_with_any_valid_any() {
        let expression = "?";
        field_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_field_invalid_any() {
        let expression = "?";
        assert!(field(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_range_field() {
        let expression = "1-4";
        range(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_all() {
        let expression = "*/2";
        period(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_range() {
        let expression = "10-20/2";
        period(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_named_range() {
        let expression = "Mon-Thurs/2";
        period(Input(expression)).unwrap();

        let expression = "February-November/2";
        period(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_point() {
        let expression = "10/2";
        period(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_period_any() {
        let expression = "?/2";
        assert!(period(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_period_named_point() {
        let expression = "Tues/2";
        assert!(period(Input(expression)).is_err());

        let expression = "February/2";
        assert!(period(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_period_specifier_range() {
        let expression = "10-12/*";
        assert!(period(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_period_with_any_all() {
        let expression = "*/2";
        period_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_range() {
        let expression = "10-20/2";
        period_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_named_range() {
        let expression = "Mon-Thurs/2";
        period_with_any(Input(expression)).unwrap();

        let expression = "February-November/2";
        period_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_point() {
        let expression = "10/2";
        period_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_period_with_any_any() {
        let expression = "?/2";
        period_with_any(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_period_with_any_named_point() {
        let expression = "Tues/2";
        assert!(period_with_any(Input(expression)).is_err());

        let expression = "February/2";
        assert!(period_with_any(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_any_specifier_range() {
        let expression = "10-12/*";
        assert!(period_with_any(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_range_field() {
        let expression = "-4";
        assert!(range(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_named_range_field() {
        let expression = "TUES-THURS";
        named_range(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_named_range_field() {
        let expression = "3-THURS";
        assert!(named_range(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_schedule() {
        let expression = "* * * * * *";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_schedule() {
        let expression = "* * * *";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_seconds_list() {
        let expression = "0,20,40 * * * * *";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_range() {
        let expression = "0-40 * * * * *";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_seconds_mix() {
        let expression = "0-5,58 * * * * *";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_seconds_range() {
        let expression = "0-65 * * * * *";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_list() {
        let expression = "103,12 * * * * *";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_seconds_mix() {
        let expression = "0-5,102 * * * * *";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_list() {
        let expression = "* * * * * MON,WED,FRI";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_list() {
        let expression = "* * * * * MON,TURTLE";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_week_range() {
        let expression = "* * * * * MON-FRI";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_days_of_week_range() {
        let expression = "* * * * * BEAR-OWL";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_period_with_range_specifier() {
        let expression = "10-12/10-12 * * * * ?";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_valid_days_of_month_any() {
        let expression = "* * * ? * *";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any() {
        let expression = "* * * * * ?";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_month_any_days_of_week_specific() {
        let expression = "* * * ? * Mon,Thu";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_days_of_week_any_days_of_month_specific() {
        let expression = "* * * 1,2 * ?";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_valid_dom_and_dow_any() {
        let expression = "* * * ? * ?";
        schedule(Input(expression)).unwrap();
    }

    #[test]
    fn test_nom_invalid_other_fields_any() {
        let expression = "? * * * * *";
        assert!(schedule(Input(expression)).is_err());

        let expression = "* ? * * * *";
        assert!(schedule(Input(expression)).is_err());

        let expression = "* * ? * * *";
        assert!(schedule(Input(expression)).is_err());

        let expression = "* * * * ? *";
        assert!(schedule(Input(expression)).is_err());
    }

    #[test]
    fn test_nom_invalid_trailing_characters() {
        let expression = "* * * * * *foo *";
        assert!(schedule(Input(expression)).is_err());

        let expression = "* * * * * * * foo";
        assert!(schedule(Input(expression)).is_err());
    }
}
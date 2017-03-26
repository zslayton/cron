use nom::*;
use std::str::{self, FromStr};
use std::collections::BTreeSet;
use std::collections::Bound::{Included, Unbounded};
use chrono::{UTC, DateTime, Duration, Datelike, Timelike};
use chrono::offset::TimeZone;
use std::iter::{self, Iterator};
use error::{Error, ErrorKind};

use time_unit::*;

pub struct Schedule {
    years: Years,
    days_of_week: DaysOfWeek,
    months: Months,
    days_of_month: DaysOfMonth,
    hours: Hours,
    minutes: Minutes,
    seconds: Seconds,
}

impl Schedule {
    fn from_field_list(fields: Vec<Field>) -> Result<Schedule, Error> {
        let number_of_fields = fields.len();
        if number_of_fields != 6 && number_of_fields != 7 {
            bail!(ErrorKind::Expression(format!("Expression has {} fields. Valid cron \
                                                expressions have 6 or 7.",
                                               number_of_fields)));
        }

        let mut iter = fields.into_iter();

        let seconds = Seconds::from_field(iter.next().unwrap())?;
        let minutes = Minutes::from_field(iter.next().unwrap())?;
        let hours = Hours::from_field(iter.next().unwrap())?;
        let days_of_month = DaysOfMonth::from_field(iter.next().unwrap())?;
        let months = Months::from_field(iter.next().unwrap())?;
        let days_of_week = DaysOfWeek::from_field(iter.next().unwrap())?;
        let years: Years = iter.next().map(Years::from_field).unwrap_or_else(|| Ok(Years::all()))?;

        Ok(Schedule::from(seconds,
                          minutes,
                          hours,
                          days_of_month,
                          months,
                          days_of_week,
                          years))
    }

    fn from(seconds: Seconds,
            minutes: Minutes,
            hours: Hours,
            days_of_month: DaysOfMonth,
            months: Months,
            days_of_week: DaysOfWeek,
            years: Years)
            -> Schedule {
        Schedule {
            years: years,
            days_of_week: days_of_week,
            months: months,
            days_of_month: days_of_month,
            hours: hours,
            minutes: minutes,
            seconds: seconds,
        }
    }

    fn next_after<Z>(&self, after: &DateTime<Z>) -> Option<DateTime<Z>>
        where Z: TimeZone
    {
        let datetime = after.clone() + Duration::seconds(1);

        // The first time we iterate through the options for each time unit, we should start with
        // the current value of each unit from the provided datetime. (e.g. if the datetime is
        // Dec 3, we should start at month=12, day=3. When we exhaust that month, we should start
        // over the next year with month=1, day=1.

        let mut month_starts = once_and_then(datetime.month(), Months::inclusive_min());
        let mut day_of_month_starts = once_and_then(datetime.day(), DaysOfMonth::inclusive_min());
        let mut hour_starts = once_and_then(datetime.hour(), Hours::inclusive_min());
        let mut minute_starts = once_and_then(datetime.minute(), Minutes::inclusive_min());
        let mut second_starts = once_and_then(datetime.second(), Seconds::inclusive_min());

        //    println!("Looking for next schedule time after {}", after.to_rfc3339());
        for year in self.years
            .ordinals()
            .range((Included(datetime.year() as u32), Unbounded))
            .cloned() {

            let month_start = month_starts.next().unwrap();
            let month_end = Months::inclusive_max();
            let month_range = (Included(month_start), Included(month_end));

            for month in self.months.ordinals().range(month_range).cloned() {

                let day_of_month_start = day_of_month_starts.next().unwrap();
                let day_of_month_end = days_in_month(month, year);
                let day_of_month_range = (Included(day_of_month_start), Included(day_of_month_end));

                'day_loop: for day_of_month in self.days_of_month
                    .ordinals()
                    .range(day_of_month_range)
                    .cloned() {

                    let hour_start = hour_starts.next().unwrap();
                    let hour_end = Hours::inclusive_max();
                    let hour_range = (Included(hour_start), Included(hour_end));

                    for hour in self.hours.ordinals().range(hour_range).cloned() {

                        let minute_start = minute_starts.next().unwrap();
                        let minute_end = Minutes::inclusive_max();
                        let minute_range = (Included(minute_start), Included(minute_end));

                        for minute in self.minutes.ordinals().range(minute_range).cloned() {

                            let second_start = second_starts.next().unwrap();
                            let second_end = Seconds::inclusive_max();
                            let second_range = (Included(second_start), Included(second_end));

                            for second in self.seconds.ordinals().range(second_range).cloned() {
                                let timezone = datetime.timezone();
                                let candidate = timezone.ymd(year as i32, month, day_of_month)
                                    .and_hms(hour, minute, second);
                                if !self.days_of_week
                                    .ordinals()
                                    .contains(&candidate.weekday().number_from_sunday()) {
                                    continue 'day_loop;
                                }
                                return Some(candidate);
                            }
                        } // End of minutes range
                        let _ = second_starts.next().unwrap();
                    } // End of hours range
                    let _ = minute_starts.next().unwrap();
                    let _ = second_starts.next().unwrap();
                } // End of Day of Month range
                let _ = hour_starts.next().unwrap();
                let _ = minute_starts.next().unwrap();
                let _ = second_starts.next().unwrap();
            } // End of Month range
            let _ = day_of_month_starts.next().unwrap();
            let _ = hour_starts.next().unwrap();
            let _ = minute_starts.next().unwrap();
            let _ = second_starts.next().unwrap();
        }

        // We ran out of dates to try.
        None
    }

    pub fn upcoming<Z>(& self, timezone: Z) -> ScheduleIterator<Z>
        where Z: TimeZone
    {
        self.after(&timezone.from_utc_datetime(&UTC::now().naive_utc()))
    }

    pub fn after<'a, Z>(&'a self, after: &DateTime<Z>) -> ScheduleIterator<'a, Z>
        where Z: TimeZone
    {
        ScheduleIterator::new(self, after)
    }
}

impl FromStr for Schedule {
    type Err = Error;
    fn from_str(expression: &str) -> Result<Self, Self::Err> {
        use nom::IResult::*;
        match schedule(expression.as_bytes()) {
            Done(_, schedule) => Ok(schedule), // Extract from nom tuple
            Error(_) => bail!(ErrorKind::Expression("Invalid cron expression.".to_owned())), //TODO: Details
            Incomplete(_) => bail!(ErrorKind::Expression("Incomplete cron expression.".to_owned())),
        }
    }
}

pub struct ScheduleIterator<'a, Z>
    where Z: TimeZone
{
    is_done: bool,
    schedule: &'a Schedule,
    previous_datetime: DateTime<Z>,
}
//TODO: Cutoff datetime?

impl<'a, Z> ScheduleIterator<'a, Z>
    where Z: TimeZone
{
    fn new(schedule: &'a Schedule, starting_datetime: &DateTime<Z>) -> ScheduleIterator<'a, Z> {
        ScheduleIterator {
            is_done: false,
            schedule: schedule,
            previous_datetime: starting_datetime.clone(),
        }
    }
}

impl<'a, Z> Iterator for ScheduleIterator<'a, Z>
    where Z: TimeZone
{
    type Item = DateTime<Z>;

    fn next(&mut self) -> Option<DateTime<Z>> {
        if self.is_done {
            return None;
        }
        if let Some(next_datetime) = self.schedule.next_after(&self.previous_datetime) {
            self.previous_datetime = next_datetime.clone();
            Some(next_datetime)
        } else {
            self.is_done = true;
            None
        }
    }
}

fn once_and_then<T>(head: T, long_tail: T) -> impl Iterator<Item = T>
    where T: Copy
{
    iter::once(head).chain(iter::once(long_tail).cycle())
}

pub type Ordinal = u32;
// TODO: Make OrdinalSet an enum.
// It should either be a BTreeSet of ordinals or an `All` option to save space.
//`All` can iterate from inclusive_min to inclusive_max and answer membership queries
pub type OrdinalSet = BTreeSet<Ordinal>;

#[derive(Debug)]
pub enum Specifier {
    All,
    Point(Ordinal),
    NamedPoint(String),
    Period(Ordinal, u32),
    Range(Ordinal, Ordinal),
    NamedRange(String, String),
}

#[derive(Debug)]
pub struct Field {
    pub specifiers: Vec<Specifier>, // TODO: expose iterator?
}

trait FromField
    where Self: Sized
{
    //TODO: Replace with std::convert::TryFrom when stable
    fn from_field(field: Field) -> Result<Self, Error>;
}

impl<T> FromField for T
    where T: TimeUnitField
{
    fn from_field(field: Field) -> Result<T, Error> {
        let mut ordinals = OrdinalSet::new(); //TODO: Combinator
        for specifier in field.specifiers {
            let specifier_ordinals: OrdinalSet = T::ordinals_from_specifier(&specifier)?;
            for ordinal in specifier_ordinals {
                ordinals.insert(T::validate_ordinal(ordinal)?);
            }
        }

        Ok(T::from_ordinal_set(ordinals))
    }
}

named!(ordinal <u32>,
  map_res!(
      map_res!(
          ws!(digit),
          str::from_utf8
      ),
      FromStr::from_str
  )
);

named!(name <String>,
  map!(
    map_res!(
      ws!(alpha),
      str::from_utf8
    ),
    str::to_owned
  )
);

named!(point <Specifier>,
  do_parse!(
    o: ordinal >>
    (Specifier::Point(o))
  )
);

named!(named_point <Specifier>,
  do_parse!(
    n: name >>
    (Specifier::NamedPoint(n))
  )
);

named!(period <Specifier>,
  complete!(
    do_parse!(
      start: ordinal >>
      tag!("/") >>
      step: ordinal >>
      (Specifier::Period(start, step))
    )
  )
);

named!(range <Specifier>,
  complete!(
    do_parse!(
      start: ordinal >>
      tag!("-") >>
      end: ordinal >>
      (Specifier::Range(start, end))
    )
  )
);

named!(named_range <Specifier>,
  complete!(
    do_parse!(
      start: name >>
      tag!("-") >>
      end: name >>
      (Specifier::NamedRange(start, end))
    )
  )
);

named!(all <Specifier>,
  do_parse!(
    tag!("*") >>
    (Specifier::All)
  )
);

named!(specifier <Specifier>,
  alt!(
    all |
    period |
    range |
    point |
    named_range |
    named_point
  )
);

named!(specifier_list <Vec<Specifier>>,
  ws!(
    alt!(
      do_parse!(
        list: separated_nonempty_list!(tag!(","), specifier) >>
        (list)
      ) |
      do_parse!(
        spec: specifier >>
        (vec![spec])
      )
    )
  )
);

named!(field <Field>,
  do_parse!(
    specifiers: specifier_list >>
    (Field {
      specifiers: specifiers
    })
  )
);

named!(shorthand_yearly <Schedule>,
  do_parse!(
    tag!("@yearly") >>
    (Schedule::from(
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

named!(shorthand_monthly <Schedule>,
  do_parse!(
    tag!("@monthly") >>
    (Schedule::from(
      Seconds::from_ordinal_set(iter::once(0).collect()),
      Minutes::from_ordinal_set(iter::once(0).collect()),
      Hours::from_ordinal_set(iter::once(0).collect()),
      DaysOfMonth::from_ordinal_set(iter::once(1).collect()),
      Months::all(),
      DaysOfWeek::all(),
      Years::all()
    ))
  )
);

named!(shorthand_weekly <Schedule>,
  do_parse!(
    tag!("@weekly") >>
    (Schedule::from(
      Seconds::from_ordinal_set(iter::once(0).collect()),
      Minutes::from_ordinal_set(iter::once(0).collect()),
      Hours::from_ordinal_set(iter::once(0).collect()),
      DaysOfMonth::all(),
      Months::all(),
      DaysOfWeek::from_ordinal_set(iter::once(1).collect()),
      Years::all()
    ))
  )
);

named!(shorthand_daily <Schedule>,
  do_parse!(
    tag!("@daily") >>
    (Schedule::from(
      Seconds::from_ordinal_set(iter::once(0).collect()),
      Minutes::from_ordinal_set(iter::once(0).collect()),
      Hours::from_ordinal_set(iter::once(0).collect()),
      DaysOfMonth::all(),
      Months::all(),
      DaysOfWeek::all(),
      Years::all()
    ))
  )
);

named!(shorthand_hourly <Schedule>,
  do_parse!(
    tag!("@hourly") >>
    (Schedule::from(
      Seconds::from_ordinal_set(iter::once(0).collect()),
      Minutes::from_ordinal_set(iter::once(0).collect()),
      Hours::all(),
      DaysOfMonth::all(),
      Months::all(),
      DaysOfWeek::all(),
      Years::all()
    ))
  )
);

named!(shorthand <Schedule>,
  alt!(
    shorthand_yearly  |
    shorthand_monthly |
    shorthand_weekly  |
    shorthand_daily   |
    shorthand_hourly
  )
);

named!(longhand <Schedule>,
  map_res!(
    complete!(
      do_parse!(
        fields: many_m_n!(6, 7, field) >>
        eof!() >>
        (fields)
      )
    ),
    Schedule::from_field_list
  )
);

named!(schedule <Schedule>,
  alt!(
    shorthand |
    longhand
  )
);

fn is_leap_year(year: Ordinal) -> bool {
    let by_four = year % 4 == 0;
    let by_hundred = year % 100 == 0;
    let by_four_hundred = year % 400 == 0;
    by_four && ((!by_hundred) || by_four_hundred)
}

fn days_in_month(month: Ordinal, year: Ordinal) -> u32 {
    let is_leap_year = is_leap_year(year);
    match month {
        9 | 4 | 6 | 11 => 30,
        2 if is_leap_year => 29,
        2 => 28,
        _ => 31,
    }
}

#[test]
fn test_next_after() {
    let expression = "0 5,13,40-42 17 1 Jan *";
    let schedule = schedule(expression.as_bytes());
    assert!(schedule.is_done());
    let schedule = schedule.unwrap().1;
    let next = schedule.next_after(&UTC::now());
    println!("NEXT AFTER for {} {:?}", expression, next);
    assert!(next.is_some());
}

#[test]
fn test_upcoming_utc() {
    let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
    let schedule = schedule(expression.as_bytes());
    assert!(schedule.is_done());
    let schedule = schedule.unwrap().1;
    let mut upcoming = schedule.upcoming(UTC);
    let next1 = upcoming.next();
    assert!(next1.is_some());
    let next2 = upcoming.next();
    assert!(next2.is_some());
    let next3 = upcoming.next();
    assert!(next3.is_some());
    println!("Upcoming 1 for {} {:?}", expression, next1);
    println!("Upcoming 2 for {} {:?}", expression, next2);
    println!("Upcoming 3 for {} {:?}", expression, next3);
}

#[test]
fn test_upcoming_local() {
    use chrono::Local;
    let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
    let schedule = schedule(expression.as_bytes());
    assert!(schedule.is_done());
    let schedule = schedule.unwrap().1;
    let mut upcoming = schedule.upcoming(Local);
    let next1 = upcoming.next();
    assert!(next1.is_some());
    let next2 = upcoming.next();
    assert!(next2.is_some());
    let next3 = upcoming.next();
    assert!(next3.is_some());
    println!("Upcoming 1 for {} {:?}", expression, next1);
    println!("Upcoming 2 for {} {:?}", expression, next2);
    println!("Upcoming 3 for {} {:?}", expression, next3);
}


#[test]
fn test_valid_from_str() {
    let schedule = Schedule::from_str("0 0,30 0,6,12,18 1,15 Jan-March Thurs");
    assert!(schedule.is_ok());
}

#[test]
fn test_invalid_from_str() {
    let schedule = Schedule::from_str("cheesecake 0,30 0,6,12,18 1,15 Jan-March Thurs");
    assert!(schedule.is_err());
}

#[test]
fn test_nom_valid_number() {
    let expression = "1997";
    assert!(point(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_point() {
    let expression = "a";
    assert!(point(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_named_point() {
    let expression = "WED";
    assert!(named_point(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_named_point() {
    let expression = "8";
    assert!(named_point(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_period() {
    let expression = "1/2";
    assert!(period(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_period() {
    let expression = "Wed/4";
    assert!(period(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_number_list() {
    let expression = "1,2";
    assert!(field(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_number_list() {
    let expression = ",1,2";
    assert!(field(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_range_field() {
    let expression = "1-4";
    assert!(range(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_range_field() {
    let expression = "-4";
    assert!(range(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_named_range_field() {
    let expression = "TUES-THURS";
    assert!(named_range(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_named_range_field() {
    let expression = "3-THURS";
    assert!(named_range(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_schedule() {
    let expression = "* * * * * *";
    assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_schedule() {
    let expression = "* * * *";
    assert!(schedule(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_seconds_list() {
    let expression = "0,20,40 * * * * *";
    assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_valid_seconds_range() {
    let expression = "0-40 * * * * *";
    assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_valid_seconds_mix() {
    let expression = "0-5,58 * * * * *";
    assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_seconds_range() {
    let expression = "0-65 * * * * *";
    assert!(schedule(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_invalid_seconds_list() {
    let expression = "103,12 * * * * *";
    assert!(schedule(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_invalid_seconds_mix() {
    let expression = "0-5,102 * * * * *";
    assert!(schedule(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_days_of_week_list() {
    let expression = "* * * * * MON,WED,FRI";
    assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_days_of_week_list() {
    let expression = "* * * * * MON,TURTLE";
    assert!(schedule(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_days_of_week_range() {
    let expression = "* * * * * MON-FRI";
    assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_days_of_week_range() {
    let expression = "* * * * * BEAR-OWL";
    assert!(schedule(expression.as_bytes()).is_err());
}

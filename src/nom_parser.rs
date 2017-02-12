use std::str::{self, FromStr};
use std::collections::BTreeSet;
use std::collections::Bound::{Included, Unbounded};
use std::borrow::Cow;
use chrono::{UTC, Local, DateTime, Duration, Datelike};
use chrono::offset::TimeZone;
use nom::*;

pub struct ExpressionError(String);

type Ordinal = u32;
// TODO: Make OrdinalSet an enum.
// It should either be a BTreeSet of ordinals or an `All` option to save space.
//`All` can iterate from inclusive_min to inclusive_max and answer membership queries
type OrdinalSet = BTreeSet<Ordinal>;

#[derive(Debug)]
pub enum Specifier {
  All,
  Point(Ordinal),
  NamedPoint(String),
  Period(Ordinal, u32),
  Range(Ordinal, Ordinal),
  NamedRange(String, String)
}

#[derive(Debug)]
pub struct Field {
  pub specifiers: Vec<Specifier> // TODO: expose iterator?
}

trait FromField where Self: Sized { //TODO: Replace with std::convert::TryFrom when stable
  fn from_field(field: Field) -> Result<Self, ExpressionError>;
}

impl <T> FromField for T where T: TimeUnitField {
  fn from_field(field: Field) -> Result<T, ExpressionError> {
    let mut ordinals = OrdinalSet::new(); //TODO: Combinator
    for specifier in field.specifiers {
      let specifier_ordinals : OrdinalSet = T::ordinals_from_specifier(&specifier)?;
      for ordinal in specifier_ordinals {
        ordinals.insert(T::validate_ordinal(ordinal)?);
      }
    }

    Ok(T::from_ordinal_set(ordinals))
  }
}

pub struct Years(OrdinalSet);
pub struct Months(OrdinalSet);
pub struct DaysOfMonth(OrdinalSet);
pub struct DaysOfWeek(OrdinalSet);
pub struct Hours(OrdinalSet);
pub struct Minutes(OrdinalSet);
pub struct Seconds(OrdinalSet);

trait TimeUnitField where Self: Sized {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self;
  fn name() -> Cow<'static, str>;
  fn inclusive_min() -> Ordinal;
  fn inclusive_max() -> Ordinal;
  fn ordinals(&self) -> &OrdinalSet;
  fn supported_ordinals() -> OrdinalSet {
    (Self::inclusive_min()..Self::inclusive_max()+1).collect()
  }
  fn all() -> Self {
    Self::from_ordinal_set(Self::supported_ordinals())
  }
  fn ordinal_from_name(name: &str) -> Result<Ordinal, ExpressionError> {
    Err(ExpressionError(format!("The '{}' field does not support using names. '{}' specified.", Self::name(), name)))
  }
  fn validate_ordinal(ordinal: Ordinal) -> Result<Ordinal, ExpressionError> {
    //println!("validate_ordinal for {} => {}", Self::name(), ordinal);
    match ordinal {
      i if i < Self::inclusive_min() => Err(
        ExpressionError(
          format!("{} must be greater than or equal to {}. ('{}' specified.)",
                  Self::name(),
                  Self::inclusive_min(),
                  i
          )
        )
      ),
      i if i > Self::inclusive_max() => Err(
        ExpressionError(
          format!("{} must be less than {}. ('{}' specified.)",
                  Self::name(),
                  Self::inclusive_max(),
                  i
          )
        )
      ),
      i => Ok(i)
    }
  }

  fn ordinals_from_specifier(specifier: &Specifier) -> Result<OrdinalSet, ExpressionError> {
    use self::Specifier::*;
    //println!("ordinals_from_specifier for {} => {:?}", Self::name(), specifier);
    match *specifier {
      All => Ok(Self::supported_ordinals()),
      Point(ordinal) => Ok((&[ordinal]).iter().cloned().collect()),
      NamedPoint(ref name) => Ok((&[Self::ordinal_from_name(name)?]).iter().cloned().collect()),
      Period(_start, _step) => unimplemented!(), //TODO
      Range(start, end) => {
        match (Self::validate_ordinal(start), Self::validate_ordinal(end)) {
          (Ok(start), Ok(end)) if start <= end => Ok((start..end+1).collect()),
          _ => Err(ExpressionError(format!("Invalid range for {}: {}-{}", Self::name(), start, end)))
        }
      },
      NamedRange(ref start_name, ref end_name) => {
        let start = Self::ordinal_from_name(start_name)?;
        let end = Self::ordinal_from_name(end_name)?;
        match (Self::validate_ordinal(start), Self::validate_ordinal(end)) {
          (Ok(start), Ok(end)) if start <= end => Ok((start..end+1).collect()),
          _ => Err(ExpressionError(format!("Invalid named range for {}: {}-{}", Self::name(), start_name, end_name)))
        }
      },
    }
  }
  //TODO: Converting names to ordinals
}

/* ===== SECONDS ===== */

impl TimeUnitField for Seconds {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Seconds(ordinal_set)
  }
  fn name<'a>() -> Cow<'static, str> {
    Cow::from("Seconds")
  }
  fn inclusive_min() -> Ordinal {
    0
  }
  fn inclusive_max() -> Ordinal {
    59
  }
  fn ordinals(&self) -> &OrdinalSet {
    &self.0
  }
}

/* ===== MINUTES ===== */

impl TimeUnitField for Minutes {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Minutes(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Minutes")
  }
  fn inclusive_min() -> Ordinal {
    0
  }
  fn inclusive_max() -> Ordinal {
    59
  }
  fn ordinals(&self) -> &OrdinalSet {
    &self.0
  }
}

/* ===== HOURS ===== */

impl TimeUnitField for Hours {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Hours(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Hours")
  }
  fn inclusive_min() -> Ordinal {
    0
  }
  fn inclusive_max() -> Ordinal {
    23
  }
  fn ordinals(&self) -> &OrdinalSet {
    &self.0
  }
}

/* ===== DAYS  ===== */

impl TimeUnitField for DaysOfMonth {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    DaysOfMonth(ordinal_set)
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
    &self.0
  }
}

/* ===== MONTHS ===== */

impl TimeUnitField for Months {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Months(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Months")
  }
  fn inclusive_min() -> Ordinal {
    1
  }
  fn inclusive_max() -> Ordinal {
    12
  }
  fn ordinal_from_name(name: &str) -> Result<Ordinal, ExpressionError> {
    //TODO: Use phf crate
    let ordinal = match name.to_lowercase().as_ref() {
      "jan" | "january" => 1,
      "feb" | "february" => 2,
      "mar" | "march" => 3,
      "apr" | "april" => 4,
      "may" => 5,
      "jun" | "june" => 6,
      "jul" | "july" => 7,
      "aug" | "august" => 8,
      "sep" | "september" => 9,
      "oct" | "october" => 10,
      "nov" | "november" => 11,
      "dec" | "december" => 12,
      _ => return Err(ExpressionError(format!("'{}' is not a valid month name.", name)))
    };
    Ok(ordinal)
  }
  fn ordinals(&self) -> &OrdinalSet {
    &self.0
  }
}

/* ===== DAYS OF WEEK ===== */

impl TimeUnitField for DaysOfWeek {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    DaysOfWeek(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Days of Week")
  }
  fn inclusive_min() -> Ordinal {
    1
  }
  fn inclusive_max() -> Ordinal {
    7
  }
  fn ordinal_from_name(name: &str) -> Result<Ordinal, ExpressionError> {
    //TODO: Use phf crate
    let ordinal = match name.to_lowercase().as_ref() {
      "sun" | "sunday" => 1,
      "mon" | "monday" => 2,
      "tue" | "tues" | "tuesday" => 3,
      "wed" | "wednesday" => 4,
      "thu" | "thurs" | "thursday" => 5,
      "fri" | "friday" => 6,
      "sat" | "saturday" => 7,
      _ => return Err(ExpressionError(format!("'{}' is not a valid day of the week.", name)))
    };
    Ok(ordinal)
  }
  fn ordinals(&self) -> &OrdinalSet {
    &self.0
  }
}

/* ===== YEARS ===== */

impl TimeUnitField for Years {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Years(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Years")
  }

  // TODO: Using the default impl, this will make a set w/100+ items each time "*" is used.
  // This is obviously suboptimal.
  fn inclusive_min() -> Ordinal {
    1970
  }
  fn inclusive_max() -> Ordinal {
    2100
  }
  fn ordinals(&self) -> &OrdinalSet {
    &self.0
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

named!(schedule <Schedule>,
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

fn is_leap_year(year: Ordinal) -> bool {
  let by_four = year % 4 == 0;
  let by_hundred = year % 100 == 0;
  let by_four_hundred = year % 400 == 0;
  return by_four && ((!by_hundred) || by_four_hundred);
}

fn days_in_month(month: Ordinal, year: Ordinal) -> u32 {
  let is_leap_year = is_leap_year(year);
  match month {
    9 | 4 | 6 | 11 => 30,
    2 if is_leap_year => 29,
    2 => 28,
    _ => 31
  }
}

struct Schedule {
  years: Years,
  days_of_week: DaysOfWeek,
  months: Months,
  days_of_month: DaysOfMonth,
  hours: Hours,
  minutes: Minutes,
  seconds: Seconds
}

impl Schedule {
  fn from_field_list(fields: Vec<Field>) -> Result<Schedule, ExpressionError> {
    let number_of_fields = fields.len();
    if number_of_fields != 6 && number_of_fields != 7 {
      return Err(ExpressionError(format!("Expression has {} fields. Valid cron expressions have 6 or 7.", number_of_fields)));
    }
    let mut iter = fields.into_iter();

    let seconds = Seconds::from_field(iter.next().unwrap())?;
    let minutes = Minutes::from_field(iter.next().unwrap())?;
    let hours = Hours::from_field(iter.next().unwrap())?;
    let days_of_month = DaysOfMonth::from_field(iter.next().unwrap())?;
    let months = Months::from_field(iter.next().unwrap())?;
    let days_of_week = DaysOfWeek::from_field(iter.next().unwrap())?;
    let years: Years = iter.next().map(Years::from_field).unwrap_or(Ok(Years::all()))?;

    Ok(Schedule::from(
      seconds,
      minutes,
      hours,
      days_of_month,
      months,
      days_of_week,
      years
    ))
  }

  fn from(seconds: Seconds, minutes: Minutes, hours: Hours, days_of_month: DaysOfMonth, months: Months, days_of_week: DaysOfWeek, years: Years) -> Schedule {
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

  pub fn next_after<Z>(&self, after: &DateTime<Z>) -> Option<DateTime<Z>> where Z: TimeZone + ZonedTimeProvider<Z> {
    let datetime = after.clone() + Duration::seconds(1);
    let timezone = Z::new();

    //    println!("Looking for next schedule time after {}", after.to_rfc3339());
    for year in self.years.ordinals().range((Included(datetime.year() as u32), Unbounded)).cloned() {

      //println!("Checking year {}", year);
      for month in self.months.ordinals().iter().cloned() {
        //println!("Checking month {}", month);
        'day_loop: for day in self.days_of_month.ordinals().range((Included(1), Included(days_in_month(month, year)))).cloned() {
          //println!("Checking day {}", day);
          for hour in self.hours.ordinals().iter().cloned() {
            //println!("Checking hour {}", hour);
            for minute in self.minutes.ordinals().iter().cloned() {
              //println!("Checking minute {}", minute);
              for second in self.seconds.ordinals().iter().cloned() {
                //println!("Checking second {}", second);

                let candidate = timezone.ymd(year as i32, month, day).and_hms(hour, minute, second);
                if candidate <= datetime {
                  //TODO: We can avoid this by only traversing months after the starting datetime during the first year's search
                  //println!("Candidate {} rejected. Too early.", candidate.to_rfc3339());
                  continue;
                }
                if !self.days_of_week.ordinals().contains(&candidate.weekday().number_from_sunday()) {
                  //TODO: If this happens, we should move to the next day, not just continue.
                  //println!("Candidate {} rejected. Incorrect weekday.", candidate.to_rfc3339());
                  continue 'day_loop;
                }

                //println!("Returning datetime {}", candidate.to_rfc3339());
                return Some(candidate);
              }
            }
          }
        }
      }
    }

    // We ran out of dates to try.
    None
  }

  pub fn upcoming<'a, Z>(&'a self) -> ScheduleIterator<'a, Z> where Z: ZonedTimeProvider<Z> {
    ScheduleIterator{
      is_done: false,
      schedule: self,
      previous_datetime: Z::now()
    }
  }
}

pub struct ScheduleIterator<'a, Z> where Z: ZonedTimeProvider<Z> {
  is_done: bool,
  schedule: &'a Schedule,
  previous_datetime: DateTime<Z>,
  //TODO: Cutoff datetime
}

impl <'a, Z> Iterator for ScheduleIterator<'a, Z> where Z: ZonedTimeProvider<Z> {
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

pub trait ZonedTimeProvider<Z>: TimeZone + Copy where Z: TimeZone + Copy
{
  fn new() -> Z;
  fn now() -> DateTime<Z>;
}

impl ZonedTimeProvider<UTC> for UTC {
  fn new() -> UTC {
    UTC
  }
  fn now() -> DateTime<UTC> {
    UTC::now()
  }
}

impl ZonedTimeProvider<Local> for Local {
  fn new() -> Local {
    Local
  }
  fn now() -> DateTime<Local> {
    Local::now()
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
  let mut upcoming: ScheduleIterator<UTC> = schedule.upcoming();
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
  let expression = "0 0,30 0,6,12,18 1,15 Jan-March Thurs";
  let schedule = schedule(expression.as_bytes());
  assert!(schedule.is_done());
  let schedule = schedule.unwrap().1;
  let mut upcoming: ScheduleIterator<Local> = schedule.upcoming();
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
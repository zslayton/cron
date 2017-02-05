use std::str::{self, FromStr};
use std::collections::BTreeSet;
use std::borrow::Cow;
use nom::*;

pub struct ExpressionError(String);

type Ordinal = u32;
type OrdinalSet = BTreeSet<Ordinal>;

#[derive(Debug)]
pub enum Specifier {
  All,
  Number(Ordinal),
  Period(Ordinal, u32),
  Range(Ordinal, Ordinal),
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
pub struct Days(OrdinalSet);
pub struct Hours(OrdinalSet);
pub struct Minutes(OrdinalSet);
pub struct Seconds(OrdinalSet);

trait TimeUnitField where Self: Sized {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self;
  fn name() -> Cow<'static, str>;
  fn inclusive_min() -> Ordinal;
  fn inclusive_max() -> Ordinal;
  fn validate_ordinal(ordinal: Ordinal) -> Result<Ordinal, ExpressionError> {
    println!("validate_ordinal for {} => {}", Self::name(), ordinal);
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
  // Does not perform validation, only transformation
  fn ordinals_from_specifier(specifier: &Specifier) -> Result<OrdinalSet, ExpressionError> {
    use self::Specifier::*;
    println!("ordinals_from_specifier for {} => {:?}", Self::name(), specifier);
    match *specifier {
      All => Ok(( Self::inclusive_min().. Self::inclusive_max()+1).collect()),
      Number(ordinal) => Ok((&[ordinal]).iter().cloned().collect()),
      Period(_start, _step) => unimplemented!(), //TODO
      Range(start, end) => {
        match (Self::validate_ordinal(start), Self::validate_ordinal(end)) {
          (Ok(start), Ok(end)) if start <= end => Ok((start..end+1).collect()),
          _ => Err(ExpressionError(format!("Invalid range for {}: {}-{}", Self::name(), start, end)))
        }
      }
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
}

/* ===== DAYS  ===== */

impl TimeUnitField for Days {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Days(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Days")
  }
  fn inclusive_min() -> Ordinal {
    1
  }
  fn inclusive_max() -> Ordinal {
    31
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
}

/* ===== YEARS ===== */

impl TimeUnitField for Years {
  fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
    Years(ordinal_set)
  }
  fn name() -> Cow<'static, str> {
    Cow::from("Years")
  }

  // TODO: Using the default impl, this will make a set w/100 items each time "*" is used.
  // This is obviously suboptimal.
  fn inclusive_min() -> Ordinal {
    1970
  }
  fn inclusive_max() -> Ordinal {
    2100
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

named!(number <Specifier>,
  do_parse!(
    o: ordinal >>
    (Specifier::Number(o))
  )
);

named!(range <Specifier>,
  do_parse!(
    start: ordinal >>
    tag!("-") >>
    end: ordinal >>
    (Specifier::Range(start, end))
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
    complete!(range) |
    number
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
        fields: many_m_n!(5, 6, field) >>
        eof!() >>
        (fields)
      )
    ),
    Schedule::from_field_list
  )
);

struct Schedule {
  years: Years,
  months: Months,
  days: Days,
  hours: Hours,
  minutes: Minutes,
  seconds: Seconds
}

impl Schedule {
  fn from_field_list(fields: Vec<Field>) -> Result<Schedule, ExpressionError> {
    let number_of_fields = fields.len();
    let mut iter = fields.into_iter();

    let seconds: Seconds;
    if number_of_fields == 6 {
      seconds = Seconds::from_field(iter.next().unwrap())?;
    } else if number_of_fields == 5 {
      let mut ordinal_set = BTreeSet::new();
      ordinal_set.insert(0);
      seconds = Seconds::from_ordinal_set(ordinal_set);
    } else {
      panic!("Field list was not the expected length.");
    }

    let minutes = Minutes::from_field(iter.next().unwrap())?;
    let hours = Hours::from_field(iter.next().unwrap())?;
    let days = Days::from_field(iter.next().unwrap())?;
    let months = Months::from_field(iter.next().unwrap())?;
    let years = Years::from_field(iter.next().unwrap())?;

    Ok(Schedule::from(
      years,
      months,
      days,
      hours,
      minutes,
      seconds
    ))
  }

  fn from(years: Years, months: Months, days: Days, hours: Hours, minutes: Minutes, seconds: Seconds) -> Schedule {
    Schedule {
      years: years,
      months: months,
      days: days,
      hours: hours,
      minutes: minutes,
      seconds: seconds,
    }
  }
}

#[test]
fn test_nom_valid_number() {
  let expression = "1997";
  assert!(field(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_number() {
  let expression = "a";
  assert!(field(expression.as_bytes()).is_err());
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
  assert!(field(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_range_field() {
  let expression = "-4";
  assert!(field(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_schedule() {
  let expression = "* * * * *";
  assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_schedule() {
  let expression = "* * * *";
  assert!(schedule(expression.as_bytes()).is_err());
}

#[test]
fn test_nom_valid_seconds() {
  let expression = "0,20,40 * * * * *";
  assert!(schedule(expression.as_bytes()).is_done());
}

#[test]
fn test_nom_invalid_seconds() {
  let expression = "0-65 * * * * *";
  assert!(schedule(expression.as_bytes()).is_err());
}


//named!(specifier <Specifier>,
//  many1!()
//)

//named!(parse_field <Field>,
//    alt!(
//        complete!(do_parse!(
//            start: parse_number >>
//            tag!("-")           >>
//            end: parse_number   >>
//            (Field::Range {
//                start: start,
//                end:   end,
//            })
//        )) |
//        map!(ws!(tag!("*")), |_| { Field::All }) |
//        map!(parse_number, |n| { Field::Number(n) })
//    )
//);
//
//named!(pub parse <Vec<Field>>, do_parse!(
//    fields: many1!(parse_field) >>
//    eof!() >>
//    (fields)
//));
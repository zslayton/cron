use std::error::{self, Error};
use std::fmt::{self, Display, Formatter};

#[derive(Debug)]
pub enum CronParseError {
  InvalidNumberOfFields(usize),
  SecondsError,
  MinutesError,
  HoursError,
  DayOfMonthError,
  MonthError,
  DayOfWeekError,
  YearError
}

pub type CronParseResult<T> = Result<T, CronParseError>;

impl Display for CronParseError {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "CronParseError: '{}'", self.description())
  }
}

impl error::Error for CronParseError {
  fn description(&self) -> &str {
    use self::CronParseError::*;
    match *self {
      InvalidNumberOfFields(_) => "You must specify either 6 or 7 whitespace-separated fields: (seconds) [minutes] [hours] [day of month] [month] [day of week] [year]",
      SecondsError => "The minutes field was not valid.",
      MinutesError => "The minutes field was not valid.",
      HoursError => "The hours field was not valid.",
      DayOfMonthError => "The day of month field was not valid.",
      MonthError => "The month field was not valid.",
      DayOfWeekError => "The day of week field was not valid.",
      YearError => "The year field was not valid."
    }
  }

  fn cause(&self) -> Option<&error::Error> {
    None
  }
}

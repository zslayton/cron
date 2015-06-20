mod unit;
pub use self::unit::UnitSchedule;

use ::error::CronParseResult;
use ::parser::Parser;

use chrono::*;

pub struct CronSchedule {
  // The original String that was parsed
  pub expression: String,
  // Schedule information
  pub seconds: Option<UnitSchedule>,
  pub minutes: UnitSchedule, 
  pub hours: UnitSchedule, 
  pub days_of_month: UnitSchedule, 
  pub months: UnitSchedule, 
  pub days_of_week: UnitSchedule, 
  pub years: UnitSchedule, 
}

impl CronSchedule {
  pub fn parse<A>(expression: A) -> CronParseResult<CronSchedule> where A : AsRef<str> {
    let parser = Parser::new();
    parser.parse(expression)
  }

  pub fn next_utc(&mut self) -> Option<DateTime<UTC>> {
    let now : DateTime<UTC> = UTC::now();
    self.next_utc_after(&now)
  }

  pub fn next_utc_after(&mut self, after: &DateTime<UTC>) -> Option<DateTime<UTC>> {
    let mut datetime = after.clone() + Duration::seconds(1);
    loop {
      let year = match self.years.get_current() {
        Some(year) => year,
        None => return None // We've run out of matchable years
      };

      let month = match self.months.get_current() {
        Some(month) => month,
        None => {
            self.bump_month();
            continue;
        }
      };

      let day = match self.days_of_month.get_current() {
        Some(day) => day,
        None => {
          self.bump_day();
          continue;
        }
      };

      let hour = match self.hours.get_current() {
        Some(hour) => hour,
        None => {
          self.bump_hour();
          continue;
        }
      };

      let minute = match self.minutes.get_current() {
        Some(minute) => minute,
        None => {
          self.bump_minute();
          continue;
        }
      };

      let mut second: Option<u32> = None;
      if let Some(ref seconds) = self.seconds {
          second = match seconds.get_current() {
          Some(second) => Some(second),
          None => {
            self.bump_second();
            continue;
          }
        }
      };

      let datetime = UTC.ymd(year as i32, month, day).and_hms(hour, minute, second.unwrap_or(0));
      return Some(datetime)
    }
  }

  fn bump_second(&mut self) {
     if let Some(ref sec_sched) = self.seconds {
        match sec_sched.get_next() {
          None => {
          // We've exhausted seconds, reset seconds and bump the minute.
            sec_sched.reset();
            self.bump_minute();
          },
          Some(second) => {
            // Do nothing
          }
        }
     }
  }
  
  fn bump_minute(&mut self) {
    match self.minutes.get_next() {
      None => {
      // We've exhausted minutes, reset minutes and bump the hour.
        self.minutes.reset();
        self.bump_hour();
      },
      Some(minute) => {
      // Do nothing
      }
    }
  }
 
  fn bump_hour(&mut self) {
    match self.hours.get_next() {
      None => {
      // We've exhausted hours, reset hours and bump the day.
        self.hours.reset();
        self.bump_day();
      },
      Some(hour) => {
      // Do nothing
      }
    }
  }

  fn bump_day(&mut self) {
    match self.days_of_month.get_next() {
      None => {
      // We've exhausted days, reset days and bump the month.
        self.days_of_month.reset();
        self.bump_month();
      },
      Some(day) => {
      // Do nothing
      }
    }
  }
  
  fn bump_month(&mut self) {
    match self.months.get_next() {
      None => {
      // We've exhausted months, reset days and bump the year.
        self.months.reset();
        //self.bump_year();
      },
      Some(month) => {
      // Do nothing
      }
    }
  }

  fn is_leap_year(year: u32) -> bool {
    let by_four = year % 4 == 0;
    let by_hundred = year % 100 == 0;
    let by_four_hundred = year % 400 == 0;
    return by_four && ((!by_hundred) || by_four_hundred);
  }

  fn next_day(datetime: &DateTime<UTC>) -> u32 {
    let mut day = datetime.day() + 1;
    let is_leap_year = CronSchedule::is_leap_year(datetime.year() as u32);
    let max_days = match datetime.month() {
      9 | 4 | 6 | 11 => 30,
      2 if is_leap_year => 29,
      2 => 28,
      _ => 31
    };
    if day > max_days {
      day = day - max_days;
    }
    day
  }

  fn next_month(datetime: &DateTime<UTC>) -> u32 {
    let mut month = datetime.month() + 1;
    if month > 12 {
      month = month - 12;
    }
    month
  }

  fn next_hour(datetime: &DateTime<UTC>) -> u32 {
    let hour = (datetime.hour() + 1) % 24;
    hour
  }

  fn next_minute(datetime: &DateTime<UTC>) -> u32 {
    let minute = (datetime.minute() + 1) % 60;
    minute
  }
  fn next_second(datetime: &DateTime<UTC>) -> u32 {
    let second = (datetime.second() + 1) % 60;
    second
  }
}

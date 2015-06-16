mod second;
mod minute;
mod hour;
mod dom;
mod month;
mod dow;
mod year;

pub use self::second::SecondSchedule;
pub use self::minute::MinuteSchedule;
pub use self::hour::HourSchedule;
pub use self::dom::DayOfMonthSchedule;
pub use self::month::MonthSchedule;
pub use self::dow::DayOfWeekSchedule;
pub use self::year::YearSchedule;

use ::error::CronParseResult;
use ::parser::Parser;

use chrono::*;

pub struct CronSchedule {
  // The original String that was parsed
  pub expression: String,
  // Schedule information
  pub seconds: Option<SecondSchedule>,
  pub minutes: MinuteSchedule, 
  pub hours: HourSchedule, 
  pub days_of_month: DayOfMonthSchedule, 
  pub months: MonthSchedule, 
  pub days_of_week: DayOfWeekSchedule, 
  pub years: YearSchedule, 
}

impl CronSchedule {
  pub fn parse<A>(expression: A) -> CronParseResult<CronSchedule> where A : AsRef<str> {
    let parser = Parser::new();
    parser.parse(expression)
  }

  pub fn next_utc(&self) -> Option<DateTime<UTC>> {
    let now : DateTime<UTC> = UTC::now();
    self.next_utc_after(&now)
  }

  pub fn next_utc_after(&self, after: &DateTime<UTC>) -> Option<DateTime<UTC>> {
    let mut datetime = after.clone() + Duration::seconds(1);
    loop {
      if !self.months.matches(datetime.month()) {
        datetime = UTC.ymd(datetime.year(), CronSchedule::next_month(&datetime), 1).and_hms(0,0,0);
        continue;
      }
      if !self.days_of_month.matches(datetime.day()) {
        datetime = UTC.ymd(datetime.year(), datetime.month(), CronSchedule::next_day(&datetime)).and_hms(0,0,0);
        continue;
      }
/*      if !self.days_of_week.matches(datetime.day()) {
        next_datetime = UTC.ymd(datetime.year(), datetime.month(), datetime.day()+1).and_hms(0,0,0);
        continue;
      } */
      if !self.hours.matches(datetime.hour()) {
        datetime = UTC.ymd(datetime.year(), datetime.month(), datetime.day()).and_hms(CronSchedule::next_hour(&datetime),0,0);
        continue;
      }
      if !self.minutes.matches(datetime.minute()) {
        datetime = UTC.ymd(datetime.year(), datetime.month(), datetime.day()).and_hms(datetime.hour(), CronSchedule::next_minute(&datetime),0);
        continue;
      }
      if let Some(ref second_schedule) = self.seconds {
        if !second_schedule.matches(datetime.second()) {
          datetime = UTC.ymd(datetime.year(), datetime.month(), datetime.day()).and_hms(datetime.hour(), datetime.minute(), CronSchedule::next_second(&datetime));
          continue;        
        }
      }
      break;
    }
    Some(datetime)
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

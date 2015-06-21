mod unit;
pub use self::unit::UnitSchedule;

use ::error::CronParseResult;
use ::parser::Parser;

use chrono::*;
use std::u32;

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

pub struct CronScheduleIterator<'a> {
  schedule: &'a CronSchedule,
  previous_datetime: Option<DateTime<UTC>>
}

impl <'a> Iterator for CronScheduleIterator<'a> {
  type Item = DateTime<UTC>;
  
  fn next(&mut self) -> Option<DateTime<UTC>> {
    let previous_datetime = match self.previous_datetime {
      Some(datetime) => datetime,
      None => UTC::now()
    };
    let next_datetime = self.schedule.next_utc_after(&previous_datetime);
    self.previous_datetime = next_datetime;
    next_datetime
  }
}

impl CronSchedule {
  pub fn parse<A>(expression: A) -> CronParseResult<CronSchedule> where A : AsRef<str> {
    let parser = Parser::new();
    parser.parse(expression)
  }

  pub fn upcoming<'a>(&'a self) -> CronScheduleIterator<'a> {
    CronScheduleIterator{
      schedule: self,
      previous_datetime: None
    }
  }

  pub fn next_utc(&self) -> Option<DateTime<UTC>> {
    let now : DateTime<UTC> = UTC::now();
    self.next_utc_after(&now)
  }

  pub fn next_utc_after(&self, after: &DateTime<UTC>) -> Option<DateTime<UTC>> {
    let mut datetime = after.clone() + Duration::minutes(1);
    
    let mut year_range = self.years.range_iter(datetime.year() as u32, u32::MAX);

    let second = None;
    
//    println!("Looking for next schedule time after {}", after.to_rfc3339());
    for year in &mut year_range {
      //println!("Checking year {}", year);
      let mut month_range = self.months.range_iter(1, 12);
      for month in &mut month_range {
        //println!("Checking month {}", month);
        let mut day_range = self.days_of_month.range_iter(1, CronSchedule::days_in_month(month, year)); 
        for day in &mut day_range {
          //println!("Checking day {}", day);
          let mut hour_range = self.hours.range_iter(0, 23);
          for hour in &mut hour_range {
            //println!("Checking hour {}", hour);
            let mut minute_range = self.minutes.range_iter(0, 59);
            for minute in &mut minute_range {
              //println!("Checking minute {}", minute);
              let candidate = UTC.ymd(year as i32, month, day).and_hms(hour, minute, second.unwrap_or(0));
              if candidate <= datetime {
                //println!("Candidate {} rejected. Too early.", candidate.to_rfc3339());
                continue;
              }
              //println!("Returning datetime {}", candidate.to_rfc3339());
              return Some(candidate);
            }
          }
        }
      }
    }

    // We ran out of dates to try.
    None
  }

  fn is_leap_year(year: u32) -> bool {
    let by_four = year % 4 == 0;
    let by_hundred = year % 100 == 0;
    let by_four_hundred = year % 400 == 0;
    return by_four && ((!by_hundred) || by_four_hundred);
  }

  fn days_in_month(month: u32, year: u32) -> u32 {
    let is_leap_year = CronSchedule::is_leap_year(year);
    match month {
      9 | 4 | 6 | 11 => 30,
      2 if is_leap_year => 29,
      2 => 28,
      _ => 31
    }
  }
}

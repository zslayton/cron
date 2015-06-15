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
}

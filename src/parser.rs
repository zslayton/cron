use ::error::CronParseResult;
use ::error::CronParseError::*;
use ::schedule::*;

pub struct Parser;

impl Parser {
  pub fn new() -> Parser {
    Parser
  }
  
  pub fn parse<A>(&self, raw_expression: A) -> CronParseResult<CronSchedule> where A : AsRef<str> {
    let raw_expression: &str = raw_expression.as_ref();

    // Break the expression into its component fields
    let whitespace: &[char] = &[' ', '\t', '\r', '\n'];
    let mut fields: Vec<&str> = raw_expression.split(whitespace).collect();
    if fields.len() != 6 && fields.len() != 7 {
      return Err(InvalidNumberOfFields(fields.len()))
    }
    fields.reverse(); // Allows us to pop from the end cheaply

    // Extract the optional seconds field
    let mut second_schedule: Option<SecondSchedule> = None; 
    if fields.len() == 7 {
      let seconds_expr = fields.pop().unwrap();
      let ss = try!(self.parse_second_expression(seconds_expr));
      second_schedule = Some(ss);
    }
    
    // There are now guaranteed to be 6 fields left
    // TODO: When slice patterns stabilize, simplify this to:
    // let &[year_expr, dow_expr, month_expr, dom_expr, hours_expr, minutes_expr] = fields.as_slice();
    let minutes_expr = fields.pop().unwrap();
    let hours_expr = fields.pop().unwrap();
    let dom_expr = fields.pop().unwrap();
    let month_expr = fields.pop().unwrap();
    let dow_expr = fields.pop().unwrap();
    let year_expr = fields.pop().unwrap();

    let minute_schedule = try!(self.parse_minute_expression(minutes_expr));
    let hour_schedule = try!(self.parse_hour_expression(hours_expr));
    let dom_schedule = try!(self.parse_dom_expression(dom_expr));
    let month_schedule = try!(self.parse_month_expression(month_expr));
    let dow_schedule = try!(self.parse_dow_expression(dow_expr));
    let year_schedule = try!(self.parse_year_expression(year_expr));

    Ok(CronSchedule {
      expression: raw_expression.to_owned(),
      seconds: second_schedule,
      minutes: minute_schedule,
      hours: hour_schedule,
      days_of_month: dom_schedule,
      months: month_schedule,
      days_of_week: dow_schedule,
      years: year_schedule
    })
  }

  fn parse_second_expression(&self, expr: &str) -> CronParseResult<SecondSchedule> {
    let second : u32 = match expr.parse() {
      Ok(second) => second,
      Err(_) => return Err(SecondsError)
    };
    
    let second_schedule = match SecondSchedule::from_second(second) {
      Some(second_schedule) => second_schedule,
      None => return Err(SecondsError)
    };
    Ok(second_schedule)
  }

  fn parse_minute_expression(&self, expr: &str) -> CronParseResult<MinuteSchedule> {
    let minute : u32 = match expr.parse() {
      Ok(minute) => minute,
      Err(_) => return Err(MinutesError)
    };
    let minute_schedule = match MinuteSchedule::from_minute(minute) {
      Some(minute_schedule) => minute_schedule,
      None => return Err(MinutesError)
    };
    Ok(minute_schedule)
  }

  fn parse_hour_expression(&self, expr: &str) -> CronParseResult<HourSchedule> {
    let hour: u32 = match expr.parse() {
      Ok(hour) => hour,
      Err(_) => return Err(HoursError)
    };
    let hour_schedule = match HourSchedule::from_hour(hour) {
      Some(hour_schedule) => hour_schedule,
      None => return Err(HoursError)
    };
    Ok(hour_schedule)
  }  

  fn parse_dom_expression(&self, expr: &str) -> CronParseResult<DayOfMonthSchedule> {
    let dom: u32 = match expr.parse() {
      Ok(dom) => dom,
      Err(_) => return Err(DayOfMonthError)
    };
    let dom_schedule = match DayOfMonthSchedule::from_day_of_month(dom) {
      Some(dom_schedule) => dom_schedule,
      None => return Err(DayOfMonthError)
    };
    Ok(dom_schedule)
  }  

  fn parse_month_expression(&self, expr: &str) -> CronParseResult<MonthSchedule> {
    let month: u32 = match expr.parse() {
      Ok(month) => month,
      Err(_) => return Err(MonthError)
    };
    let month_schedule = match MonthSchedule::from_month(month) {
      Some(month_schedule) => month_schedule,
      None => return Err(MonthError)
    };
    Ok(month_schedule)
  }  

  fn parse_dow_expression(&self, expr: &str) -> CronParseResult<DayOfWeekSchedule> {
    let day_of_week: u32 = match expr.parse() {
      Ok(dow) => dow,
      Err(_) => return Err(DayOfWeekError)
    };
    let day_of_week_schedule = match DayOfWeekSchedule::from_day_of_week(day_of_week) {
      Some(day_of_week_schedule) => day_of_week_schedule,
      None => return Err(DayOfWeekError)
    };
    Ok(day_of_week_schedule)
  }  

  fn parse_year_expression(&self, expr: &str) -> CronParseResult<YearSchedule> {
    let year: u32 = match expr.parse() {
      Ok(year) => year,
      Err(_) => return Err(YearError)
    };
    let year_schedule = match YearSchedule::from_year(year) {
      Some(year_schedule) => year_schedule,
      None => return Err(YearError)
    };
    Ok(year_schedule)
  }  
}

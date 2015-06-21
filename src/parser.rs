use ::error::CronParseResult;
use ::error::CronParseError::*;
use ::schedule::*;

//use std::collections::BTreeSet;

//TODO: Make an Error that can be translated into a CronParseError
enum CronFieldValue {
  Any(u32),
  Number(u32),
  List(Vec<CronFieldValue>),
  Range(u32, u32, u32),
  InvalidInput
}

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
    let mut second_schedule: Option<UnitSchedule> = None; 
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

  fn parse_field(&self, expr: &str) -> CronFieldValue {
    use self::CronFieldValue::*;
    //TODO: Handle bad input gracefully
    //println!("FIELD: {}", expr);
    if let Some(_comma_index) = expr.find(',') {
      //println!("It's a list.");
      let subfields: Vec<CronFieldValue> = expr.split(',')
        .inspect(|_subexpr| /*println!("Num: {}", num)*/{})
        .map(|subexpr| {
            self.parse_field(subexpr)
        })
        .collect();
      return List(subfields)
    }
    if let Some(_dash_index) = expr.find('-') {
      //println!("It's a range.");
      //TODO: Look for step specifier '/'. Assuming step=1 for now.
      let range : Vec<u32> = expr.split('-').map(|num|num.parse::<u32>().ok().expect("Couldn't parse range number!")).collect();
      let min : u32 = range[0];
      let max : u32 = range[1];
      return Range(min, max, 1);
    }
    //println!("It's a number.");
    return Number( expr.parse::<u32>().ok().expect("Couldn't parse number!"));
  }

  // TODO: Make this a method on the CronField itself
  fn cron_field_values(&self, field_value: &CronFieldValue) -> Vec<u32> {
    use self::CronFieldValue::*;
    let mut units : Vec<u32> = Vec::new();
    match *field_value {
      Number(number) => {
        //println!("Adding number");
        units.push(number);
      },
      List(ref subfields) => {
        //println!("Adding list");
        for subfield in subfields {
          let numbers = self.cron_field_values(subfield);
          for number in numbers {
             units.push(number);
          }
        }
      },
      Range(min, max, step) => {
        //println!("Adding range");
        if min > max {
          panic!("Invalid range! Min must be <= max.");
        }
        let mut number = min;
        while number <= max {
          units.push(number);
          number += step;
        }
      },
      _ => panic!("Unsupported field value!")
    };
    //println!("Returning {:?}", units);
    units
  }

  fn parse_second_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let seconds_value = self.parse_field(expr);
    let seconds = self.cron_field_values(&seconds_value);

    Ok(UnitSchedule::from_values(seconds))
  }

  fn parse_minute_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let minutes_value = self.parse_field(expr);
    let minutes = self.cron_field_values(&minutes_value);
    Ok(UnitSchedule::from_values(minutes))
  }

  fn parse_hour_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let hours_value = self.parse_field(expr);
    let hours = self.cron_field_values(&hours_value);
    Ok(UnitSchedule::from_values(hours))
  }  

  fn parse_dom_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let dom_value = self.parse_field(expr);
    let dom = self.cron_field_values(&dom_value);
    Ok(UnitSchedule::from_values(dom))
  }  

  fn parse_month_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let month_value = self.parse_field(expr);
    let month = self.cron_field_values(&month_value);
    Ok(UnitSchedule::from_values(month))
  }  

  fn parse_dow_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let dom_value = self.parse_field(expr);
    let dom = self.cron_field_values(&dom_value);
    Ok(UnitSchedule::from_values(dom))
  }  

  fn parse_year_expression(&self, expr: &str) -> CronParseResult<UnitSchedule> {
    let year_value = self.parse_field(expr);
    let year = self.cron_field_values(&year_value);
    Ok(UnitSchedule::from_values(year))
  }  
}

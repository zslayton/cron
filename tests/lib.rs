extern crate cron;
extern crate chrono;

#[cfg(test)]
mod tests {
  use cron::CronSchedule;
  use chrono::*;

  #[test]
  fn test_parse_with_seconds() {
    let expression = "1 2 3 4 5 6 2015";
    assert!(CronSchedule::parse(expression).is_ok());
  }

  #[test]
  fn test_parse_with_seconds_list() {
    let expression = "1,30,40 2 3 4 5 6 2015";
    assert!(CronSchedule::parse(expression).is_ok());
  }
  
  #[test]
  fn test_parse_with_lists() {
    let expression = "1,30,40 2,17,51 3,7,11 4 5 6 2015";
    let schedule = CronSchedule::parse(expression).unwrap();
    let mut date = UTC::now();
    println!("Fire times for {}:", expression);
    for _ in 0..5 {
      date = schedule.next_utc_after(&date).unwrap();
      println!("-> {}", date);
    }
    assert!(true);
  }

  #[test]
  fn test_parse_without_seconds() {
    let expression = "1 2 3 4 5 2015";
    assert!(CronSchedule::parse(expression).is_ok());
  }
  
  #[test]
  fn test_parse_too_many_fields() {
    let expression = "1 2 3 4 5 6 7 8 9 2015";
    assert!(CronSchedule::parse(expression).is_err());
  }
  
  #[test]
  fn test_not_enough_fields() {
    let expression = "1 2 3 2015";
    assert!(CronSchedule::parse(expression).is_err());
  }

  #[test]
  fn test_next_utc() {
    let expression = "1 2 3 4 5 6 2015";
    let schedule = CronSchedule::parse(expression).unwrap();
    let next = schedule.next_utc().unwrap();
    println!("Next fire time: {}", next.to_rfc3339());
  }
}

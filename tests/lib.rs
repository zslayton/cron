extern crate cron;
extern crate chrono;

#[cfg(test)]
mod tests {
  use cron::Schedule;
  use std::str::FromStr;
  use chrono::*;

  #[test]
  fn test_readme() {
    let expression =   "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
    let schedule = Schedule::from_str(expression).unwrap();
    println!("README: Upcoming fire times for '{}':", expression);
    for datetime in schedule.upcoming(UTC).take(10) {
      println!("README: -> {}", datetime);
    }
  }

  #[test]
  fn test_anything_goes() {
    let expression = "* * * * * * *";
    let schedule = Schedule::from_str(expression).unwrap();
    println!("All stars: Upcoming fire times for '{}':", expression);
    for datetime in schedule.upcoming(UTC).take(10) {
      println!("All stars: -> {}", datetime);
    }
  }

  #[test]
  fn test_parse_with_year() {
    let expression = "1 2 3 4 5 6 2015";
    assert!(Schedule::from_str(expression).is_ok());
  }

  #[test]
  fn test_parse_with_seconds_list() {
    let expression = "1,30,40 2 3 4 5 Mon-Fri";
    assert!(Schedule::from_str(expression).is_ok());
  }
  
  #[test]
  fn test_parse_with_lists() {
    let expression = "1 2,17,51 1-3,6,9-11 4,29 2,3,7 Tues";
    let schedule = Schedule::from_str(expression).unwrap();
    let mut date = UTC::now();
    println!("Fire times for {}:", expression);
    for _ in 0..20 {
      date = schedule.after(&date).next().expect("No further dates!");
      println!("-> {}", date);
    }
    assert!(true);
  }

  #[test]
  fn test_upcoming_iterator() {
    let expression = "0 2,17,51 1-3,6,9-11 4,29 2,3,7 Wed";
    let schedule = Schedule::from_str(expression).unwrap();
    println!("Upcoming fire times for '{}':", expression);
    for datetime in schedule.upcoming(UTC).take(12) {
      println!("-> {}", datetime);
    }
    assert!(true);
  }

  #[test]
  fn test_parse_without_year() {
    let expression = "1 2 3 4 5 6";
    assert!(Schedule::from_str(expression).is_ok());
  }
  
  #[test]
  fn test_parse_too_many_fields() {
    let expression = "1 2 3 4 5 6 7 8 9 2019";
    assert!(Schedule::from_str(expression).is_err());
  }
  
  #[test]
  fn test_not_enough_fields() {
    let expression = "1 2 3 2019";
    assert!(Schedule::from_str(expression).is_err());
  }

  #[test]
  fn test_next_utc() {
    let expression = "1 2 3 4 10 Fri";
    let schedule = Schedule::from_str(expression).unwrap();
    let next = schedule.upcoming(UTC).next().expect("There was no upcoming fire time.");
    println!("Next fire time: {}", next.to_rfc3339());
  }
}

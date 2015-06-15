pub struct SecondSchedule {
  second: u32
}

impl SecondSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_second(second: u32) -> Option<SecondSchedule> {
    if second > 59 {
      return None;
    }
    let second_schedule = SecondSchedule {
      second: second
    };
    Some(second_schedule)
  }
}

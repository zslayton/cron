pub struct MinuteSchedule {
  minute: u32
}

impl MinuteSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_minute(minute: u32) -> Option<MinuteSchedule> {
    if minute > 59 {
      return None;
    }
    let minute_schedule = MinuteSchedule {
      minute: minute
    };
    Some(minute_schedule)
  }

  pub fn matches(&self, minute: u32) -> bool {
    return self.minute == minute;
  }

  pub fn next_on_or_after(&self, minute: u32) -> (bool, u32) {
    (true, self.minute)
  }

}

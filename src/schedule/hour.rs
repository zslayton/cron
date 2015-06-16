pub struct HourSchedule {
  hour: u32
}

impl HourSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_hour(hour: u32) -> Option<HourSchedule> {
    if hour > 23 {
      return None;
    }
    let hour_schedule = HourSchedule {
      hour: hour
    };
    Some(hour_schedule)
  }
  pub fn matches(&self, hour: u32) -> bool {
    return self.hour == hour;
  }
}

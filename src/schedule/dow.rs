pub struct DayOfWeekSchedule {
  day_of_week: u32
}

impl DayOfWeekSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_day_of_week(day_of_week: u32) -> Option<DayOfWeekSchedule> {
    if day_of_week > 6 {
      return None;
    }
    let day_of_week_schedule = DayOfWeekSchedule {
      day_of_week: day_of_week
    };
    Some(day_of_week_schedule)
  }
}

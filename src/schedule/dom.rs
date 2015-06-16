pub struct DayOfMonthSchedule {
  day_of_month: u32
}

impl DayOfMonthSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_day_of_month(day_of_month: u32) -> Option<DayOfMonthSchedule> {
    if day_of_month < 1 || day_of_month > 31 {
      return None;
    }
    let day_of_month_schedule = DayOfMonthSchedule {
      day_of_month: day_of_month
    };
    Some(day_of_month_schedule)
  }
  
  pub fn matches(&self, day_of_month: u32) -> bool {
    return self.day_of_month == day_of_month;
  }
}

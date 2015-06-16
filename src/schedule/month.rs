pub struct MonthSchedule {
  month: u32
}

impl MonthSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_month(month: u32) -> Option<MonthSchedule> {
    if month < 1 || month > 12 {
      return None;
    }
    let month_schedule = MonthSchedule {
      month: month
    };
    Some(month_schedule)
  }
  
  pub fn matches(&self, month: u32) -> bool {
    return self.month == month;
  }
}

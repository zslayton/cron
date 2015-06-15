pub struct YearSchedule {
  year: u32
}

impl YearSchedule {
  // TODO: Use a Result to propagate error explanations
  pub fn from_year(year: u32) -> Option<YearSchedule> {
    if year < 1970 || year > 10_000 {
      return None;
    }
    let year_schedule = YearSchedule {
      year: year
    };
    Some(year_schedule)
  }
}

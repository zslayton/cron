use crate::time_unit::{DaysOfMonth, DaysOfWeek, Hours, Minutes, Months, Seconds, Years};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleFields {
    years: Years,
    days_of_week: DaysOfWeek,
    months: Months,
    days_of_month: DaysOfMonth,
    hours: Hours,
    minutes: Minutes,
    seconds: Seconds,
}

impl ScheduleFields {
    // Constructor
    pub fn new(
        seconds: Seconds,
        minutes: Minutes,
        hours: Hours,
        days_of_month: DaysOfMonth,
        months: Months,
        days_of_week: DaysOfWeek,
        years: Years,
    ) -> Self {
        Self {
            years,
            days_of_week,
            months,
            days_of_month,
            hours,
            minutes,
            seconds,
        }
    }

    // Getters
    pub fn years(&self) -> &Years { &self.years }
    pub fn months(&self) -> &Months { &self.months }
    pub fn days_of_month(&self) -> &DaysOfMonth { &self.days_of_month }
    pub fn days_of_week(&self) -> &DaysOfWeek { &self.days_of_week }
    pub fn hours(&self) -> &Hours { &self.hours }
    pub fn minutes(&self) -> &Minutes { &self.minutes }
    pub fn seconds(&self) -> &Seconds { &self.seconds }
}
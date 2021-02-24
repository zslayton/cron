use crate::time_unit::{DaysOfMonth, DaysOfWeek, Hours, Minutes, Months, Seconds, TimeUnitField, Years};
use crate::field::{FromField, Field};
use crate::error::{Error, ErrorKind};

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
    ) -> ScheduleFields {
        ScheduleFields {
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

    pub fn from_field_list(fields: Vec<Field>) -> Result<Self, Error> {
        let number_of_fields = fields.len();
        if number_of_fields != 6 && number_of_fields != 7 {
            return Err(ErrorKind::Expression(format!(
                "Expression has {} fields. Valid cron \
                 expressions have 6 or 7.",
                number_of_fields
            ))
            .into());
        }

        let mut iter = fields.into_iter();

        let seconds = Seconds::from_field(iter.next().unwrap())?;
        let minutes = Minutes::from_field(iter.next().unwrap())?;
        let hours = Hours::from_field(iter.next().unwrap())?;
        let days_of_month = DaysOfMonth::from_field(iter.next().unwrap())?;
        let months = Months::from_field(iter.next().unwrap())?;
        let days_of_week = DaysOfWeek::from_field(iter.next().unwrap())?;
        let years: Years = iter.next().map_or_else(|| Ok(Years::all()), Years::from_field)?;

        Ok(Self::new(
            seconds,
            minutes,
            hours,
            days_of_month,
            months,
            days_of_week,
            years,
        ))
    }
}
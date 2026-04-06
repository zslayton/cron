use chrono::TimeDelta;

/// Controls which cron expression variants are accepted by the parser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CronScheduleParts {
    /// Accept only 5-part expressions: minute hour day-of-month month day-of-week.
    Five,
    /// Accept only 6-part expressions: second minute hour day-of-month month day-of-week.
    Six,
    /// Accept both 5-part and 6-part expressions.
    Both,
}

/// Controls accepted day-of-week numeric notation in cron expressions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DayOfWeekNumbering {
    /// Sunday=1 through Saturday=7.
    OneToSeven,
    /// Sunday=0 through Saturday=6.
    ZeroToSix,
}

/// Controls how day-of-month and day-of-week fields are combined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DowDomOperand {
    /// Both day-of-month and day-of-week must match when both are restricted.
    And,
    /// Either day-of-month or day-of-week may match when both are restricted.
    Or,
}

/// Parsing and interpretation configuration for cron schedules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleConfig {
    pub cron_schedule_parts: CronScheduleParts,
    pub day_of_week_numbering: DayOfWeekNumbering,
    pub dow_dom_operand: DowDomOperand,
    pub search_interval: TimeDelta,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            cron_schedule_parts: CronScheduleParts::Six,
            day_of_week_numbering: DayOfWeekNumbering::OneToSeven,
            dow_dom_operand: DowDomOperand::And,
            search_interval: TimeDelta::days(400 * 366),
        }
    }
}

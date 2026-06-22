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
    OneIndexed,
    /// Sunday=0 or 7, Monday=1 through Saturday=6.
    ZeroIndexed,
}

/// Controls how day-of-month and day-of-week fields are combined.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DowDomOperand {
    /// Both day-of-month and day-of-week must match when both are restricted.
    And,
    /// Either day-of-month or day-of-week may match when both are restricted.
    Or,
}

/// Controls how nonexistent local times are handled during schedule iteration.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonexistentTimeBehavior {
    /// Skip scheduled local times that do not exist in the configured time zone.
    Skip,
    /// Return the next existent time after a scheduled local time that does not exist.
    NextExistent,
}

/// Parsing and interpretation configuration for cron schedules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScheduleConfig {
    pub cron_schedule_parts: CronScheduleParts,
    pub day_of_week_numbering: DayOfWeekNumbering,
    pub wraparound_ranges: bool,
    pub dow_dom_operand: DowDomOperand,
    pub nonexistent_time_behavior: NonexistentTimeBehavior,
    pub search_interval: TimeDelta,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            cron_schedule_parts: CronScheduleParts::Six,
            day_of_week_numbering: DayOfWeekNumbering::OneIndexed,
            wraparound_ranges: false,
            dow_dom_operand: DowDomOperand::And,
            nonexistent_time_behavior: NonexistentTimeBehavior::Skip,
            search_interval: TimeDelta::days(400 * 366),
        }
    }
}

#![allow(dead_code)]
use std::collections::BTreeSet;

pub struct CronExpression {
  // The original String that was parsed
  expression: String,
  // Schedule information
  seconds: BTreeSet<usize>,
  minutes: BTreeSet<usize>, 
  hours: BTreeSet<usize>, 
  days_of_month: BTreeSet<usize>, 
  months: BTreeSet<usize>, 
  days_of_week: BTreeSet<usize>, 
  years: BTreeSet<usize>, 
}

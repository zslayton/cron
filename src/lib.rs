#![deny(rust_2018_idioms)]
#![deny(rustdoc::broken_intra_doc_links)]
#![allow(clippy::needless_doctest_main)]
// Without a date and time library feature, schedules can be parsed and inspected only.
#![cfg_attr(
    not(any(feature = "chrono", feature = "jiff")),
    allow(dead_code, unused_imports)
)]
//! A cron expression parser and schedule explorer
//! # Example
//! ```
//! use cron::Schedule;
//! # #[cfg(feature = "chrono")]
//! use chrono::Utc;
//! use std::str::FromStr;
//!
//! # #[cfg(not(feature = "chrono"))]
//! # fn main() {}
//! # #[cfg(feature = "chrono")]
//! fn main() {
//!   //               sec  min   hour   day of month   month   day of week   year
//!   let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
//!   let schedule = Schedule::from_str(expression).unwrap();
//!   println!("Upcoming fire times:");
//!   for datetime in schedule.upcoming(Utc).take(10) {
//!     println!("-> {}", datetime);
//!   }
//! }
//!
//! /*
//! Upcoming fire times:
//! -> 2018-06-01 09:30:00 UTC
//! -> 2018-06-01 12:30:00 UTC
//! -> 2018-06-01 15:30:00 UTC
//! -> 2018-06-15 09:30:00 UTC
//! -> 2018-06-15 12:30:00 UTC
//! -> 2018-06-15 15:30:00 UTC
//! -> 2018-08-01 09:30:00 UTC
//! -> 2018-08-01 12:30:00 UTC
//! -> 2018-08-01 15:30:00 UTC
//! -> 2018-08-15 09:30:00 UTC
//! */
//! ```
//!
//! # Date and time libraries
//!
//! The `chrono` feature, enabled by default, provides the methods above for `chrono::DateTime`.
//! The `jiff` feature provides the same schedules for `jiff::ZonedDateTime`, through
//! `Schedule::upcoming_zoned`, `Schedule::after_zoned` and `Schedule::includes_zoned`. Both
//! features can be enabled together. To use only jiff:
//!
//! ```toml
//! [dependencies]
//! cron = { version = "0.17", default-features = false, features = ["jiff"] }
//! ```
//!
//! ```
//! # #[cfg(feature = "jiff")]
//! # {
//! use cron::Schedule;
//! use jiff::tz::TimeZone;
//! use std::str::FromStr;
//!
//! let schedule = Schedule::from_str("0 30 9 * * Mon-Fri *").unwrap();
//! for datetime in schedule.upcoming_zoned(TimeZone::UTC).take(3) {
//!     println!("-> {datetime}");
//! }
//! # }
//! ```

/// Error types used by this crate.
pub mod error;

mod datetime;
mod ordinal;
mod parsing;
mod queries;
mod schedule;
mod specifier;
mod time_unit;

pub use crate::schedule::Schedule;
#[cfg(feature = "chrono")]
pub use crate::schedule::{OwnedScheduleIterator, ScheduleIterator};
#[cfg(feature = "jiff")]
pub use crate::schedule::{OwnedZonedScheduleIterator, ZonedScheduleIterator};
pub use crate::time_unit::TimeUnitSpec;

#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]
#![feature(conservative_impl_trait)]
#![feature(step_by)]

extern crate chrono;
extern crate nom;

mod time_unit;
mod schedule;

pub use schedule::Schedule;

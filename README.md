# cron [![Rust](https://github.com/zslayton/cron/workflows/Rust/badge.svg)](https://github.com/zslayton/cron/actions) [![](https://img.shields.io/crates/v/cron.svg)](https://crates.io/crates/cron) [![](https://docs.rs/cron/badge.svg)](https://docs.rs/cron)
A cron expression parser.

```rust
use cron::Schedule;
use chrono::Utc;
use std::str::FromStr;

fn main() {
  //               sec  min   hour   day of month   month   day of week   year
  let expression = "0   30   9,12,15     1,15       May-Aug  Mon,Wed,Fri  2018/2";
  let schedule = Schedule::from_str(expression).unwrap();
  println!("Upcoming fire times:");
  for datetime in schedule.upcoming(Utc).take(10) {
    println!("-> {}", datetime);
  }
}

/*
Upcoming fire times:
-> 2018-06-01 09:30:00 UTC
-> 2018-06-01 12:30:00 UTC
-> 2018-06-01 15:30:00 UTC
-> 2018-06-15 09:30:00 UTC
-> 2018-06-15 12:30:00 UTC
-> 2018-06-15 15:30:00 UTC
-> 2018-08-01 09:30:00 UTC
-> 2018-08-01 12:30:00 UTC
-> 2018-08-01 15:30:00 UTC
-> 2018-08-15 09:30:00 UTC
*/
```

## Parsing With Config Options

`Schedule::from_str(...)` uses the default config.

If you need custom behavior, use the builder:

```rust
use chrono::TimeDelta;
use cron::{
    CronScheduleParts, DayOfWeekNumbering, DowDomOperand, NonexistentTimeBehavior, Schedule,
};

let schedule = Schedule::builder()
    .allowed_cron_schedule_parts(CronScheduleParts::All) // 5-, 6-, or 7-part expressions
    .day_of_week_numbering(DayOfWeekNumbering::ZeroIndexed) // Vixie-style DOW numbering
    .wraparound_ranges(true)                       // allow ranges like Nov-Mar
    .dow_dom_operand(DowDomOperand::Or)                   // combine DOM + DOW with OR
    .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent) // map skipped local times
    .search_interval(TimeDelta::days(400 * 366))          // bound search window
    .parse("30 9 1 * 1")
    .unwrap();
```

Convenience constructors:

```rust
use cron::{CronScheduleParts, Schedule};

let custom = Schedule::builder()
    .allowed_cron_schedule_parts(CronScheduleParts::FiveOrSix)
    .parse("30 9 * * Mon")
    .unwrap();

let with_year = Schedule::builder()
    .allowed_cron_schedule_parts(CronScheduleParts::Seven)
    .parse("0 0 0 1 1 * 2020/2")
    .unwrap();

let default = Schedule::default().parse("0 30 9 * * Mon").unwrap();
let vixie = Schedule::vixie().parse("0 0 0 * Nov-Mar 7-mon").unwrap();
```

Default config values:

- `cron_schedule_parts`: `CronScheduleParts::SixOrSeven`
- `day_of_week_numbering`: `DayOfWeekNumbering::OneIndexed`
- `wraparound_ranges`: `false`
- `dow_dom_operand`: `DowDomOperand::And`
- `nonexistent_time_behavior`: `NonexistentTimeBehavior::Skip`
- `search_interval`: `400 * 366` days

The optional year field is the seventh field. Open-ended year searches are bounded by the configured `search_interval`.

## Development

The minimum supported Rust version (MSRV) is 1.65.

Run these checks locally before opening a PR:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps --all-features
cargo test
cargo test --no-default-features
cargo test --all-features
cargo package --allow-dirty --list
```

Use `cargo test --all-features` to cover feature-gated code, including `serde`.

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.

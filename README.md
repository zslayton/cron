# cron [![Rust](https://github.com/zslayton/cron/workflows/Rust/badge.svg)](https://github.com/zslayton/cron/actions) [![](https://img.shields.io/crates/v/cron.svg)](https://crates.io/crates/cron) [![](https://docs.rs/cron/badge.svg)](https://docs.rs/cron)
A cron expression parser. Works with stable Rust v1.28.0.

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
use cron::{CronScheduleParts, DayOfWeekNumbering, DowDomOperand, Schedule};

let schedule = Schedule::builder()
    .allowed_cron_schedule_parts(CronScheduleParts::Both) // 5-part and 6-part expressions
    .day_of_week_numbering(DayOfWeekNumbering::ZeroToSix) // Vixie-style DOW numbering
    .dow_dom_operand(DowDomOperand::Or)                   // combine DOM + DOW with OR
    .search_interval(TimeDelta::days(400 * 366))          // bound search window
    .parse("30 9 1 * 1")
    .unwrap();
```

Convenience constructors:

```rust
use cron::{CronScheduleParts, Schedule};

let custom = Schedule::builder()
    .allowed_cron_schedule_parts(CronScheduleParts::Both)
    .parse("30 9 * * Mon")
    .unwrap();

let default = Schedule::default().parse("0 30 9 * * Mon").unwrap();
let vixie = Schedule::vixie().parse("0 0 0 * * 0").unwrap();
```

Default config values:

- `cron_schedule_parts`: `CronScheduleParts::Six`
- `day_of_week_numbering`: `DayOfWeekNumbering::OneToSeven`
- `dow_dom_operand`: `DowDomOperand::And`
- `search_interval`: `400 * 366` days

## License

Licensed under either of
 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)
at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.

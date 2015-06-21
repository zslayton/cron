# cron [![](https://api.travis-ci.org/zslayton/cron.png?branch=master)](https://travis-ci.org/zslayton/cron) [![](http://meritbadge.herokuapp.com/cron)](https://crates.io/crates/cron)
## A Cron Expression Parser in Rust

Currently pre-alpha.

```rust
fn main() {
  //                  min     hour     day  month    year
  let expression = "2,17,51 1-3,6,9-11 4,29 2,3,7 * 2015-2017";
  let schedule = CronSchedule::parse(expression).unwrap();
  println!("Upcoming fire times for '{}':", expression);
  for datetime in schedule.upcoming().take(12) {
    println!("-> {}", datetime);
  }
}
```
```
Upcoming fire times for '2,17,51 1-3,6,9-11 4,29 2,3,7 6 2015-2017':
-> 2015-07-04 01:02:00 UTC
-> 2015-07-04 01:17:00 UTC
-> 2015-07-04 01:51:00 UTC
-> 2015-07-04 02:02:00 UTC
-> 2015-07-04 02:17:00 UTC
-> 2015-07-04 02:51:00 UTC
-> 2015-07-04 03:02:00 UTC
-> 2015-07-04 03:17:00 UTC
-> 2015-07-04 03:51:00 UTC
-> 2015-07-04 06:02:00 UTC
-> 2015-07-04 06:17:00 UTC
-> 2015-07-04 06:51:00 UTC
```

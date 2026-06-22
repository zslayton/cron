use chrono::{DateTime, TimeZone, Utc};
use chrono_tz::Tz;
use criterion::{criterion_group, criterion_main, Criterion};
use cron::Schedule;
use std::hint::black_box;
use std::str::FromStr;

const PARSE_CASES: &[(&str, &str)] = &[
    ("all_fields_dense", "* * * * * * *"),
    (
        "named_ranges_and_steps",
        "0 30 9,12,15 1,15 May-Aug Mon,Wed,Fri 2018/2",
    ),
    ("shorthand_hourly", "@hourly"),
    ("sparse_year_bound", "0 0 0 29 2 * 2096"),
];

fn schedule(expression: &str) -> Schedule {
    Schedule::from_str(expression).unwrap()
}

fn tz(name: &str) -> Tz {
    name.parse().unwrap()
}

fn utc_datetime(
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, second)
        .unwrap()
}

fn tz_datetime(
    timezone: Tz,
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
) -> DateTime<Tz> {
    timezone
        .with_ymd_and_hms(year, month, day, hour, minute, second)
        .unwrap()
}

fn consume_forward<Z>(schedule: &Schedule, start: &DateTime<Z>, count: usize) -> i64
where
    Z: TimeZone,
{
    schedule
        .after(start)
        .take(count)
        .fold(0, |acc, date_time| acc ^ date_time.timestamp())
}

fn consume_reverse<Z>(schedule: &Schedule, start: &DateTime<Z>, count: usize) -> i64
where
    Z: TimeZone,
{
    schedule
        .after(start)
        .rev()
        .take(count)
        .fold(0, |acc, date_time| acc ^ date_time.timestamp())
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");
    for &(name, expression) in PARSE_CASES {
        group.bench_function(name, |b| {
            b.iter(|| Schedule::from_str(black_box(expression)).unwrap())
        });
    }
    group.finish();
}

fn bench_includes(c: &mut Criterion) {
    let schedule = schedule("0,15,30,45 0/5 8-17 ? Jan,Jun,Dec Mon-Fri 2020-2030");
    let london = tz("Europe/London");
    let date_times = [
        tz_datetime(london, 2025, 1, 6, 8, 0, 0),
        tz_datetime(london, 2025, 1, 6, 8, 5, 15),
        tz_datetime(london, 2025, 6, 11, 17, 55, 45),
        tz_datetime(london, 2025, 12, 26, 11, 30, 30),
        tz_datetime(london, 2025, 2, 6, 8, 0, 0),
        tz_datetime(london, 2025, 1, 4, 8, 0, 0),
        tz_datetime(london, 2025, 1, 6, 7, 55, 0),
        tz_datetime(london, 2031, 1, 6, 8, 0, 0),
    ];

    let mut group = c.benchmark_group("includes");
    group.bench_function("single_hit", |b| {
        let date_time = date_times[0];
        b.iter(|| schedule.includes(black_box(date_time)))
    });
    group.bench_function("mixed_hits_and_misses", |b| {
        b.iter(|| {
            let matches = date_times.iter().fold(0_u64, |matches, date_time| {
                matches + u64::from(schedule.includes(black_box(*date_time)))
            });
            black_box(matches)
        })
    });
    group.finish();
}

fn bench_dense_iteration(c: &mut Criterion) {
    let every_second = schedule("* * * * * * *");
    let every_minute = schedule("0 * * * * * *");
    let start = utc_datetime(2025, 1, 1, 0, 0, 0);
    let mut group = c.benchmark_group("dense_iteration");

    group.bench_function("every_second_forward_1024", |b| {
        b.iter(|| consume_forward(black_box(&every_second), black_box(&start), black_box(1024)))
    });
    group.bench_function("every_second_reverse_1024", |b| {
        b.iter(|| consume_reverse(black_box(&every_second), black_box(&start), black_box(1024)))
    });
    group.bench_function("every_minute_forward_1024", |b| {
        b.iter(|| consume_forward(black_box(&every_minute), black_box(&start), black_box(1024)))
    });

    group.finish();
}

fn bench_sparse_iteration(c: &mut Criterion) {
    let leap_day = schedule("0 0 0 29 2 * *");
    let distant_year = schedule("0 0 0 1 1 * 2100");
    let impossible_february_day = schedule("0 0 0 31 2 * *");
    let start = utc_datetime(1970, 1, 1, 0, 0, 0);
    let reverse_start = utc_datetime(2101, 1, 1, 0, 0, 0);
    let mut group = c.benchmark_group("sparse_year_bound_iteration");

    group.bench_function("leap_day_forward_16", |b| {
        b.iter(|| consume_forward(black_box(&leap_day), black_box(&start), black_box(16)))
    });
    group.bench_function("leap_day_reverse_16", |b| {
        b.iter(|| {
            consume_reverse(
                black_box(&leap_day),
                black_box(&reverse_start),
                black_box(16),
            )
        })
    });
    group.bench_function("distant_year_next_from_1970", |b| {
        b.iter(|| {
            let next = distant_year.after(black_box(&start)).next();
            black_box(next)
        })
    });
    group.bench_function("impossible_date_exhausts_years", |b| {
        b.iter(|| {
            let next = impossible_february_day.after(black_box(&start)).next();
            black_box(next)
        })
    });

    group.finish();
}

fn bench_dst_iteration(c: &mut Criterion) {
    let los_angeles = tz("America/Los_Angeles");
    let berlin = tz("Europe/Berlin");
    let hourly = schedule("0 0 * * * * *");
    let every_15_minutes = schedule("0 0/15 * * * * *");
    let nonexistent_daily_time = schedule("0 30 2 * * * *");
    let los_angeles_fall_back_start = tz_datetime(los_angeles, 2022, 11, 6, 0, 30, 0);
    let los_angeles_fall_back_reverse_start = tz_datetime(los_angeles, 2022, 11, 6, 4, 30, 0);
    let los_angeles_spring_forward_start = tz_datetime(los_angeles, 2022, 3, 13, 0, 30, 0);
    let berlin_fall_back_start = tz_datetime(berlin, 2022, 10, 30, 1, 30, 0);
    let nonexistent_start = tz_datetime(los_angeles, 2022, 3, 12, 0, 0, 0);
    let mut group = c.benchmark_group("dst_nonexistent_time_iteration");

    group.bench_function("spring_forward_hourly_forward_8", |b| {
        b.iter(|| {
            consume_forward(
                black_box(&hourly),
                black_box(&los_angeles_spring_forward_start),
                black_box(8),
            )
        })
    });
    group.bench_function("fall_back_subhourly_forward_12", |b| {
        b.iter(|| {
            consume_forward(
                black_box(&every_15_minutes),
                black_box(&los_angeles_fall_back_start),
                black_box(12),
            )
        })
    });
    group.bench_function("fall_back_subhourly_reverse_12", |b| {
        b.iter(|| {
            consume_reverse(
                black_box(&every_15_minutes),
                black_box(&los_angeles_fall_back_reverse_start),
                black_box(12),
            )
        })
    });
    group.bench_function("berlin_fall_back_full_repeated_hour", |b| {
        b.iter(|| {
            consume_forward(
                black_box(&every_15_minutes),
                black_box(&berlin_fall_back_start),
                black_box(12),
            )
        })
    });
    group.bench_function("nonexistent_0230_daily_skips_gap", |b| {
        b.iter(|| {
            consume_forward(
                black_box(&nonexistent_daily_time),
                black_box(&nonexistent_start),
                black_box(4),
            )
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_includes,
    bench_dense_iteration,
    bench_sparse_iteration,
    bench_dst_iteration
);
criterion_main!(benches);

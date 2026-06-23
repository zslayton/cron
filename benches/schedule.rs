use chrono::{DateTime, TimeDelta, TimeZone, Utc};
use chrono_tz::America::Los_Angeles;
use chrono_tz::Australia::Lord_Howe;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use cron::{CronScheduleParts, DowDomOperand, NonexistentTimeBehavior, Schedule};
use std::str::FromStr;

const DENSE_EVENT_COUNT: usize = 2_000;
const SPARSE_EVENT_COUNT: usize = 16;
const DST_EVENT_COUNT: usize = 8;

fn nth_match<Z>(schedule: &Schedule, start: &DateTime<Z>, count: usize) -> Option<DateTime<Z>>
where
    Z: TimeZone + 'static,
{
    schedule.after(start).take(count).last()
}

fn bench_parse(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse");

    for (name, expression) in [
        ("every_second", "* * * * * * *"),
        (
            "named_ranges",
            "0 30 9,12,15 1,15 May-Aug Mon,Wed,Fri 2018/2",
        ),
        ("year_bound", "0 0 0 29 2 * 2024-2096/4"),
    ] {
        group.bench_with_input(
            BenchmarkId::from_parameter(name),
            &expression,
            |b, expression| b.iter(|| Schedule::from_str(black_box(*expression)).unwrap()),
        );
    }

    group.bench_function("vixie_special_fields", |b| {
        b.iter(|| {
            Schedule::vixie()
                .parse(black_box("0 0 2 L,15W Jan,Mar Mon#2 2024/2"))
                .unwrap()
        })
    });

    group.bench_function("five_part_config", |b| {
        b.iter(|| {
            Schedule::builder()
                .allowed_cron_schedule_parts(CronScheduleParts::Five)
                .parse(black_box("30 9 * * Mon"))
                .unwrap()
        })
    });

    group.finish();
}

fn bench_includes(c: &mut Criterion) {
    let weekday_window = Schedule::from_str("0 0/15 9-17 * * Mon-Fri *").unwrap();
    let window_hit = Utc.with_ymd_and_hms(2024, 6, 17, 9, 15, 0).unwrap();
    let window_miss = Utc.with_ymd_and_hms(2024, 6, 17, 18, 0, 0).unwrap();

    let vixie_special = Schedule::vixie()
        .days_matching(DowDomOperand::Or)
        .parse("0 0 9 L * Mon#1 *")
        .unwrap();
    let special_hit = Utc.with_ymd_and_hms(2024, 2, 29, 9, 0, 0).unwrap();
    let special_miss = Utc.with_ymd_and_hms(2024, 2, 28, 9, 0, 0).unwrap();

    let mut group = c.benchmark_group("includes");
    group.bench_function("weekday_window_hit", |b| {
        b.iter(|| weekday_window.includes(black_box(window_hit)))
    });
    group.bench_function("weekday_window_miss", |b| {
        b.iter(|| weekday_window.includes(black_box(window_miss)))
    });
    group.bench_function("vixie_special_hit", |b| {
        b.iter(|| vixie_special.includes(black_box(special_hit)))
    });
    group.bench_function("vixie_special_miss", |b| {
        b.iter(|| vixie_special.includes(black_box(special_miss)))
    });
    group.finish();
}

fn bench_dense_iteration(c: &mut Criterion) {
    let every_second = Schedule::from_str("* * * * * * *").unwrap();
    let every_15_seconds = Schedule::from_str("0/15 * * * * * *").unwrap();
    let start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

    let mut group = c.benchmark_group("iteration_dense");
    group.throughput(Throughput::Elements(DENSE_EVENT_COUNT as u64));
    group.bench_function("every_second_forward", |b| {
        b.iter(|| {
            black_box(nth_match(
                &every_second,
                black_box(&start),
                DENSE_EVENT_COUNT,
            ))
        })
    });
    group.bench_function("every_15_seconds_forward", |b| {
        b.iter(|| {
            black_box(nth_match(
                &every_15_seconds,
                black_box(&start),
                DENSE_EVENT_COUNT,
            ))
        })
    });
    group.finish();
}

fn bench_sparse_and_year_bound_iteration(c: &mut Criterion) {
    let leap_days = Schedule::from_str("0 0 0 29 2 * 2024-2096/4").unwrap();
    let leap_start = Utc.with_ymd_and_hms(2023, 1, 1, 0, 0, 0).unwrap();

    let monthly = Schedule::from_str("0 0 0 1 * * *").unwrap();
    let monthly_start = Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap();

    let far_future = Schedule::from_str("0 0 0 1 1 * 2099").unwrap();
    let far_future_start = Utc.with_ymd_and_hms(1970, 1, 1, 0, 0, 0).unwrap();

    let bounded_search_miss = Schedule::builder()
        .search_interval(TimeDelta::days(365 * 20))
        .parse("0 0 0 1 1 * 2099")
        .unwrap();

    let or_day_scan = Schedule::vixie().parse("0 0 0 31 * Mon 2099").unwrap();

    let mut group = c.benchmark_group("iteration_sparse_year_bound");
    group.throughput(Throughput::Elements(SPARSE_EVENT_COUNT as u64));
    group.bench_function("leap_days_forward", |b| {
        b.iter(|| {
            black_box(nth_match(
                &leap_days,
                black_box(&leap_start),
                SPARSE_EVENT_COUNT,
            ))
        })
    });
    group.bench_function("monthly_forward", |b| {
        b.iter(|| {
            black_box(nth_match(
                &monthly,
                black_box(&monthly_start),
                SPARSE_EVENT_COUNT,
            ))
        })
    });

    group.throughput(Throughput::Elements(1));
    group.bench_function("far_future_single_match", |b| {
        b.iter(|| black_box(far_future.after(black_box(&far_future_start)).next()))
    });
    group.bench_function("far_future_bounded_miss", |b| {
        b.iter(|| {
            black_box(
                bounded_search_miss
                    .after(black_box(&far_future_start))
                    .next(),
            )
        })
    });
    group.bench_function("or_semantics_day_scan", |b| {
        b.iter(|| black_box(or_day_scan.after(black_box(&far_future_start)).next()))
    });
    group.finish();
}

fn bench_dst_and_nonexistent_time(c: &mut Criterion) {
    let hourly = Schedule::from_str("0 0 * * * * *").unwrap();
    let subhourly = Schedule::from_str("0 */15 * * * * *").unwrap();
    let nonexistent_next = Schedule::builder()
        .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
        .parse("0 30 2 * * * *")
        .unwrap();
    let lord_howe_nonexistent_next = Schedule::builder()
        .nonexistent_time_behavior(NonexistentTimeBehavior::NextExistent)
        .parse("0 15 2 * * * *")
        .unwrap();

    let fall_back_start = Los_Angeles
        .with_ymd_and_hms(2022, 11, 6, 0, 59, 59)
        .unwrap();
    let spring_forward_start = Los_Angeles
        .with_ymd_and_hms(2022, 3, 13, 0, 59, 59)
        .unwrap();
    let nonexistent_start = Los_Angeles
        .with_ymd_and_hms(2022, 3, 13, 1, 59, 59)
        .unwrap();
    let lord_howe_spring_forward_start =
        Lord_Howe.with_ymd_and_hms(2022, 10, 2, 1, 59, 59).unwrap();

    let mut group = c.benchmark_group("iteration_dst_nonexistent");
    group.throughput(Throughput::Elements(DST_EVENT_COUNT as u64));
    group.bench_function("hourly_fall_back_fold", |b| {
        b.iter(|| {
            black_box(nth_match(
                &hourly,
                black_box(&fall_back_start),
                DST_EVENT_COUNT,
            ))
        })
    });
    group.bench_function("hourly_spring_forward_gap_skip", |b| {
        b.iter(|| {
            black_box(nth_match(
                &hourly,
                black_box(&spring_forward_start),
                DST_EVENT_COUNT,
            ))
        })
    });
    group.bench_function("subhourly_fall_back_fold", |b| {
        b.iter(|| {
            black_box(nth_match(
                &subhourly,
                black_box(&fall_back_start),
                DST_EVENT_COUNT,
            ))
        })
    });
    group.bench_function("daily_nonexistent_next_existent", |b| {
        b.iter(|| {
            black_box(nth_match(
                &nonexistent_next,
                black_box(&nonexistent_start),
                DST_EVENT_COUNT,
            ))
        })
    });
    group.bench_function("daily_lord_howe_spring_forward_gap", |b| {
        b.iter(|| {
            black_box(nth_match(
                &lord_howe_nonexistent_next,
                black_box(&lord_howe_spring_forward_start),
                DST_EVENT_COUNT,
            ))
        })
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_parse,
    bench_includes,
    bench_dense_iteration,
    bench_sparse_and_year_bound_iteration,
    bench_dst_and_nonexistent_time
);
criterion_main!(benches);

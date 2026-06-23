# AGENTS.md

## Repository Layout

This is a Rust library crate for parsing cron expressions and iterating matching schedules.

Core code lives under `src/`:

- `src/lib.rs`: public API surface and crate exports.
- `src/schedule.rs`: `Schedule` construction, iteration, display, serde support, and date-time matching behavior.
- `src/config.rs`: builder/configuration types such as cron field modes, day matching, wraparound ranges, and nonexistent-time behavior.
- `src/parsing.rs`: cron expression parser.
- `src/queries.rs`: schedule query/iteration helpers.
- `src/time_unit/`: per-field cron logic for seconds, minutes, hours, days, months, weekdays, and years.
- `src/ordinal.rs`: compact ordinal set/range representation used by time units.

Tests live in `tests/lib.rs` and inline `#[cfg(test)]` modules in `src/`. Prefer adding behavior and integration coverage in `tests/lib.rs` unless a narrow unit test beside the affected module is clearly better.

Benchmarks live in `benches/schedule.rs`.

## Local Checks

Run these before considering work complete:

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo doc --no-deps --all-features
cargo test
cargo test --no-default-features
cargo test --all-features
cargo bench --no-run
cargo package --allow-dirty --list
```

Use `cargo test --all-features` to cover optional feature-gated code such as `serde`.

For performance-sensitive changes, compare Criterion results against a saved baseline:

```sh
cargo bench --bench schedule -- --save-baseline before
cargo bench --bench schedule -- --baseline before
```

When the worktree is dirty, use a temp copy for before/after benchmark comparisons so local changes and Criterion output do not contaminate the repo.

## MSRV

The crate declares `rust-version = "1.65"`.

Keep code compatible with Rust 1.65. Avoid newer standard library APIs even when clippy suggests them. Recent examples that are not MSRV-compatible:

- `Option::is_some_and`, stabilized after 1.65. Use `map_or(false, ...)`.
- integer `.is_multiple_of(...)`, stabilized after 1.65. Use `% rhs == 0`.

`cargo clippy --all-targets --all-features -- -D warnings` will flag some incompatible APIs once `rust-version` is set.

The full dev-dependency graph may not resolve cleanly with Cargo/Rust 1.65 because benchmark tooling can pull newer transitive dependencies. For MSRV checks, prefer verifying the library as a downstream dependency rather than resolving all dev-dependencies.

## Dependency And CI Notes

`Cargo.lock` is ignored for this library crate. Cargo may generate it locally during resolution or package checks; do not stage it unless the project policy changes.

Dependabot is configured for Cargo and GitHub Actions. Minor and patch updates are grouped; major updates are left separate.

Avoid adding CI-only checks that cannot also be run locally without extra tools. In particular, do not add `cargo-deny` CI unless the repo also commits to making `cargo deny check` part of the local developer workflow.

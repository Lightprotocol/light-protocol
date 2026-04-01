# Autoresearch: migrate to Solana 3 + Anchor 1 + Pinocchio 0.10

## Objective
Finish the in-flight dependency migration to Solana 3.x, Anchor 1.x, and Pinocchio 0.10 across the Rust workspace. Primary goal is reducing migration breakage to zero without cheating on validation. Changes should preserve behavior and keep representative migration-sensitive tests green.

## Metrics
- **Primary**: `migration_failures` (count, lower is better) — number of failing commands in `./autoresearch.sh`
- **Secondary**:
  - `passing_commands` — commands passing in the fast migration suite
  - `suite_seconds` — wall clock seconds spent in the fast suite

## How to Run
`./autoresearch.sh`

The script runs a focused fast suite of compile-heavy migration checks and emits structured `METRIC` lines.

## Files in Scope
- `Cargo.toml`, `Cargo.lock` — workspace dependency versions and feature wiring
- `program-libs/**` — shared libraries affected by Solana/Pinocchio API changes
- `programs/**` — on-chain programs, especially pinocchio/native codepaths
- `sdk-libs/**` — SDK crates and macros impacted by API/type changes
- `program-tests/**`, `sdk-tests/**` — migration-sensitive tests and fixtures
- `Anchor.toml` — Anchor config if migration requires it

## Off Limits
- Benchmark cheating: do not weaken tests, skip real migration checks, or paper over failures with cfg hacks that reduce coverage.
- Unrelated refactors.
- External vendored code unless required to unblock the migration and clearly justified.

## Constraints
- Prefer real API migrations over compatibility shims.
- Keep the benchmark representative; don’t optimize only for the fast suite if it harms broader correctness.
- `autoresearch.checks.sh` is the correctness gate when the fast suite reaches zero failures.
- All final kept changes should move the repo toward genuinely green tests.

## What's Been Tried
- Session started on top of an in-flight migration branch with many existing edits already present.
- First manual baseline `cargo check` revealed obvious bad automated replacements in `program-libs/account-checks` (e.g. `AccountView as AccountInfo` appearing in type/expression positions).

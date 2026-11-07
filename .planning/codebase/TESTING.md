# Testing Patterns

**Analysis Date:** 2026-11-07

## Test Framework
**Runner:** Cargo built-in harness, configuration in `Cargo.toml`; Swift executable self-tests.
**Assertion Library:** Rust assert macros; Swift preconditions/rejection helpers.
**Run Commands:** `cargo test --locked`; `cargo fmt --check`; `cargo clippy --locked --all-targets`; `sh scripts/release.sh`.
Bundling executes `ShelfUpdater --self-test`; `--verify-release <assets> <public-key-file>` verifies real signed release output without installation.

## Test File Organization
**Location:** `#[cfg(test)] mod tests` in source files; Swift tests in `native/Updater.swift`.
**Naming:** Behavior names such as `model_apply_preserves_agent_choices_and_rejects_flags_without_saving` (`src/settings.rs`).
**Structure:** Tests exercise observable state and saved output, not private call order.

## Test Structure
**Suite Organization:** Each module owns its fixtures and assertions.
**Patterns:** Unique temp settings/run paths; Rune Runtime pointer actions; explicit fixture removal.

## Mocking
**Framework:** Generated harmless executable stubs in `src/agent.rs`, not real model requests.
**Patterns:** Feed ACP/RPC responses, unsupported flags, empty/nonzero results, delayed mode readiness and exact session output.
**What to Mock:** Paid inference, provider processes and unrelated disk content.
**What NOT to Mock:** Reader path/descriptor boundaries, consent-setting persistence, signature/hash rejection.

## Fixtures and Factories
**Test Data:** `catalog::fixture()` and app fixture state; `prefs::path_for_state()` isolates test preferences.
**Location:** `src/catalog.rs`, `src/main.rs`, `src/prefs.rs` and co-located tests.

## Coverage
**Requirements:** No percentage threshold configured. Last local full run: 44 Rust tests passed; native verification tests passed.
**View Coverage:** No coverage-report command configured; do not invent coverage percentages.

## Test Types
**Unit Tests:** Levels, protections, model preferences, UI controls, geometry and release verification.
**Integration Tests:** Stub protocol/session lifecycle, broker scope/revocation/symlink behavior; one macOS live catalog/gauge test.
**E2E Tests:** Headless screenshots and controls are exercised, but real native model turns, tool routing, Terminal continuation and full app replacement/restart remain unverified.

## Common Patterns
**Async Testing:** Poll jobs with finite deadlines; production cancellation/deadline errors stay recoverable (`src/agent.rs`).
**Error Testing:** Assert selection/results remain intact and authority does not widen; use disposable signature keys and rollback fixtures, never the owner's installed app.

*Testing analysis: 2026-11-07*

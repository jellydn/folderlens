# Coding Conventions

**Analysis Date:** 2026-11-09

## Naming Patterns

**Files:** Rust modules use snake_case; private module paths are explicit (`src/agent.rs`, `src/catalog.rs`, `src/settings.rs`).

**Functions:** Behavior names use snake_case, such as `is_ticked`, `ask_access`, `prepare_run` and `resume_run`. Tests state an observable rule, such as `untick_holds_until_the_level_changes` (`src/main.rs`).

**Variables:** Domain names distinguish `initial`/`current` access, saved context, current selection, and exact sessions (`src/broker.rs`, `src/agent.rs`).

**Types:** PascalCase data/owners: `State`, `Catalog`, `Prefs`, `Settings`, `Job`, `Session`, `Access`, `Reader`, `Launch` and `Manifest`. Constants use SCREAMING_SNAKE_CASE.

## Code Style

**Formatting:** Rust uses `cargo fmt`; no custom rustfmt file. Swift source uses two-space indentation; the Pi extension uses tabs and an imported SDK JSDoc type. Follow each file's current format.

**Linting:** CI runs `cargo fmt --check` and `cargo clippy --locked --all-targets`. Changes to `src/pi-reader.mjs` also need installed SDK type validation, lint and Biome (`AGENTS.md`). No repository JS toolchain/config is present.

## Import Organization

**Order:** Standard library groups, third-party crates, then crate/super modules (`src/agent.rs`, `src/broker.rs`). Narrow platform-specific imports stay inside `#[cfg]` code where useful.

**Path Aliases:** No application aliases. `#[path = "…"]` declares private ownership, not re-export barrels. `src/settings_updates.rs` uses its parent Settings helpers.

## Error Handling

**Patterns:** Recoverable Rust operations use `Result<_, String>` or fixed `&'static str` messages, `?`, and early returns. Missing live measurements use conservative fallbacks. Tests use `unwrap`/assertions on controlled fixtures (`src/agent.rs`, `src/catalog.rs`).

Unsupported providers, unavailable models and unverified OpenCode versions fail before a prompt. Provider errors are redacted before display. Swift uses typed `UpdateError` and fixed safe user messages, with no unsigned fallback (`src/provider.rs`, `native/Updater.swift`).

## Logging

**Framework:** No logging/telemetry dependency in `Cargo.toml`.

**Patterns:** Per-run `agent.log` contains redacted diagnostics. Lines with likely secrets, paths or email addresses are withheld. Do not log saved context, private signing input or raw transport errors (`src/agent.rs`, `scripts/sign-release.py`, `native/Updater.swift`).

## Comments

**When to Comment:** Module docs explain ownership and constraints. Inline comments explain ordering/safety: original-scope freezing, descriptor checks, private-mode readiness, atomic completion and installation provenance (`src/broker.rs`, `src/provider.rs`, `src/agent.rs`, `native/Updater.swift`).

**JSDoc/TSDoc:** `src/pi-reader.mjs` imports the `ExtensionAPI` type from `@earendil-works/pi-coding-agent` in JSDoc. No TypeScript source or generated API docs.

## Function Design

**Size:** Views use composable `El` builders; helpers own coherent domain work. The larger files combine production and co-located tests. Do not extract helpers only to reduce line counts (`src/main.rs`, `src/agent.rs`, `src/settings.rs`).

**Parameters:** Views read state; input handlers mutate typed state and `Tx`. Workers send updates through `AppHandle`. Process arguments use `Command::arg/args`; shell launchers quote each argument/environment value (`src/prefs.rs`).

**Return Values:** Authority checks return denial, not fallback access. Reviews return only nonempty completed output. Record choice changes only an in-memory flag and never deletes (`src/agent.rs`, `src/main.rs`).

## Module Design

**Exports:** App actions are `pub`; child module internals generally use `pub(super)` or private visibility. Scan is owned by catalog; provider/broker by agent; update settings by Settings. Keep security policy with its owner (`AGENTS.md`).

**Barrel Files:** None. Tests live beside their owner except the executable continuation test. Accepted cross-module contracts are recorded in `docs/adr/0001-scoped-agent-sessions.md` and `docs/adr/0002-publisher-verified-updates.md`.

---

*Convention analysis: 2026-11-09*

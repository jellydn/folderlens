# Coding Conventions

**Analysis Date:** 2026-11-14

## Naming Patterns

**Files:**
- Rust modules use snake_case, one file per owner: `src/agent.rs`, `src/catalog.rs`, `src/settings.rs`, `src/status.rs`, `src/notch.rs`.
- Child modules are separate files that the owner includes with `#[path]`, not files that `src/main.rs` declares: `src/scan.rs` (from `src/catalog.rs`), `src/broker.rs` and `src/provider.rs` (from `src/agent.rs`), `src/settings_updates.rs` (as `mod updates` in `src/settings.rs`). Do not add `mod scan` to `src/main.rs` (`AGENTS.md`).
- Non-Rust sources: `src/pi-reader.mjs` (Pi extension), `native/Updater.swift` (update helper), `scripts/*.sh` and `scripts/*.py`.

**Functions:**
- snake_case verbs or rules: `apply_level`, `is_ticked`, `request_scan`, `finish_scan`, `launch_size` (`src/main.rs`); `prepare_run`, `resume_run`, `path_for_state` (`src/prefs.rs`); `redact_diagnostics` (`src/agent.rs`).
- Test names state an observable rule as a sentence: `a_locked_row_stays_unticked_and_confirm_deletes_nothing`, `untick_holds_until_the_level_changes` (`src/main.rs`), `reads_require_original_and_current_selection_and_exclude_blocked_paths` (`src/broker.rs`).

**Variables:**
- Short domain words, not type names: `s` for app state in handlers (`src/main.rs`), `rt` for a test runtime, `tx` for the Rune `Tx`, `root`/`bin` for test fixture directories and stub executables (`src/agent.rs`).
- UI element keys are kebab-case strings that tests query: `"candidate-estimate"`, `"confirm"`, `"action-ask"`, `"row-spotify"`, `"check-cache-held"` (`src/main.rs`).

**Types:**
- PascalCase structs/enums: `State`, `Usage` (`src/main.rs`), `Catalog`, `Leaf`, `Group`, `Level` (`src/catalog.rs`), `Job`, `Access` (`src/agent.rs`), `Reader` (`src/broker.rs`), `Launch`, `Provider` (`src/provider.rs`), `Settings` (`src/settings.rs`).
- Constants use SCREAMING_SNAKE_CASE: `MIN_PANEL_HEIGHT`, `LINE` (`src/main.rs`); `RUNNING`, `CANCELLED`, `FINALIZING` (`src/agent.rs`); theme colors `INK`, `MUTED`, `PANEL` (`src/theme.rs`); `prefs::ALWAYS` protected roots (`src/prefs.rs`).
- Field doc comments explain state meaning, for example `ticks: Vec<Option<bool>>` — "None follows the level. Some is a tick the user set." (`src/main.rs`).

## Code Style

**Formatting:**
- `cargo fmt` with default settings; there is no `rustfmt.toml`. CI runs `cargo fmt --check` (`.github/workflows/ci.yml`). The check passes at 0c68acb.
- Toolchain is pinned to Rust 1.99.1 with `clippy` and `rustfmt` components (`rust-toolchain.toml`); edition 2021 (`Cargo.toml`).
- JS uses tabs and double quotes (`src/pi-reader.mjs`, `tests/pi-reader.mjs`). Swift uses two-space indentation (`native/Updater.swift`). Follow each file's current format.

**Linting:**
- `cargo clippy --locked --all-targets` in CI (`.github/workflows/ci.yml`, `.github/workflows/release.yml`). There is no `clippy.toml` and no crate-level lint attributes. Commit 0c68acb ("refactor: clear the existing clippy warnings") made the run warning-free; keep it that way.
- The only `#[allow]` attribute is `#[allow(dead_code)]` on `Group.size_label` in `src/catalog.rs`.
- `src/pi-reader.mjs` has no checked-in JS toolchain (no `package.json`, Biome or ESLint config). Validate changes against the installed Pi SDK: types, lint, Biome (`AGENTS.md`).

## Import Organization

**Order:**
1. `std` imports, grouped by module (`std::io`, `std::path`, `std::sync`, `std::time`).
2. Third-party crates: `serde_json`, `rune`.
3. Crate or parent modules: `use crate::prefs;`, `use super::Access;`, then `#[path]` child modules and their `use` lines.

Examples: `src/agent.rs`, `src/broker.rs`, `src/main.rs`. `cargo fmt` sorts within each group; blank lines separate the groups.

**Path Aliases:**
- None. `#[path = "…"]` sets private module ownership only (`src/catalog.rs`, `src/agent.rs`, `src/settings.rs`).
- macOS-only code and dependencies are gated with `#[cfg(target_os = "macos")]` (`src/status.rs`, `Cargo.toml` `block2`/`libc`).

## Error Handling

**Patterns:**
- Recoverable operations return `Result<_, String>` or `Result<_, &'static str>` and use `?` and early returns: `Job::start_in`, `validate_model` (`src/agent.rs`), `Provider::for_bin`, `verify_opencode` (`src/provider.rs`), `prepare_run` (`src/prefs.rs`), `update::check` (`src/update.rs`). There is no `anyhow`/`thiserror` dependency (`Cargo.toml`).
- Error strings are shown to users as written. Each one says what failed, what to check, and that nothing changed. For example: "…No review was sent and no fallback was selected." (`src/agent.rs`), "…FolderLens will not switch agents." (`src/provider.rs`), "Agent ended without an answer (…) … FolderLens did not change your selection." (`src/agent.rs`). Write new messages in this form.
- Unsupported providers, unavailable models and unverified OpenCode versions fail before a prompt is sent (`src/provider.rs`, `src/agent.rs`).
- Missing live measurements fall back to the last known value, for example `cpu.unwrap_or(s.catalog.cpu)` in `start_workers` (`src/main.rs`). A failed catalog sets `missing` and the UI shows "—" (`src/main.rs`, `src/catalog.rs`).
- Swift uses a typed `enum UpdateError: Error, LocalizedError` with fixed user messages and no unsigned fallback (`native/Updater.swift`).
- Tests use `unwrap`, `expect` and `unwrap_or_else(|| panic!(…))` only on controlled fixtures (`src/main.rs`, `src/agent.rs`).

## Logging

**Framework:** None. No logging or telemetry crate (`Cargo.toml`).

**Patterns:**
- Each agent run writes a per-run `agent.log` (`src/agent.rs`). `redact_diagnostics` replaces lines with possible credentials or local paths with fixed "[… was withheld.]" text (`src/agent.rs`). Do not put local paths or credentials in error logs (`AGENTS.md`).
- `println!` is used only for the broker's JSON-RPC responses on stdout (`src/broker.rs`). `eprintln!` is used only for the OpenCode resume helper's failure (`src/provider.rs`).

## Comments

**When to Comment:**
- Module `//!` docs state ownership and the safety contract: "Record choice deletes nothing" (`src/main.rs`), "A bounded, read-only ACP client…" (`src/agent.rs`), the tier and lock rules (`src/catalog.rs`).
- Inline comments explain why: thread rules ("AppKit pointer tracking stays on the UI thread; system commands do not." in `src/main.rs`), import ordering ("Publish a complete copy without overwriting settings from another process." in `src/prefs.rs`), and fixture meaning in tests ("Build output was used under an hour ago, so Aggressive leaves it unticked." in `src/main.rs`).

**JSDoc/TSDoc:**
- `src/pi-reader.mjs` imports the `ExtensionAPI` type from `@earendil-works/pi-coding-agent` in one JSDoc `@param`. There is no TypeScript source and no generated API docs.

## Function Design

**Size:** Views are long builder chains; domain helpers stay focused. Large files keep production code and co-located tests together (`src/main.rs` 2,628 lines, of which `mod tests` starts at line 1653; `src/settings.rs` 1,520; `src/status.rs` 1,382; `src/agent.rs` 1,217). Do not split helpers only to reduce line counts.

**Builder-style UI:** Rune views are pure functions `fn view(s: &State, cx: &mut Cx<State>) -> El` (`src/main.rs`, `src/settings.rs`, `src/notch.rs`) that chain `col`/`row`/`text` with `.child(…)`, `.key(…)`, `.font(…)`, `.color(paint(…))`, `.wrap()`, `.w_full()` and `.scroll(ScrollSpec { … })`. Every element that a test reads or clicks gets a `.key("kebab-name")`.

**Parameters:** Views only read state. Input handlers take `(&mut State, &KeyEvent, &mut Tx)` (`on_key` in `src/main.rs`, `src/palette.rs`). Workers send results back through `rune::AppHandle::update`/`update_if` on Rune's UI queue (`start_workers` in `src/main.rs`); do not move system commands into `poll` (`AGENTS.md`). Process arguments use `Command::arg`/`args`, never string interpolation; models go through `--model` where supported (`src/provider.rs`, `AGENTS.md`).

**Return Values:** Authority checks return a denial, not fallback access (`src/broker.rs`). Reviews return only nonempty completed output; empty output is an error (`src/agent.rs`). `poll` returns `bool` for "state changed" (`src/main.rs`).

**No deletion path:** Record choice (key `"confirm"`) only sets `s.confirmed = true` in memory (`src/main.rs`). Production code has no `remove_dir`/`trash` call on scanned paths. The only production `remove_file` deletes FolderLens's own temporary import file (`src/prefs.rs`). Do not add a delete or cleanup path (`AGENTS.md`).

**Estimate wording:** Sizes are labelled as estimates and never as reclaimed space: "Estimated · review only, not reclaimed" and "Estimated {} · review only, not reclaimed. FolderLens deletes nothing." (`src/main.rs`), "Estimated cleanup candidates {}, not reclaimed space" and "Cleanup candidates · estimated" (`src/notch.rs`). Keep this wording in new UI (`AGENTS.md`).

## Module Design

**Exports:** `src/main.rs` declares the top-level modules (`agent`, `catalog`, `notch`, `palette`, `prefs`, `settings`, `status`, `theme`, `update`). Items used across modules are `pub`. Child-module internals use `pub(super)`, for example `pub(super) struct Reader` (`src/broker.rs`) and `pub(super) fn view` (`src/settings_updates.rs`). Security policy stays with its owner: scan in catalog, broker/provider in agent, update settings in Settings.

**`cfg(test)` splits:**
- Test-only helpers live outside `mod tests` when other modules need them: `fixture_state()` and `ticked_ids()` in `src/main.rs`; `src/notch.rs` tests call `crate::fixture_state()`.
- Paired functions swap behavior for tests: `#[cfg(test)] pub fn path_for_state()` returns a unique temp file `folderlens-prefs-<pid>-<n>.txt`; `#[cfg(not(test))]` returns the real `settings.txt` (`src/prefs.rs`). `choose_folder()` returns `None` under `cfg(test)` and runs the `osascript` folder picker otherwise (`src/prefs.rs`).
- `catalog::fixture()` is not `cfg(test)`. Headless `*-example` scenes also use it (`src/catalog.rs`, `src/main.rs`, `src/notch.rs`).

**Barrel Files:** None. Tests are co-located `#[cfg(test)] mod tests` blocks in every Rust module except `src/palette.rs` and `src/provider.rs`; `src/status.rs` has two (one inside its macOS module). `tests/opencode_continuation.rs` is the only Rust integration test. Cross-module contracts are recorded in `docs/adr/0001-scoped-agent-sessions.md` and `docs/adr/0002-publisher-verified-updates.md`.

---

*Convention analysis: 2026-11-14*

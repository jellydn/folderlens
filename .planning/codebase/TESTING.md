# Testing Patterns

**Analysis Date:** 2026-11-14

## Test Framework

**Runner:**
- Cargo's built-in Rust test harness on the pinned toolchain Rust 1.99.1 (`rust-toolchain.toml`, `Cargo.toml`). There is no dev-dependency and no test crate.
- Rune's headless `rune::Runtime` drives the UI in tests (`src/main.rs`, `src/settings.rs`, `src/notch.rs`). Rune is pinned by git `rev` (`Cargo.toml`).
- Node's built-in `node:test` runs the Pi reader test (`tests/pi-reader.mjs`). There is no `package.json`.
- Swift self-test inside the update helper: `FolderLensUpdater --self-test` (`native/Updater.swift` `selfTest()`, run by `scripts/bundle.sh`).
- Config: none beyond `Cargo.toml` and `rust-toolchain.toml`. Full checks need macOS (`AGENTS.md`).

**Assertion Library:**
- Rust `assert!`, `assert_eq!`, with format messages such as `"{width}: {key}: {rect:?}"` (`src/main.rs`).
- `node:assert/strict` (`tests/pi-reader.mjs`).
- Swift `precondition` and a local `reject` helper (`native/Updater.swift`).

**Run Commands:**
```bash
cargo test --locked                          # All Rust tests, headless (73 at 0c68acb)
cargo test --locked oversized_protocol_lines  # One test by name filter
node --test tests/pi-reader.mjs              # Pi reader boundary (1 test)
cargo fmt --check                            # Formatting gate
cargo clippy --locked --all-targets          # Lint gate; warning-free since 0c68acb
sh scripts/release.sh                        # Bundle (runs the Swift self-test) and package; no upload
```
No watch mode and no coverage command exist.

**Counts:** `rg -c '#\[test\]' src tests` gives 73 Rust tests: `src/main.rs` 26, `src/agent.rs` 9, `src/status.rs` 8, `src/prefs.rs` 7, `src/settings.rs` 7, `src/catalog.rs` 4, `src/scan.rs` 3, `src/broker.rs` 3, `src/notch.rs` 2, `src/settings_updates.rs` 1, `src/update.rs` 1, `src/theme.rs` 1, `tests/opencode_continuation.rs` 1. On 2026-11-14 on macOS, `cargo test --locked` passed 72 binary tests and 1 integration test, 0 ignored; `node --test tests/pi-reader.mjs` passed 1; `cargo fmt --check` and `cargo clippy --locked --all-targets` gave no output or warnings.

## Test File Organization

**Location:**
- Co-located `#[cfg(test)] mod tests` at the end of each owner module (`src/main.rs` line 1653, `src/agent.rs`, `src/settings.rs`, …). `src/palette.rs` and `src/provider.rs` have no test module. Their behavior is tested through `src/main.rs` and `src/agent.rs`.
- `src/status.rs` has two test modules: one inside `mod macos` and one at file level.
- `tests/opencode_continuation.rs` is the only Cargo integration test. It runs the built binary through `env!("CARGO_BIN_EXE_folderlens")`.
- `tests/pi-reader.mjs` imports `../src/pi-reader.mjs`.

**Naming:**
- Rule sentences in snake_case: `quick_review_keeps_recording_visible_and_non_destructive`, `agent_progress_and_results_leave_ticks_and_estimates_unchanged` (`src/main.rs`), `cancellation_stops_a_silent_agent_without_creating_a_resumable_chat` (`src/agent.rs`).

**Structure:**
```
src/<module>.rs        # production code, then #[cfg(test)] mod tests { use super::*; ... }
tests/opencode_continuation.rs   # binary-level continuation test
tests/pi-reader.mjs              # node:test for src/pi-reader.mjs
```

## Test Structure

**Suite Organization:**
```rust
// src/main.rs
#[cfg(test)]
mod tests {
    use super::*;
    use rune::{Env, Runtime};

    fn runtime() -> Runtime<State> {
        let env = Env { system_dark: true, scale: 2.0 };
        let mut rt = Runtime::new(fixture_state(), view, (720.0, 718.0), env);
        rt.set_system_reduced_motion(true);
        rt.frame(0.0);
        rt
    }

    fn click(rt: &mut Runtime<State>, key: &str) {
        rt.update(|_, tx| { tx.instant(); tx.scroll_into_view(key); });
        rt.frame(0.0);
        let (x, y) = rt.find(key).unwrap_or_else(|| panic!("missing {key}"));
        rt.pointer_down(x, y);
        rt.pointer_up(x, y);
        rt.frame(0.0);
    }
}
```

**Patterns:**
- Setup: `runtime()` from `fixture_state()`, which is `state_from(catalog::fixture())` with a temp prefs path (`src/main.rs`). Settings tests build `rune::Runtime::new(state(), view, size, Env { … })` from `settings::state()`, which also uses `prefs::path_for_state()` (`src/settings.rs`).
- Input: `click(&mut rt, "key")`, `rt.key(KeyEvent { … })`, `rt.wheel(x, y, dy)`, `rt.set_viewport((w, h), 2.0)`, and `rt.update(|s, tx| …)` for direct state changes or a `poll` call (`src/main.rs`, `src/settings.rs`).
- Queries: `rt.find(key)` (center point or `None`), `rt.presented(key)` (rect for fit checks), `rt.elements()` (text by key), `rt.state` (model) (`src/main.rs`).
- Teardown: tests delete only their own temp files and directories, for example `std::fs::remove_file(path).unwrap()` on the temp prefs path (`src/main.rs`, `src/settings.rs`) and `remove_dir_all(root)` for stub roots (`src/agent.rs`, `src/scan.rs`). They never delete runs, settings or conversations outside the temp dir (`AGENTS.md`).
- Assertions check observable results: tick IDs, `rt.state.confirmed`, element presence, and rects inside the viewport at widths `360.0, 619.0, 620.0, 960.0` (both sides of the 620-point layout switch) (`src/main.rs`).

## Mocking

**Framework:** None. No mocking crate (`Cargo.toml`). Fakes are real executables and real files.

**Patterns:**
```rust
// src/agent.rs — provider stub as a /bin/sh script in a temp root
let root = prefs::path_for_state().with_extension("oversized-protocol");
std::fs::create_dir(&root).unwrap();
let bin = root.join("grok");
std::fs::write(&bin, "#!/bin/sh\n/usr/bin/head -c 2097153 /dev/zero | /usr/bin/tr '\\000' x\nexec sleep 30\n").unwrap();
std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
let mut job = Job::start_in(&root.join("Runs"), bin.to_str().unwrap(), "Grok", "", "Saved selection", Access::default()).unwrap();
```
- Stub file names select the adapter (`grok`, `agy`, `copilot`, `pi`, `opencode`). Stubs answer ACP/RPC messages, report a version, delay private configuration, emit oversized or silent output, ignore stdin EOF, or leave a descendant that holds stdout (`src/agent.rs`, `tests/opencode_continuation.rs`).
- `src/prefs.rs` (`ask_runs_keep_private_snapshots_and_start_the_configured_agent`, `#[cfg(unix)]`) uses a `/bin/sh` stub that prints `$PWD`, `$#`, `GROK_FOLDER_TRUST` and its arguments, to check that a prepared run starts the configured agent in its private run folder. The temp root name contains `'` and `$` to test quoting.
- `tests/pi-reader.mjs` writes a `#!/bin/sh` reader helper and a fake Pi `ExtensionAPI` object (`on`, `registerTool`) and restores `process.env` in `finally`.
- Process-global callbacks are isolated by re-running the test binary as a child with `FOLDERLENS_STATUS_TEST_CHILD=1` and `--exact <name>` (`src/status.rs`).

**What to Mock:**
- Every agent CLI, model and provider login. Never call a real CLI, a paid model or the network in tests (`AGENTS.md`).
- The folder picker: `prefs::choose_folder()` returns `None` under `cfg(test)` (`src/prefs.rs`).
- The settings file: `prefs::path_for_state()` returns a unique temp file `folderlens-prefs-<pid>-<n>.txt` under `cfg(test)` (`src/prefs.rs`). Tests must not touch the real `settings.txt` (`AGENTS.md`).
- Run folders: under `cfg(test)`, `agent::Job::start` uses `$TMPDIR/folderlens-runs-<pid>`, not the real `Runs/`, so UI tests that press Ask or Retry cannot write there (`src/agent.rs`). Agent tests call `Job::start_in` with their own temp root.

**What NOT to Mock:**
- Filesystem symlink and descriptor boundaries, Unix-socket reads and revocation (`src/broker.rs`, `src/agent.rs`).
- Allocated-block, hard-link and cache-header scanning on temp trees (`src/scan.rs`).
- Settings persistence: tests write with `prefs::save_to` and read back (`src/main.rs`, `src/settings.rs`).
- The non-deleting Record choice: assert the catalog is unchanged after `click(&mut rt, "confirm")` (`src/main.rs`).

## Fixtures and Factories

**Test Data:**
```rust
// src/main.rs
#[cfg(test)]
fn fixture_state() -> State {
    state_from(catalog::fixture())
}
```
- `catalog::fixture()` gives 7 leaves (`backups`, `build`, `logs`, `toolchains`, `deps`, `cache-held`, `spotify`) in 6 groups. It has locked rows, per-level hour gates and fixed readings: disk 84.9 GB free of 494.35 GB, CPU 30%, memory 79% (`src/catalog.rs`). Expected ticks: Careful `["logs"]`, Balanced `["logs", "spotify"]`, Aggressive `["logs", "deps", "spotify"]` (`src/main.rs`).
- `ticked_ids(&State)` and `leaf_at(&State, id)` read results (`src/main.rs`).
- `agent::Job::preview(…)` and `agent::Job::failed(…)` give synthetic progress, results and errors without a process (`src/main.rs` scenes, `src/agent.rs`).

**Location:**
- `catalog::fixture()` is production code (not `cfg(test)`) because headless `*-example` scenes also use it (`src/catalog.rs`, `src/main.rs`, `src/notch.rs`). `fixture_state()` and `ticked_ids()` are `cfg(test)` items outside `mod tests`, so `src/notch.rs` tests can call `crate::fixture_state()`.
- Keep new UI tests on `fixture_state()` (`AGENTS.md`).

## Coverage

**Requirements:** No percentage gate and no coverage tool. CI on `macos-26` requires, in order: `cargo fmt --check`, `cargo clippy --locked --all-targets`, `cargo test --locked`, `node --test tests/pi-reader.mjs`, then `sh scripts/release.sh` and an artifact upload (`.github/workflows/ci.yml`). The tag workflow runs the same gates, then signs and runs `FolderLensUpdater --verify-release target/release/assets native/update-public-key.txt` (`.github/workflows/release.yml`).

**View Coverage:**
```bash
# None configured. Read the behavior tests per module.
```

## Test Types

**Unit Tests:**
- Level gates, locks, held and unlocked rows, group ticks, the cleanup line, CPU deltas and project-day rules (`src/main.rs`, `src/catalog.rs`).
- Prefs round trip, the legacy Shelf settings import, protected-root boundaries, agent detection (`src/prefs.rs`).
- Theme contrast for every theme (`src/theme.rs`); installer start limited to app bundles (`src/update.rs`); update defaults and opt-ins (`src/settings_updates.rs`); notch and panel placement on displays (`src/status.rs`).

**Integration Tests:**
- Provider protocol, session resume, cancellation and reaping with `/bin/sh` stubs (`src/agent.rs`); real socket reads through the broker (`src/broker.rs`).
- `tests/opencode_continuation.rs` runs `folderlens --folderlens-opencode-resume <stub> --standalone --session exact-chat`. It accepts OpenCode `2.0.23`, rejects `2.2.0` with "requires 2.0.23", and must fail with "timed out" in under 6 s when a descendant holds stdout.
- `tests/pi-reader.mjs` checks the `folderlens_read_selected` tool, the exact-model gate, blocked `read`/`bash`/`shelf_read_selected` tools and blocked user bash.
- Live Mac: `the_live_index_feeds_this_mac` runs `catalog::load_live()` and `ps` (`src/main.rs`); `this_mac_reports_cpu_ticks` reads Mach `host_statistics` (`src/catalog.rs`). These read the real disk and need macOS.

**E2E Tests:**
- Not automated. Headless scene renders (below) and runtime input tests cover UI contracts only. Real model turns, tool routing during a turn, native chat continuation, tray routing, install/restart and Gatekeeper remain unverified (`AGENTS.md`, `docs/guide.md`).

**Headless scene renders:**
```bash
cargo run -- --theme dark --scene ledger-example --snap shots/review-ledger.png
cargo run -- --window settings --scene displays-example --theme dark --snap displays.png
cargo run -- --window notch --scene expanded-example --size 480x230 --theme dark --snap notch.png
cargo run -- --theme dark --scene ledger-example --crop candidate-estimate --snap estimate.png
cargo run -- --list-scenes
```
The first command is from `AGENTS.md` and `docs/guide.md`; the notch form is from `docs/guide.md`. The settings and `--crop` lines are examples of the same flags.
- Rune parses `--scene`, `--snap <png>`, `--size WxH`, `--scale`, `--theme`, `--window <name>`, `--list-scenes`, and `--crop` (`x,y,w,h` in points, or an element key) (Rune `crates/rune/src/tooling/args.rs` at the pinned rev).
- Scenes are listed in `AppSpec.scenes` (`src/main.rs`), `src/settings.rs` and `src/notch.rs`. `*-example` and `agent-error-details` use `catalog::fixture()` (`live_state` in `src/main.rs`). Other panel scenes (`careful`, `balanced`, `menu`, …) scan the real Mac.
- A panel scene that opens the action list (`menu`, `menu-example`) must be matched in `launch_size()`, or the headless window clips the list (`src/main.rs`, `AGENTS.md`).
- After panel changes, regenerate `shots/review-*.png` and `shots/exclusion-rules.png` with `--theme dark`. Keep the original screenshots linked in `README.md` as provenance (`AGENTS.md`).

## Common Patterns

**Async Testing:**
```rust
// src/agent.rs
fn wait(job: &mut Job) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while job.result.is_none() && Instant::now() < deadline {
        job.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(job.result.is_some(), "worker did not finish");
}
```
- Poll with a finite `Instant` deadline and short sleeps. Cancel the job before removing its fixture root (`src/agent.rs`). Measure that a hang is bounded with `start.elapsed() < Duration::from_secs(6)` (`tests/opencode_continuation.rs`).

**Error Testing:**
```rust
// src/agent.rs — failure leaves no saved success
let dir = job.script.parent().unwrap();
assert!(!dir.join("analysis.md").exists());
assert!(!dir.join("session.txt").exists());
assert!(result.is_some_and(|r| r.is_err_and(|e| e.contains("2 MiB"))));
```
- Check that failures save no `analysis.md`/`session.txt`, give no wider access, leak no paths or secrets in `agent.log`, and leave ticks, estimates and the catalog unchanged (`src/agent.rs`, `src/main.rs`).

---

*Testing analysis: 2026-11-14*

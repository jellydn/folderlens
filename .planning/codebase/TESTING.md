# Testing Patterns

**Analysis Date:** 2026-11-09

## Test Framework

**Runner:** Cargo's built-in Rust harness (`Cargo.toml`); Rune headless `Runtime`; Swift helper self-tests (`native/Updater.swift`). Full checks require macOS.

**Assertion Library:** Rust `assert!`, `assert_eq!` and controlled-fixture `unwrap`; Swift `precondition` and a local rejection helper.

**Run Commands:**
```sh
cargo test --locked
cargo test --locked oversized_protocol_lines
cargo fmt --check
cargo clippy --locked --all-targets
sh scripts/release.sh
```
Bundling runs `ShelfUpdater --self-test`. Signed assets are checked with `ShelfUpdater --verify-release <assets> native/update-public-key.txt`; this does not install or launch the downloaded app (`scripts/bundle.sh`, `.github/workflows/release.yml`).

## Test File Organization

**Location:** Co-located `#[cfg(test)] mod tests` in Rust owners. `tests/opencode_continuation.rs` invokes the real binary's early continuation mode. `native/Updater.swift::selfTest()` runs inside the updater executable.

**Naming:** Behavioral snake_case, such as `recording_is_keyboard_reachable_but_empty_selection_is_not` (`src/main.rs`) and `reads_require_original_and_current_selection_and_exclude_blocked_paths` (`src/broker.rs`).

**Structure:** Module tests share private fixtures; executable tests exercise startup without Rune. Process-global status callback state is tested in a child test process (`src/status.rs`).

## Test Structure

**Suite Organization:** Tests live with the module that owns the contract. UI tests instantiate a real Rune runtime with fixed data, pointer/key/wheel input and layout queries (`src/main.rs`, `src/settings.rs`).

**Patterns:** `fixture_state()` → `Runtime::new` → frame → input → frame → assert observable state/geometry. Settings persistence reads the temp file back; broker tests use real files/sockets. Tests remove only their own fixture directories.

## Mocking

**Framework:** No mocking crate. Tests generate executable `/bin/sh` stubs for provider binaries (`src/agent.rs`, `tests/opencode_continuation.rs`).

**Patterns:** Stubs reply to ACP/RPC messages, expose models/modes, delay private configuration, save exact session fixtures, emit failures or hold pipes open. Oversized unterminated output must fail without waiting for a newline; blank diagnostic lines count toward the log cap (`src/agent.rs`).

**What to Mock:** Real inference and provider authentication; unrelated user data. Never call a paid model or change shared provider configuration during tests (`AGENTS.md`).

**What NOT to Mock:** Filesystem descriptor/symlink boundaries, Unix socket revocation, exact saved launchers, settings persistence, cryptographic rejection or non-deleting selection behavior.

## Fixtures and Factories

**Test Data:** `catalog::fixture()` supplies seven leaves/six groups with asymmetric gates and sizes (`src/catalog.rs`). `fixture_state()` starts at no selection (`src/main.rs`). `Job::preview` supplies synthetic progress/results without a process (`src/agent.rs`).

**Location:** Co-located factories; `prefs::path_for_state()` returns unique temporary settings paths under `cfg(test)`. Scan tests create temporary home/project/app trees. Swift tests generate disposable keys, archive bytes, cask receipts and rollback files.

Synthetic panel `*-example` and `agent-error-details` scenes avoid live scanning. Other panel snapshots load live catalog state; Settings scenes use example preferences without persisting scene choices (`src/main.rs`, `src/settings.rs`).

## Coverage

**Requirements:** No percentage gate or coverage-report tool is configured. CI requires formatting, Clippy, full tests and packaged app output (`.github/workflows/ci.yml`). Keep current execution evidence separate from this source map; historical test counts are not a current result.

**View Coverage:** No repository coverage command. Inspect the behavioral suites directly. Do not report headless renders as native interaction proof.

## Test Types

**Unit Tests:** Preset gates, protected/held/unlocked eligibility, group selection, CPU deltas, allocated blocks/hard links/cache format, per-agent models and UI layout/input (`src/catalog.rs`, `src/scan.rs`, `src/main.rs`, `src/prefs.rs`, `src/settings.rs`).

**Integration Tests:** Provider protocol/session/cancellation stubs and real broker socket reads (`src/agent.rs`, `src/broker.rs`). The production continuation test rejects a changed OpenCode version and bounds descendant-held stdout (`tests/opencode_continuation.rs`). `the_live_index_feeds_this_mac` checks a real scan/disk/gauges; `this_mac_reports_cpu_ticks` also reads live Mach ticks (`src/main.rs`, `src/catalog.rs`).

**Native helper tests:** Metadata signatures/expiry/channel/tag rejection, corrupted archive rejection, quarantine attributes, cask/unknown ownership and atomic-exchange rollback fixtures (`native/Updater.swift`). These do not replace an actual install/restart test.

**E2E Tests:** Headless rendered scenes and input tests cover UI contracts. Real native model/tool turns, Terminal continuation, complete tray routing on supported macOS versions, install/restart and Gatekeeper approval remain outside verified automated coverage (`AGENTS.md`, `RELEASE_NOTES.md`).

## Common Patterns

**Async Testing:** Poll jobs with finite `Instant` deadlines and short sleeps; cancel before fixture removal. Read channels with bounded timeouts. Tests cover silent agents, blocked prompt writes, inherited version pipes and partial broker packets (`src/agent.rs`, `src/broker.rs`).

**Error Testing:** Assert no report/session/usable launcher on failure, no authority widening, no leaked diagnostic paths/secrets, and unchanged ticks/catalog. Signed release checks use a committed public key; self-tests generate disposable private keys (`src/agent.rs`, `native/Updater.swift`).

**UI Regression Testing:** Check both sides of the 620-point layout switch, minimum height, compact/wide keyboard recording, fixed Apply model, action-list scrolling and wrapped reasons. Outside clicks/transient session ends preserve the panel; explicit controls still close it (`src/main.rs`, `src/settings.rs`, `src/status.rs`).

---

*Testing analysis: 2026-11-09*

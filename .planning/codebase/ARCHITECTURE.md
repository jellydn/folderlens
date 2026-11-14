# Architecture

**Analysis Date:** 2026-11-14

This is the start of the codebase map for FolderLens 0.3.7 (`Cargo.toml`). Other map files: [STRUCTURE.md](STRUCTURE.md), [STACK.md](STACK.md), [INTEGRATIONS.md](INTEGRATIONS.md), [CONVENTIONS.md](CONVENTIONS.md), [TESTING.md](TESTING.md), [CONCERNS.md](CONCERNS.md). Binding rules are in `AGENTS.md`. Accepted boundary decisions:

- [ADR 1: scoped agent sessions](../../docs/adr/0001-scoped-agent-sessions.md): run folders, broker reads, provider adapters.
- [ADR 2: publisher-verified updates](../../docs/adr/0002-publisher-verified-updates.md): native helper, signatures, manual install.
- [ADR 3: review works without AI](../../docs/adr/0003-review-works-without-ai.md): scans, ticks, Record choice and the local summary need no agent.

## Pattern Overview

**Overall:** One Rust binary crate (`Cargo.toml`, package `folderlens`) that runs a state-driven Rune GPU UI as a macOS accessory app. It has three Rune windows (review panel, Settings, notch), background worker threads, an isolated agent job with an app-owned read broker, and a separate Swift update helper (`native/Updater.swift`).

**Key Characteristics:**
- Review only. Presets, manual overrides and protections derive the ticks (`src/main.rs`: `is_ticked`, `leaf_view`). Record choice sets the in-memory flag `State::confirmed` and never deletes or changes the catalog (`src/main.rs`: `record_choice`). No delete or cleanup path exists in `src/`.
- Slow work stays off the UI thread. A scan worker and a gauge worker send results through Rune's UI queue with `AppHandle::update` / `update_if` (`src/main.rs`: `start_workers`). The update helper runs on its own thread (`src/update.rs`). `poll` only reads preferences, dispatches due scan requests and polls the agent job (`src/main.rs`: `poll`).
- Saved context and live read authority are separate. Each Ask writes a new private run folder (`src/prefs.rs`: `prepare_run`). Live reads go through one broker whose scope is the frozen original selection ∩ the current eligible selection (`src/broker.rs`: `Reader::allows`). See ADR 1.
- The panel stays useful without AI. Scans, intervals, presets, ticks, estimates and Record choice need no agent, and `local_summary` shows a non-AI reading when no agent can run or the last review failed (`src/main.rs`; ADR 3).

## Layers

**Review state and panel UI:**
- Purpose: Own the review state, derive ticks and estimates, build the panel, dispatch scans, poll the agent job.
- Location: `src/main.rs`
- Contains: `State`, `state_from`, `live_state`, `start_workers`, `request_scan` / `schedule_scan` / `scan_if_due` / `finish_scan`, `poll`, `view`, `overview` / `usage_readings`, `scan_controls`, `status_line`, `agent_help`, `local_summary`, `agent_review`, `record_choice`, shared `button` / `ButtonStyle` / `rule`, `launch_size`, `main` with the panel scene list, and 26 tests.
- Depends on: `src/catalog.rs`, `src/prefs.rs`, `src/palette.rs`, `src/agent.rs`, `src/notch.rs`, `src/settings.rs`, `src/status.rs`, `src/theme.rs`, `src/update.rs`.
- Used by: Rune (`AppSpec` callbacks), `src/palette.rs` and `src/notch.rs` (they call `is_ticked`, `leaf_view`, `toggle_scans`, `request_scan`, `schedule_scan`).

**Actions list (palette):**
- Purpose: Keyboard-driven action list for one target row or group, plus Ask Agent start and the access snapshot.
- Location: `src/palette.rs`
- Contains: `Target`, `open` / `close` / `fit` / `height` / `open_height`, `on_key` (⌘J, ⌘D, ⌘U, ⌘R, ⌘P, ⌘,, ⌘Q, Escape, arrows, filter text), `run` (Finder, Keep, Unlock, Select, Analyse, Ask, Pause, Settings, tick/clear), `ask_prompt`, `ask_access`, `ask`.
- Depends on: `State` and helpers in `src/main.rs`, `src/catalog.rs`, `src/prefs.rs`, `src/settings.rs`, `src/agent.rs`.
- Used by: `src/main.rs` (`view`, `on_key`, Ask and Retry buttons).

**Settings window:**
- Purpose: Separate Rune window that edits `Prefs` and saves each change to `settings.txt`.
- Location: `src/settings.rs`; private Updates page in `src/settings_updates.rs` (`#[path = "settings_updates.rs"] mod updates`).
- Contains: `Page` (General, Agent, Protected shown as "Keep safe", Access, Updates), `Settings` state, `spec` with Settings scenes, `open` / `hide` / `install` / `update_status`, display choice, agent list and model Apply (`save_model` calls `agent::validate_model`), protected folders and keep-hours stepper, permissions.
- Depends on: `src/prefs.rs`, `src/status.rs` (displays, `conceal`), `src/notch.rs` (`hide` / `show`), `src/agent.rs` (`validate_model`, `model_note`), `src/update.rs` (`check`), `src/theme.rs`.
- Used by: `src/main.rs` (window setup, Settings buttons), `src/palette.rs`, `src/notch.rs`, `src/update.rs` (`update_status`).

**Notch window:**
- Purpose: Default compact (300×32) / expanded (480×230) control strip with candidate total, disk, CPU, memory, interval and Scan now controls. It owns no scan or agent job.
- Location: `src/notch.rs`
- Contains: `Readings::from_review`, `Notch`, `sync` (review → notch through the notch window's UI queue), `send` (notch action → review state through the review window's UI queue), `show` / `hide` / `restore_work_area` / `fit_work_area`, `Visibility` (Visible, Hidden, SpaceHidden), notch scenes.
- Depends on: `State` helpers in `src/main.rs`, `src/catalog.rs`, `src/status.rs`, `src/settings.rs`.
- Used by: `src/main.rs` (`poll` calls `notch::sync`; `main` installs it), `src/status.rs`, `src/settings.rs`.

**Native menu bar and placement:**
- Purpose: AppKit status item (Rune has none), panel show/hide, notch frame placement on the chosen display.
- Location: `src/status.rs` (`macos` submodule uses raw Objective-C messages through `block2` / `libc`).
- Contains: `arm`, `on_poll` (creates the item on the first live frame), `reveal`, `conceal`, `is_visible`, `displays`, `set_notch_placement`, `position_notch`, pure `notch_origin` and `popover_origin` with tests.
- Depends on: `src/prefs.rs` (`NotchPosition`), `src/notch.rs`, `src/settings.rs`.
- Used by: `src/main.rs`, `src/notch.rs`, `src/settings.rs`, `src/palette.rs`.

**Preferences and run files:**
- Purpose: One `key=value` settings file shared by all windows, protected roots, agent detection, Ask prompt text, and private run folders and launchers.
- Location: `src/prefs.rs`
- Contains: `ALWAYS`, `MAX_KEEP_HOURS`, `NotchPosition`, `Prefs`, `user_path`, `path_for_state` (temp file under `cfg(test)`), `load_from` (one-time legacy Shelf import), `save_to`, `TOOLS` / `detect_agents` / `agents`, `prepare_ask`, `prepare_run`, `resume_run`, `choose_folder`, `open_full_disk_access`.
- Depends on: `src/theme.rs` (`Theme`), `serde_json`.
- Used by: every UI module and `src/agent.rs`; `native/Updater.swift` reads the same file for update preferences.

**Catalog and measurement:**
- Purpose: Find candidate folders, measure allocated size, apply tier gates and in-use locks, read disk, CPU and memory.
- Location: `src/catalog.rs`, which privately includes `src/scan.rs` with `#[path = "scan.rs"] mod scan`.
- Contains: `Level`, `Leaf`, `Group`, `Catalog`, `can_tick`, `auto_tick`, `format_bytes`, `load_live` (uses size cache), `refresh_live` (remeasures), `pending`, `fixture`, `assemble`, `cpu_ticks` / `cpu_delta` / `memory_now`, `measure`. `src/scan.rs` holds `RULES`, `scan_root`, `measure_all` (up to 8 threads), `measure` (512-byte blocks, one count per inode, no symlink follow), and the `sizes.tsv` v2 cache.
- Depends on: `std`, and the commands `ps`, `diskutil info /`, `sysctl -n hw.memsize`, `vm_stat`, plus Mach `host_statistics` for CPU (`src/catalog.rs`).
- Used by: `src/main.rs`, `src/palette.rs`, `src/notch.rs`.

**Agent execution and authority:**
- Purpose: Run one bounded, read-only agent review per request and own its lifecycle (ADR 1).
- Location: `src/agent.rs`, which privately includes `src/broker.rs` and `src/provider.rs` with `#[path]`. `src/pi-reader.mjs` is embedded with `include_str!` in `src/provider.rs`.
- Contains: `Access`, `Job` (`start`, `poll`, `status`, `cancel`, `set_access`, `open_chat`, `preview`, `failed`), `Session` (ACP and Pi RPC), `analyze`, `incoming`, `redact_diagnostics`, `reader_helper`. `src/provider.rs`: `Provider`, `Launch::prepare`, `verify_opencode`, `resume_helper`, `wait_for_private_opencode_mode`. `src/broker.rs`: `Reader`, `Broker`, `helper`, `mcp_response`.
- Depends on: `src/prefs.rs`, `serde_json`, `libc`.
- Used by: `src/palette.rs` (`ask`), `src/main.rs` (`poll`, panel), `src/settings.rs` (`validate_model`, `model_note`).

**Theme:**
- Purpose: Seven fixed palettes and a scoped paint adapter.
- Location: `src/theme.rs`
- Contains: token constants, `Theme` (Default, Kanagawa, Kanagawa Lotus, Dracula, Alucard, Tokyo Night, Tokyo Night Day), thread-local `scope` / `Scope` guard, `paint`.
- Used by: each root `view` (`src/main.rs`, `src/settings.rs`, `src/notch.rs`) opens a scope for its saved theme, so windows and parallel headless tests do not share one global theme.

**Updates:**
- Purpose: Launch the bundled Swift helper; the helper does all network, signature and download work (ADR 2).
- Location: `src/update.rs` (launcher), `native/Updater.swift` (helper), `native/update-public-key.txt` (public Ed25519 pin).
- Depends on: an app bundle layout `FolderLens.app/Contents/MacOS/FolderLensUpdater` (`src/update.rs`: `helper`).
- Used by: `src/main.rs` (`update::start` in `on_ready`), `src/settings_updates.rs` (`update::check`).

## Data Flow

**Scans and gauges:**
1. Live startup builds `State` from `catalog::pending()` so the panel opens at once (`src/main.rs`: `live_state`). Headless runs load `catalog::fixture()` for `*-example` and `agent-error-details` scenes, else `catalog::load_live()`.
2. In `on_ready`, `start_workers` stores an `mpsc::Sender<()>` in `State::scan_requests` and sends the first request (`src/main.rs`).
3. The scan thread takes one request at a time. The first uses `catalog::load_live()` (cached sizes in `~/Library/Application Support/FolderLens/sizes.tsv`); later requests use `catalog::refresh_live()` and remeasure every path (`src/main.rs`, `src/scan.rs`: `scan_root`).
4. `catalog::load_catalog` runs `scan::scan_root`, reads `ps -ax` command lists, then `assemble` drops hinted children under a kept parent, locks rows used by a running command or a listed tool, and builds groups (`src/catalog.rs`).
5. The worker sends the catalog through `AppHandle::update`; `finish_scan` keeps manual ticks by path and open groups by ID, remaps the action target, clears `confirmed` and the note, and pushes a new `Access` to a running job (`src/main.rs`). A `missing` catalog keeps the previous results and sets a note.
6. Scan now (`request_scan`), Pause / Resume (`toggle_scans`) and the 30 min / 1 hour presets come from the panel, the Actions list (⌘P) or the notch (`send`). Intervals are session-only and start paused (`State::scans_running = false`). `poll` calls `scan_if_due`, which only sends a request (`src/main.rs`, `src/notch.rs`, `src/palette.rs`).
7. The gauge thread samples CPU ticks and memory once a second and calls `update_if`, which redraws only when a value changed (`src/main.rs`: `start_workers`; `src/catalog.rs`: `cpu_delta`, `memory_now`).
8. Actions → Measure folder again (`analyse`) sends `ScanRequest::Measure` with the target paths to the same scan worker. It is refused while a scan runs, and a due interval scan waits for it. The worker calls `catalog::measure`, and `finish_measure` applies sizes by path and clears Record choice (`src/palette.rs`: `analyse`, `src/main.rs`).

**Selection, ticks and estimates:**
1. `leaf_view` adds app-level protection to each scanned leaf: `prefs::ALWAYS` roots (ignore Unlock), session unlocks in `State::unlocked`, kept roots in `Prefs::kept`, and the "changed within N h" hold from `Prefs::keep_hours` (`src/main.rs`).
2. `catalog::can_tick` requires an unlocked leaf with at least one tier gate; `catalog::auto_tick` compares `idle_h` with the gates that the level enables (Careful → careful; Balanced → careful, balanced; Aggressive → all three) (`src/catalog.rs`).
3. `is_ticked` uses a manual override in `State::ticks` when present, else `auto_tick`, and always returns false for an ineligible leaf (`src/main.rs`).
4. `apply_level` / `cycle_level` reset all overrides; `toggle_leaf`, `toggle_group`, palette `tick` / `clear`, Keep and Unlock change ticks or protections and clear `confirmed` (`src/main.rs`, `src/palette.rs`).
5. Estimates are sums of `Leaf::bytes` (allocated blocks) for ticked leaves: `ticked_bytes`, `cleanup_line`, the footer "N selected · X estimated", and the notch candidate total (eligible bytes) (`src/main.rs`, `src/notch.rs`). The UI wording says estimate, not reclaimed space.

**Record choice:**
1. The footer button `confirm` is disabled when nothing is ticked (`src/main.rs`: `record_choice`).
2. A press sets `State::confirmed` to true only if at least one leaf is ticked. Nothing is written to disk and the catalog does not change; the footer says "Nothing deleted · in memory only".
3. Any change to ticks, level, protections, measurement, a rescan, or non-appearance preferences clears it (`src/main.rs`: `poll`, `finish_scan`, `apply_level`, `toggle_leaf`; `src/palette.rs`). Theme and notch-display changes keep it. Tests `a_locked_row_stays_unticked_and_confirm_deletes_nothing` and `quick_review_keeps_recording_visible_and_non_destructive` assert the catalog is unchanged (`src/main.rs`).

**Ask Agent run (ADR 1):**
1. The user presses Ask Agent, Retry, or the Actions item Ask (⌘J). `palette::ask` refuses a second running job (`src/palette.rs`).
2. `ask_prompt` builds the context from the current ticks (not the hovered row). `prefs::prepare_ask` returns an error and opens Settings when no agent is chosen or `allow_start` is off; paths are included only when `send_paths` is on (`src/palette.rs`, `src/prefs.rs`).
3. `ask_access` builds `Access`: empty when start or sharing is off; else selected = ticked leaf paths, blocked = `ALWAYS` roots, kept roots and every unticked leaf (`src/palette.rs`).
4. `Job::start` validates the model, then `prefs::prepare_run` creates a new `0700` folder `Runs/chat-<nanos>-<pid>-<seq>` with exclusive `create_dir`, writes `context.md` and a failing `Ask.command` placeholder (`src/agent.rs`, `src/prefs.rs`).
5. `broker::Reader::new` freezes the canonical original scope; `Broker::start` binds `/tmp/folderlens-read-<pid>-<nanos>-<seq>/read.sock` in a `0700` folder, because macOS limits socket paths to 104 bytes (`src/broker.rs`).
6. A worker thread runs `analyze`: `Launch::prepare` builds provider arguments (`src/provider.rs`), the child runs in the run folder with `NO_COLOR=1`, stderr is redacted into `agent.log` (max 16 KiB), and stdout lines are bounded to 2 MiB (`src/agent.rs`).
7. Provider routes: Grok and Antigravity use ACP `fs/read_text_file` and `session/request_permission`, allowed only for read/search locations that the reader permits. Copilot and OpenCode get only the MCP tool `read_selected` through `--folderlens-reader <socket>`; Pi gets only the extension tool `folderlens_read_selected` from `src/pi-reader.mjs`. Writes, terminal and other capabilities get a JSON-RPC error (`src/agent.rs`: `incoming`; `src/broker.rs`; `src/provider.rs`).
8. Each live read: canonicalize → `Reader::allows` (context file, or original selected ∩ not original blocked ∩ current `Access`) → `O_NOFOLLOW | O_NONBLOCK` open → descriptor path check → at most 512 KiB UTF-8 text or 1,024 directory entries → recheck → at most 2,000 lines (`src/broker.rs`: `Reader::read`).
9. `poll` and `view` push the current `ask_access` into the job; `poll` cancels the job when `allow_start`, the agent binary or the model changes (`src/main.rs`).
10. A nonempty answer wins the atomic `RUNNING → FINALIZING` transition, then `analysis.md`, `session.txt` and the exact resume `Ask.command` are written. An empty answer, cancel or error saves no success files (`src/agent.rs`: `analyze`; `src/prefs.rs`: `resume_run`). Pi must reach `agent_settled`.
11. On error, the broker stops and the panel shows the first error line, Details, Agent settings and Retry, plus `local_summary` (`src/main.rs`: `agent_review`, `local_summary`). On success, "Continue saved chat in Terminal" opens `Ask.command` only for the same agent and model (`src/agent.rs`: `open_chat`). Grok/Antigravity Terminal continuation does not use the broker.
12. Dropping the `Job` (replacement or `shutdown`) cancels, stops the broker and waits up to 2 s for the provider to be reaped (`src/agent.rs`: `Drop for Job`; `src/main.rs`: `shutdown`). Restarting the app does not restore old grants.

**Local summary (no AI, ADR 3):**
1. `local_summary` renders when `agent_bin` is empty or `allow_start` is off, or when the last job ended in `Err` (`src/main.rs`).
2. With a nonempty selection it shows the item count and total, the top three groups and the three largest folders by estimated size, and "review only, not reclaimed. FolderLens deletes nothing." It reads `is_ticked` and catalog sizes only; it starts no process and writes no file.

**Updates (ADR 2):**
1. On a live (not headless) start, `update::start` runs `FolderLensUpdater --automatic <bundle> <settings.txt> <pid>` every hour on a thread (`src/update.rs`).
2. The helper reads `update_checks`, `update_downloads` and `update_prereleases` from `settings.txt`, and skips unless 24 h passed since `~/Library/Application Support/FolderLens/Updates/last-check` (`native/Updater.swift`: `Updater.check`).
3. It pages GitHub releases for `jellydn/folderlens`, downloads `update.json` and `update.json.sig`, and verifies them with the pinned Ed25519 key (tag, channel, version, expiry) (`native/Updater.swift`: `verifiedManifest`).
4. Homebrew installs get `brew upgrade --cask jellydn/tap/folderlens` guidance and stop. Otherwise the ZIP downloads (automatic only if `update_downloads` is on; else after a Download prompt), with a size bound, SHA/size check and quarantine.
5. Only a manual check (Settings → Updates → Check for Updates…, `src/settings_updates.rs` → `update::check`) rechecks the ZIP and offers Reveal in Finder. Replacement is manual. `Updater.install` and `requireDirectInstallation` remain but are reached only from `selfTest` (`native/Updater.swift`).

**State Management:**
- `State` in `src/main.rs` holds the catalog, level, per-leaf overrides (`ticks: Vec<Option<bool>>`), open groups, `confirmed`, a `Prefs` copy, palette state, session unlocks, the note, the current `agent::Job`, scan request sender and interval flags, and the last window height.
- `Prefs` persist in `settings.txt` (`src/prefs.rs`). Settings saves at once; the panel `poll` reloads the file each tick and compares (`src/main.rs`).
- The notch keeps a copy (`Readings`) that `notch::sync` refreshes; notch actions mutate `State` through `send` (`src/notch.rs`).
- Live access is `Arc<Mutex<Access>>` shared by `Job` and `Reader`; cancellation is an `AtomicU8` with RUNNING, CANCELLED and FINALIZING (`src/agent.rs`).
- Window visibility uses atomics and a `Visibility` mutex: the review, Settings and notch are not shown together (`src/status.rs`: `macos::install`; `src/settings.rs`: `open`; `src/notch.rs`).

## Key Abstractions

**Catalog / Leaf / Group / Level:**
- Purpose: Measured candidates with tier gates and locks; not deletion instructions.
- Examples: `src/catalog.rs`, `src/scan.rs` (`Rule`, `Kind`, `RULES`)
- Pattern: Groups index contiguous leaf ranges (`first`, `count`); views derive per-render values from `State`.

**Prefs / NotchPosition / ALWAYS:**
- Purpose: Persistent choices and fixed protected roots.
- Examples: `src/prefs.rs`
- Pattern: Plain `key=value` text, temp-file rename on save, per-agent model map in `agent_models` JSON.

**Job / Session:**
- Purpose: App-owned lifecycle and one protocol conversation (ACP JSON-RPC or Pi RPC).
- Examples: `src/agent.rs`
- Pattern: Worker thread + `mpsc` result channel polled by the UI; `Drop` kills and reaps the child.

**Provider / Launch:**
- Purpose: Exact per-provider flags, model rules, private configuration and resume command.
- Examples: `src/provider.rs`
- Pattern: Closed enum (Grok, Antigravity, Copilot, Pi, OpenCode) chosen by binary file name; any other binary fails with a visible error.

**Access / Reader / Broker:**
- Purpose: Frozen original scope, current scope and bounded local read transport.
- Examples: `src/agent.rs` (`Access`), `src/broker.rs`
- Pattern: Unix socket server thread; headless helper modes forward one request each.

**Target:**
- Purpose: The row or group that Actions applies to.
- Examples: `src/palette.rs`
- Pattern: Index enum remapped by path or group ID after each rescan (`src/main.rs`: `finish_scan`).

**Theme / Scope:**
- Purpose: Scoped palette selection per view build.
- Examples: `src/theme.rs`
- Pattern: Thread-local RAII guard.

**Manifest / Updater:**
- Purpose: Signed release metadata and update policy.
- Examples: `native/Updater.swift`
- Pattern: Separate executable; fails closed on missing or invalid proof.

## Entry Points

**App binary:**
- Location: `src/main.rs` (`main`)
- Triggers: `cargo run`, `FolderLens.app/Contents/MacOS/FolderLens`.
- Responsibilities: First calls `agent::reader_helper()` for helper modes. Then builds the panel `AppSpec` (scenes, `poll`, `on_key`, `live_state`, `view`) as an accessory app with `on_quit(shutdown)`, adds the hidden `settings` and `notch` windows, installs their handles, arms the status item, and in `on_ready` starts the workers and the update loop.

**Headless helper modes (no UI, no scan, no model request):**
- Location: `src/broker.rs` (`helper`: `--folderlens-reader <socket>` MCP stdio server, `--folderlens-read <socket>` single request for Pi), `src/provider.rs` (`resume_helper`: `--folderlens-opencode-resume <bin> …`, rechecks OpenCode 2.0.23 before `exec`).
- Triggers: Provider CLIs and saved OpenCode `Ask.command` launchers.

**Headless shots:**
- Location: Rune scene support with scene lists in `src/main.rs` (panel), `src/settings.rs` (Settings), `src/notch.rs` (notch); `launch_size` in `src/main.rs`.
- Triggers: `cargo run -- --theme dark --scene <name> --snap shots/<file>.png` (add `--window settings`; `--list-scenes`), per `AGENTS.md`.

**Saved chat launcher:**
- Location: `Runs/chat-…/Ask.command`, written by `src/prefs.rs` (`prepare_run`, `resume_run`).
- Triggers: "Continue saved chat in Terminal" (`src/agent.rs`: `open_chat`) or the user in Finder.
- Responsibilities: Resume only the exact completed session with shell-quoted arguments and environment.

**Update helper:**
- Location: `native/Updater.swift` (`@main struct FolderLensUpdater`)
- Triggers: `src/update.rs` (`--automatic` / `--manual`), `scripts/bundle.sh` (`--self-test`), release workflow (`--verify-release <dir> <key>`).

## Error Handling

**Strategy:** Fail closed on authority, model, provider version and publisher proof. Keep the user's catalog, ticks and protections unchanged on every failure.

**Patterns:**
- Scan failure returns a `missing` catalog; `finish_scan` keeps the old results and shows a note (`src/main.rs`).
- UI errors are short strings in `State::note` (`src/main.rs`, `src/palette.rs`).
- Agent errors are `Result<String, String>` passed through `redact_diagnostics`, which withholds lines with possible credentials or local paths (`src/agent.rs`). The panel shows the first line, Details, Agent settings and Retry (`src/main.rs`: `agent_review`).
- `Provider::for_bin` and `validate_model` reject unsupported CLIs, flag-like or spaced model IDs, unprefixed Pi/OpenCode models and blank OpenCode models (`src/provider.rs`). Settings shows the error before save (`src/settings.rs`: `save_model`).
- The updater shows fixed notices and hides transport and path details (`native/Updater.swift`: `main`, `UpdateError`). `src/update.rs` reports a helper start failure through `settings::update_status`.

## Cross-Cutting Concerns

**Logging:** No telemetry and no app-wide logger. Each agent run writes redacted, size-bounded stderr to its own `agent.log` (`src/agent.rs`: `analyze`).

**Validation:** Exact model IDs and OpenCode 2.0.23 gate (`src/provider.rs`); OpenCode private mode `folderlens-readonly` confirmed from `config_option_update` (`src/provider.rs`: `wait_for_private_opencode_mode`); descriptor-checked bounded reads (`src/broker.rs`); 256 KiB answer limit and 600 s deadline (`src/agent.rs`); Ed25519 metadata and archive hash/size (`native/Updater.swift`).

**Authentication:** Agent accounts belong to each CLI; FolderLens stores no keys (`src/provider.rs`). Agent start needs `allow_start`; path sharing needs `send_paths` (`src/prefs.rs`, `src/settings.rs`). Updates use a dedicated FolderLens public key (`native/update-public-key.txt`). Provider controls are not an OS sandbox (ADR 1).

**Window policy:** The review panel, Settings and notch are not visible together: showing the panel hides notch and Settings, hiding it shows the notch, and `settings::open` conceals the panel and hides the notch (`src/status.rs`, `src/settings.rs`). Escape closes Actions first, then the panel (`src/palette.rs`: `on_key`). The notch hides itself when the work area is too small and returns only from that state (`src/notch.rs`: `Visibility`).

**Concurrency rule:** System commands (`ps`, `diskutil`, `vm_stat`, scans, agent CLIs, update helper) run on worker threads, never in `poll` (`src/main.rs`, `src/update.rs`, `src/agent.rs`). Measure folder again also runs on the scan worker (`src/main.rs`: `ScanRequest`).

---

*Architecture analysis: 2026-11-14*

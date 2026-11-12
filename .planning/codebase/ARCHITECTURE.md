# Architecture

**Analysis Date:** 2026-11-09

## Pattern Overview

**Overall:** A state-driven native accessory app, with background catalog/gauge workers, isolated agent jobs and a separate native updater.

**Key Characteristics:**
- Quick review derives eligible ticks from presets, manual overrides and exclusions (`src/main.rs`). Record choice stores only an in-memory flag; no cleanup/delete path exists.
- Scan/gauge startup and agent/update processes run outside UI polling. Rune's UI queue receives worker results (`src/main.rs`, `src/update.rs`).
- Saved context and live read authority are separate. Read scope can shrink but cannot exceed its frozen original selection (`src/broker.rs`).

## Layers

**Presentation and state:**
- `src/main.rs` owns `State`, selection/exclusion derivation, shared button styles, Quick review scenes and the app/window configuration.
- `src/notch.rs` owns the default compact/expanded controls. Readings flow from the review state through Rune's UI queue; notch actions return to that same state and scan worker. It owns no scan or AI job.
- `src/palette.rs` owns folder Actions, keyboard navigation, Ask context/access and Finder actions. Ask uses the eligible selection, not the target row.
- `src/settings.rs` owns a separate `Settings` state/window; private `src/settings_updates.rs` owns its Updates page. Shared preferences pass through `src/prefs.rs`.
- `src/theme.rs` owns fixed palette choices and the shared paint adapter. Each root view scopes its saved theme for synchronous view construction, restoring the previous scope afterwards; no process-wide selection leaks between windows or parallel headless tests. Notch readings carry the Review theme through the existing UI queue.

**Catalog and measurement:**
- `src/catalog.rs` owns leaves, groups, selection gates, process-use locks and system readings. It privately includes `src/scan.rs` for rules, traversal, allocated-block measurement and cache storage.
- `src/gpu.rs` reads typed IOKit values. An available device statistic at zero takes precedence over renderer/tiler fallback.

**Agent execution and authority:**
- `src/agent.rs` owns `Job`, ACP/RPC messages, cancellation, reports and exact-session completion.
- Its private `src/provider.rs` owns supported arguments, exact models, version gates and private per-run configuration.
- Its private `src/broker.rs` owns descriptor-checked reads, a Unix socket, JSON forwarding and MCP. `src/pi-reader.mjs` exposes only that reader to Pi.

**Native integration:**
- `src/status.rs` owns retained AppKit status-item callbacks, explicit show/hide state and selected-display notch placement. General offers Top left, Top center, Top right, or Lower center (default). Edge positions use the visible work area; center positions use the full display's center. Top center on a notched display uses `frame.maxY - max(panel_height, safeAreaInsets.top)` for its bottom, matching the native camera-row anchor. All other placements retain work-area/menu-bar clearance and the lower-row inset. Horizontal and lower bounds remain work-area clamped; an oversized frame requests collapse or tray fallback through the notch owner. Physical-camera occlusion is not handled by the content layout. No other app's size is assumed. Preferences migrate the old below/top-row switch, preserving its lower-row inset; Review polling publishes the typed position and saved display UUID for native frame callbacks. Review hides the notch; closing Review restores it. Settings hides both; a tray open hides Settings. The tray remains the fallback.
- `src/update.rs` launches `native/Updater.swift` on a worker. The helper owns HTTP, publisher verification, download policy and the blocked native-install entry points.

## Data Flow

**Catalog and selection:**
1. Live startup creates a pending catalog immediately (`src/main.rs::live_state`).
2. `start_workers` loads the catalog and sends it through `AppHandle::update`; a second worker sends gauge deltas about once a second.
   The scan worker accepts one request at a time. Scan now and the app-owned
   30/60-minute interval controls reuse it; only the startup scan uses cached sizes.
   UI polling dispatches due requests without doing scan work. Pause stops future
   requests, not an active scan. Intervals are session-only, start paused and never
   start an agent. Successful refreshes preserve overrides by path and open groups
   by ID, remap action targets, clear the recorded choice and update reader access.
3. `leaf_view` adds always-protected roots, kept roots, recent-change holds and session unlocks. `is_ticked` applies a manual override or preset only when eligible.
4. Selection, exclusion or measurement changes clear the recorded flag. Recording never changes the catalog.

**Agent review:**
1. An explicit Ask checks permission and a nonempty eligible selection (`src/palette.rs`).
2. `Job::start` saves a new private snapshot with a non-resumable launcher, freezes original access and starts a broker (`src/agent.rs`, `src/prefs.rs`).
3. Provider setup validates model/version/mode before the prompt. ACP callbacks or MCP/Pi tools use the same reader boundary.
4. Polling updates current access; cancellation or configuration changes stop work. A nonempty completed review wins an atomic finalization transition before saving report/session and exact resume arguments.
5. Failure/replacement/app exit closes the broker. Completed saved context remains; restarting Shelf does not revive old grants.

**Live read:** Canonical path → frozen original ∩ current eligible scope → `O_NOFOLLOW`/nonblocking open → descriptor path check → bounded text/listing → scope recheck (`src/broker.rs`). The saved context file is separately readable. Limits are 512 KiB UTF-8, 2,000 lines and 1,024 directory entries.

**Update:** Hourly worker wake → native persistent 24-hour throttle → GitHub candidate → pinned signed metadata → bounded archive download and signed hash/size check. Automatic download stops there. All current installs fail `requireDirectInstallation` before consent and replacement. Retained installer/rollback code is fixture-tested but unreachable for current distributions (`native/Updater.swift`).

**State Management:** `State` keeps ticks, expanded rows, record flag, session unlocks and the current job. `Prefs` persists provider/models, protections, permissions and update settings. The Settings window saves independently; panel polling reloads changes. Broker current access uses `Arc<Mutex<Access>>`; job cancellation uses an atomic lifecycle.

## Key Abstractions

**Catalog / Leaf / Group:** Measured candidates and preset gates, not deletion instructions (`src/catalog.rs`).
**Job / Session:** App-owned lifecycle and one background protocol conversation (`src/agent.rs`).
**Launch / Provider:** Common analysis/resume restrictions with provider-specific setup (`src/provider.rs`).
**Reader / Broker / Access:** Frozen authority, current grants and bounded local transport (`src/broker.rs`, `src/agent.rs`).
**Manifest / Updater:** Publisher proof, channel/version policy and manual-install guidance (`native/Updater.swift`).

## Entry Points

**App:** `src/main.rs::main` handles reader/continuation modes first, then configures `main` and `settings` Rune windows. Synthetic `*-example` scenes avoid disk scans.
**Headless helpers:** `--shelf-reader`, `--shelf-read` (`src/broker.rs`), and `--shelf-opencode-resume` (`src/provider.rs`) use no UI or model request during helper startup.
**Updater:** `ShelfUpdater.main` supports automatic/manual checks, harmless `--self-test` and offline `--verify-release` (`native/Updater.swift`).
**Terminal:** Each `Ask.command` resumes only a completed exact session, with shell-quoted arguments/environment (`src/prefs.rs`).

## Error Handling

**Strategy:** Fail closed on authority, schema, model and publisher proof; preserve the user's catalog and ticks.

**Patterns:** Agent failures show redacted details, Settings and Retry; success-only launchers prevent unrelated chats. The updater reports fixed safe notices rather than transport/path details. Retained atomic bundle exchange supports rollback fixtures, not a verified live installation (`src/agent.rs`, `native/Updater.swift`).

## Cross-Cutting Concerns

**Logging:** Per-run redacted diagnostics (`src/agent.rs`); no remote telemetry.
**Validation:** Exact models, OpenCode 2.0.23, private-mode readiness, descriptor scope and pinned Ed25519 proof.
**Authentication:** CLI-owned accounts; dedicated Shelf publisher key. Prompts/provider controls are not an OS sandbox. Grok/Antigravity Terminal continuation keeps native controls, not the selected-only broker.
**Window policy:** Outside clicks and transient session ends do not dismiss Quick review. Close, Escape and a tray toggle do; Escape closes Actions first (`src/status.rs`, `src/palette.rs`). Native Settings→tray routing remains unverified.

See [ADR 1](../../docs/adr/0001-scoped-agent-sessions.md), [ADR 2](../../docs/adr/0002-publisher-verified-updates.md) and `AGENTS.md` for binding boundaries and commands.

---

*Architecture analysis: 2026-11-09*

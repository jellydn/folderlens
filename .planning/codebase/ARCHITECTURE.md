# Architecture

**Analysis Date:** 2026-11-07

## Pattern Overview
**Overall:** Native state-driven UI with background scan, agent and updater workers.
**Key Characteristics:** Confirm records a choice without deleting; filesystem inspection and application installation have separate approval boundaries.

## Layers
**Presentation:** `src/main.rs`, `src/palette.rs`, `src/settings.rs`, `src/settings_updates.rs`; Rune views and explicit actions.
**Catalog:** `src/catalog.rs`, its private `src/scan.rs`, and `src/gpu.rs`; measured sizes, levels, holds and live gauges.
**Agent execution:** `src/agent.rs` owns jobs/protocol lifecycle; private `src/provider.rs` owns CLI models/configuration; private `src/broker.rs` owns bounded reads.
**Native shell:** `src/status.rs` bridges AppKit status item and panel dismissal; `src/update.rs` starts `native/Updater.swift` outside the UI thread.

## Data Flow
**Scan:** Startup pending catalog → background `load_live()` → Rune UI update → level/protection-derived tick selection.
**Review:** Explicit Ask → saved context/private run → provider handshake/model selection → bounded reader → nonempty completed result/session → exact Terminal launcher.
**Read:** Original grant ∩ current eligible selection → canonical path → opened descriptor validation → bounded text/listing → recheck before returning.
**Update:** Daily/manual trigger → GitHub candidate → signed metadata → signed size/hash archive check → separate install/restart consent → bundle validation → staged replacement/rollback.
**State Management:** `State` owns ticks/holds/job; `Prefs` persists choices; the reader shares current access with a frozen initial boundary.

## Key Abstractions
**Job:** Background analysis lifecycle and saved-chat broker ownership (`src/agent.rs`).
**Launch:** One provider-specific source for analysis/resume arguments and private configuration (`src/provider.rs`).
**Reader/Broker:** App-owned live authority; app exit/cancel/failure closes tools without deleting saved chat (`src/broker.rs`).
**Manifest/Updater:** Verified publisher policy and consent installer (`native/Updater.swift`).

## Entry Points
**App:** `src/main.rs::main`; reader helpers run before Rune or scans.
**Updater:** Swift `ShelfUpdater.main`; bundling runs `--self-test`, release workflow runs `--verify-release`.
**Terminal:** Per-run `Ask.command` resumes an exact completed session, never an unrelated fresh chat.

## Error Handling
**Strategy:** Fail visibly and preserve scan/selection state; fail closed at permission, provider schema and publisher trust boundaries.
**Patterns:** Agent failures stop progress and support redacted details/settings/retry; installer failure keeps a previous app for rollback.

## Cross-Cutting Concerns
**Logging:** Redacted bounded diagnostics, not raw provider transport data.
**Validation:** Provider/model capability checks, descriptor checks and pinned release signatures.
**Authentication:** Provider-managed accounts; independent Shelf publisher key. Neither callbacks nor prompts are an OS sandbox.
See [ADR 1](../../docs/adr/0001-scoped-agent-sessions.md) and [ADR 2](../../docs/adr/0002-publisher-verified-updates.md).

*Architecture analysis: 2026-11-07*

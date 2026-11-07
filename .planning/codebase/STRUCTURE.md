# Codebase Structure

**Analysis Date:** 2026-11-07

## Directory Layout
The source tree has three implementation roots: `src/`, `native/`, and `scripts/`.
Guidance lives in `AGENTS.md`, the codebase map in `.planning/codebase/`, and decisions in `docs/adr/`.

## Directory Purposes
**src:** Rust views, catalog, settings, worker protocols and reader helpers.
**native:** Swift update helper and pinned public key.
**scripts:** Bundle/package/signing scripts; no provider credentials.
**.github/workflows:** CI and release publication.
**shots:** Committed README screenshots.

## Key File Locations
**Entry Points:** `src/main.rs`; `native/Updater.swift`.
**Configuration:** `Cargo.toml`, `rust-toolchain.toml`, `src/prefs.rs`.
**Core Logic:** `src/catalog.rs` privately includes `src/scan.rs`; `src/agent.rs` privately includes broker/provider; `src/settings.rs` privately includes updates page.
**Testing:** Co-located Rust test modules; Swift self-tests in `native/Updater.swift`.

## Naming Conventions
**Files:** Rust snake_case; Swift helper named for its native responsibility.
**Directories:** Small responsibility-based roots, no backend or frontend workspace split.

## Where to Add New Code
**New Feature:** Add behavior to its existing owner; keep tests beside the public contract.
**New Component/Module:** New provider policy belongs in `src/provider.rs`; filesystem authority belongs in `src/broker.rs`, not UI callbacks.
**Utilities:** Prefer the canonical helpers in `src/prefs.rs`; do not add a general utility layer for a one-use wrapper.

## Special Directories
**target:** Generated binaries, bundles, release assets and pinned signing tools; not committed.
**.amp/in:** Local review evidence/scratch; excluded locally, not a release input.
**Application Support/Shelf:** Runtime settings/cache/Runs/Updates; never treat it as disposable test data.

*Structure analysis: 2026-11-07*

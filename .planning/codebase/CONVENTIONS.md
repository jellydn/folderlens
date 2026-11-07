# Coding Conventions

**Analysis Date:** 2026-11-07

## Naming Patterns
**Files:** Rust modules use snake_case (`src/settings_updates.rs`).
**Functions:** Actions name behavior, such as `prepare_run` and `resume_run` in `src/prefs.rs`.
**Variables:** Domain names distinguish saved context from live access.
**Types:** `State`, `Prefs`, `Job`, `Access`, `Reader`, `Launch`, `Manifest`.

## Code Style
**Formatting:** `cargo fmt`; Swift uses swift-format; Pi extension follows the installed SDK's JS format.
**Linting:** `cargo clippy --locked --all-targets`; JS changes require SDK typecheck, lint and Biome per `AGENTS.md`.

## Import Organization
**Order:** Standard library, dependencies, crate modules in Rust (`src/agent.rs`).
**Path Aliases:** Explicit private `#[path]` modules, not re-export barrels.

## Error Handling
**Patterns:** Results for recoverable process/permission errors; agent errors are redacted before UI display. Unsupported providers/models fail without switching tools.

## Logging
**Framework:** No application telemetry framework.
**Patterns:** Keep diagnostics bounded and actionable; never log raw credentials, context paths or signing material.

## Comments
**When to Comment:** Explain constraints and ordering, such as descriptor revalidation or OpenCode private-mode readiness.
**JSDoc/TSDoc:** `src/pi-reader.mjs` uses an imported SDK API type; no standalone JS app.

## Function Design
**Size:** Keep coherent responsibilities; do not move blocking operations into Rune UI polling.
**Parameters:** Pass validated provider/model and access state explicitly; do not interpolate command strings.
**Return Values:** User-visible nonempty reports and preserved selection state are contracts, not mock call order.

## Module Design
**Exports:** Public app entry actions with private provider/broker implementation modules.
**Barrel Files:** None; respect the private module ownership described in `AGENTS.md`.

*Convention analysis: 2026-11-07*

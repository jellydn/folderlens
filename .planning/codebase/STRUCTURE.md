# Codebase Structure

**Analysis Date:** 2026-11-14

One binary crate, `folderlens` 0.3.7 (`Cargo.toml`), plus one Swift helper (`native/Updater.swift`). It is not a workspace or monorepo. Start with [ARCHITECTURE.md](ARCHITECTURE.md) for layers and data flow.

## Directory Layout

```
2026-11-13-jellydn-folderlens/
├── src/                  # Rust app (14 .rs files) and the embedded Pi reader extension
│   ├── main.rs           # State, panel view, poll, workers, scenes, window setup (2,628 lines)
│   ├── catalog.rs        # Catalog/Leaf/Group/Level, gates, locks, gauges; includes scan.rs
│   ├── scan.rs           # Built-in RULES, folder traversal, allocated-size measure, sizes.tsv cache
│   ├── palette.rs        # Actions list, keyboard handling, Ask prompt/access/start
│   ├── prefs.rs          # settings.txt, ALWAYS roots, agent detection, run folders, launchers
│   ├── settings.rs       # Settings window (General, Agent, Keep safe, Access); includes settings_updates.rs
│   ├── settings_updates.rs # Settings → Updates page
│   ├── status.rs         # AppKit status item, show/hide, display list, notch placement
│   ├── notch.rs          # Compact/expanded notch window
│   ├── agent.rs          # Job/Session lifecycle, ACP and Pi RPC; includes broker.rs, provider.rs
│   ├── broker.rs         # Bounded read broker, Unix socket, MCP helper modes
│   ├── provider.rs       # Provider adapters, model rules, OpenCode gate and resume helper
│   ├── update.rs         # Launches the bundled FolderLensUpdater
│   ├── theme.rs          # Seven palettes, scoped paint
│   └── pi-reader.mjs     # Pi extension, embedded with include_str! and copied into each Pi run
├── native/
│   ├── Updater.swift     # FolderLensUpdater: checks, signatures, download, self-test
│   └── update-public-key.txt # Public Ed25519 pin (no private key)
├── scripts/
│   ├── bundle.sh         # Build ad-hoc signed target/release/FolderLens.app (+ updater self-test)
│   ├── release.sh        # DMG, ZIP, SHA256SUMS under target/release/assets; no upload
│   ├── sign-release.py   # Sign update.json and assets with the Actions secret
│   ├── sparkle-tools.sh  # Fetch pinned Sparkle 2.10.1 sign_update into target/
│   └── icons.py          # Regenerate icon and favicon assets
├── tests/
│   ├── opencode_continuation.rs # Integration test of --folderlens-opencode-resume
│   └── pi-reader.mjs     # node:test for src/pi-reader.mjs with a harmless helper
├── shots/                # 28 PNG headless captures, current and historical
├── docs/
│   ├── guide.md          # Full user guide
│   └── adr/              # 0001 scoped agent sessions, 0002 publisher-verified updates, 0003 review without AI
├── assets/               # FolderLens.icns, icons, favicons, webmanifest; legacy Shelf.icns, shelf-icon.png
├── .github/workflows/    # ci.yml (checks + package), release.yml (v* tag publish)
├── .agents/              # setup, resume: orb environment scripts
├── .planning/codebase/   # Seven codebase-map documents (this folder)
├── Cargo.toml, Cargo.lock, rust-toolchain.toml, renovate.json
├── AGENTS.md             # Agent rules, commands, quirks
├── README.md, RELEASE_NOTES.md, LICENSE
└── .gitignore            # /target, /.cargo-home, /.amp/portals/*
```

## Directory Purposes

**`src/`:**
- Purpose: The whole app. Module tree from `src/main.rs`: `agent`, `catalog`, `notch`, `palette`, `prefs`, `settings`, `status`, `theme`, `update`.
- Contains: Rust modules with co-located `#[cfg(test)] mod tests`, and `src/pi-reader.mjs`.
- Key files: `src/main.rs`, `src/catalog.rs`, `src/agent.rs`, `src/prefs.rs`.
- Private includes (not declared in `src/main.rs`): `src/scan.rs` via `#[path = "scan.rs"] mod scan` in `src/catalog.rs`; `src/broker.rs` and `src/provider.rs` via `#[path]` in `src/agent.rs`; `src/settings_updates.rs` as `mod updates` in `src/settings.rs`.

**`native/`:**
- Purpose: Native update helper and its trust anchor (ADR 2).
- Contains: `native/Updater.swift` (compiled by `scripts/bundle.sh` to `Contents/MacOS/FolderLensUpdater`), `native/update-public-key.txt` (copied to `Contents/Resources`).
- Key files: `native/Updater.swift` (`@main struct FolderLensUpdater`, `Updater.check`, `presentArchive`, `selfTest`).

**`scripts/`:**
- Purpose: Local and CI packaging. No script uploads a release.
- Contains: POSIX shell and Python.
- Key files: `scripts/bundle.sh` (bundle ID `io.github.jellydn.folderlens`, `LSUIElement`, macOS 14.0, arm64 updater), `scripts/release.sh` (checks tag = `v<Cargo version>`, `codesign --verify`, `lipo -verify_arch arm64`).

**`tests/`:**
- Purpose: Tests that run the built binary or the JS extension.
- Contains: `tests/opencode_continuation.rs` (uses `CARGO_BIN_EXE_folderlens` and `/bin/sh` stubs for OpenCode 2.0.23 vs 2.2.0), `tests/pi-reader.mjs` (run with `node --test`).
- Unit tests live in `src/` (counts of `#[test]`: `src/main.rs` 26, `src/agent.rs` 9, `src/status.rs` 8, `src/prefs.rs` 7, `src/settings.rs` 7, `src/catalog.rs` 4, `src/scan.rs` 3, `src/broker.rs` 3, `src/notch.rs` 2, one each in `src/update.rs`, `src/theme.rs`, `src/settings_updates.rs`). Swift checks are `selfTest()` in `native/Updater.swift`.

**`shots/`:**
- Purpose: Screenshots for `README.md` and `docs/guide.md`.
- Contains: Current captures (`review-*.png`, `exclusion-rules.png`, `quick-*.png`, `notch-*.png`, `display-*.png`, `theme-settings.png`); historical provenance linked from `docs/guide.md` (`tray.png`, `settings-protected.png`, `settings-agent.png`, `review-agent-settings.png`); and older captures that `README.md`, `docs/guide.md` and `RELEASE_NOTES.md` do not link (`careful.png`, `balanced.png`, `aggressive.png`, `selected.png`, `confirmed.png`, `menu.png`, `settings-access.png`, `settings-general.png`).
- Regenerate `review-*.png` and `exclusion-rules.png` per `AGENTS.md` with `--theme dark`; keep historical files unchanged.

**`docs/`:**
- Purpose: User guide and accepted decisions.
- Key files: `docs/guide.md`, `docs/adr/0001-scoped-agent-sessions.md`, `docs/adr/0002-publisher-verified-updates.md`, `docs/adr/0003-review-works-without-ai.md`.

**`assets/`:**
- Purpose: App icon and documentation favicons.
- Key files: `assets/FolderLens.icns` (copied by `scripts/bundle.sh`), `assets/folderlens-icon.png` (source for `scripts/icons.py`). `assets/Shelf.icns` and `assets/shelf-icon.png` are pre-rename leftovers that no script reads.

**`.github/workflows/`:**
- Purpose: CI on `macos-26`.
- Key files: `.github/workflows/ci.yml` (fmt, clippy, test, node test, `scripts/release.sh`, artifact upload), `.github/workflows/release.yml` (same checks, signing, publish on `v*`; `prerelease` and `make_latest` follow a `-` in the tag).

## Key File Locations

**Entry Points:**
- `src/main.rs`: `main` (helper dispatch, then three Rune windows and workers).
- `src/broker.rs`: `helper` for `--folderlens-reader` and `--folderlens-read`.
- `src/provider.rs`: `resume_helper` for `--folderlens-opencode-resume`.
- `native/Updater.swift`: `FolderLensUpdater.main` (`--automatic`, `--manual`, `--self-test`, `--verify-release`).

**Configuration:**
- `Cargo.toml`: package, Rune pinned by git `rev`, `serde_json`; macOS-only `block2`, `libc`.
- `rust-toolchain.toml`: Rust 1.99.1 with clippy and rustfmt.
- `renovate.json`: dependency updates (Renovate owns `Cargo.lock` and the Rune `rev`, per `AGENTS.md`).
- `src/prefs.rs`: runtime settings schema; no settings file is checked in.

**Core Logic:**
- `src/main.rs`: tick derivation (`leaf_view`, `is_ticked`), scan dispatch, Record choice, `local_summary`.
- `src/catalog.rs` + `src/scan.rs`: candidate discovery and measurement.
- `src/palette.rs`: Actions and Ask start.
- `src/agent.rs` + `src/provider.rs` + `src/broker.rs`: agent review and read authority.
- `native/Updater.swift`: update verification.

**Testing:**
- `src/*.rs`: unit and headless Rune `Runtime` tests on `fixture_state()` (`src/main.rs`).
- `tests/opencode_continuation.rs`: binary-level continuation test.
- `tests/pi-reader.mjs`: Pi reader boundary.

## Naming Conventions

**Files:**
- Rust modules are snake_case by responsibility: `settings_updates.rs`, `palette.rs`.
- Private submodule files sit beside their owner and are included with `#[path]`: `src/scan.rs`, `src/broker.rs`, `src/provider.rs`, `src/settings_updates.rs`.
- JS uses `.mjs` (`src/pi-reader.mjs`, `tests/pi-reader.mjs`); shell `.sh`; Swift `Updater.swift`.
- Screenshots use kebab-case names that match their scene or purpose: `shots/review-ledger.png`, `shots/notch-compact.png`.
- ADRs: `docs/adr/NNNN-kebab-title.md`.
- Map documents: uppercase `.planning/codebase/ARCHITECTURE.md`.

**Directories:**
- Flat by role: `src/`, `native/`, `scripts/`, `tests/`, `shots/`, `docs/`, `assets/`.
- Runtime run folders: `chat-<nanos>-<pid>-<seq>` (`src/prefs.rs`: `prepare_run`); broker folders: `/tmp/folderlens-read-<pid>-<nanos>-<seq>` (`src/broker.rs`: `Broker::start`).

**Scenes and keys:**
- Synthetic scenes end in `-example` (plus `agent-error-details`) and load `catalog::fixture()` (`src/main.rs`: `live_state`). Other panel scenes scan the real Mac.
- Element keys are kebab-case strings used by tests: `"confirm"`, `"ask-agent"`, `"local-summary"`, `"notch-scan"`.

## Where to Add New Code

**New Feature:**
- Panel behavior, tick rules or footer: `src/main.rs`. Tests beside it on `fixture_state()`.
- Folder actions or keyboard shortcuts: `src/palette.rs`. If a scene opens the list, update `launch_size()` in `src/main.rs`.
- Notch controls: `src/notch.rs`; change review state only through `send`.
- Preferences: add the field to `Prefs` and both `load_from` / `save_to` in `src/prefs.rs`, then the UI in `src/settings.rs`.
- Scan rules: `RULES` in `src/scan.rs`; gate logic in `src/catalog.rs`.
- Tests: co-located `#[cfg(test)] mod tests` in the owning file; binary-level tests in `tests/`.

**New Component/Module:**
- Provider flags or private configuration: `src/provider.rs`; protocol and lifecycle: `src/agent.rs`; read authority: `src/broker.rs`. Keep ADR 1 aligned.
- Update policy or verification: `native/Updater.swift`, not UI polling. Keep ADR 2 aligned.
- A new top-level module needs `mod name;` in `src/main.rs`. Do not add `mod scan` there.

**Utilities:**
- Shared UI helpers (`button`, `ButtonStyle`, `rule`, `checkbox`): `src/main.rs`.
- Paths, protected roots, launchers: `src/prefs.rs` (`path_under`, `expand_home`, `always_name`, `prepare_run`, `resume_run`).
- Byte formatting: `catalog::format_bytes` in `src/catalog.rs`.
- Colors: tokens and `paint` in `src/theme.rs`.

## Special Directories

**`target/`:**
- Purpose: Cargo output, `target/release/FolderLens.app`, `target/release/assets`, `target/sparkle-2.10.1` signing tools.
- Generated: Yes (`cargo`, `scripts/bundle.sh`, `scripts/release.sh`, `scripts/sparkle-tools.sh`)
- Committed: No (`.gitignore`)

**`.cargo-home/`:**
- Purpose: Optional local Cargo home (`src/main.rs` doc comment example).
- Generated: Yes
- Committed: No (`.gitignore`)

**`.amp/`:**
- Purpose: Local agent thread files (`.amp/in/`) and portal registrations (`.amp/portals/`, ignored).
- Generated: Yes
- Committed: No

**`.planning/codebase/`:**
- Purpose: The seven codebase-map documents.
- Generated: By agents, reviewed by hand
- Committed: Yes

**`~/Library/Application Support/FolderLens/` (outside the repo):**
- Purpose: Runtime data: `settings.txt` (`src/prefs.rs`: `user_path`), `sizes.tsv` (`src/scan.rs`), `Runs/` with `context.md`, `Ask.command`, `agent.log`, `analysis.md`, `session.txt` and provider files (`src/prefs.rs`, `src/agent.rs`, `src/provider.rs`), `Updates/` with `last-check` and verified ZIPs (`native/Updater.swift`).
- Generated: Yes, at run time
- Committed: No. Never delete real runs, settings or conversations, also during tests (`AGENTS.md`). Tests use a temp `settings.txt` from `prefs::path_for_state()`.

**`~/Library/Application Support/Shelf/` (outside the repo):**
- Purpose: Legacy pre-rename settings. `prefs::load_from` imports `Shelf/settings.txt` once when FolderLens settings are absent, and leaves the original unchanged (`src/prefs.rs`).
- Generated: By older Shelf releases
- Committed: No

---

*Structure analysis: 2026-11-14*

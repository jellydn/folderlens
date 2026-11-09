# Codebase Structure

**Analysis Date:** 2026-11-09

## Directory Layout

- `src/`: Rust app, private protocol/scan modules, and the Pi reader extension.
- `native/`: Swift updater and committed publisher public key.
- `scripts/`: Local bundle, release archive, publisher signing and icon tools.
- `tests/`: Production-executable continuation integration test.
- `assets/`: App icon and documentation favicon resources.
- `shots/`: Current headless captures and retained historical images.
- `docs/`: Full guide and accepted boundary ADRs.
- `.planning/codebase/`: These seven source maps.
- `.github/workflows/`: macOS checks and tag publication.

This is one binary crate (`Cargo.toml`), not a frontend/backend monorepo.

## Directory Purposes

**Application (`src/`):** `main.rs` composes windows and Quick review state. `palette.rs` owns Actions; `settings.rs` owns preferences UI; `prefs.rs` owns persistence and run launchers. `status.rs` bridges the menu bar. `update.rs` launches, but does not implement, the native updater.

**Native helper (`native/`):** `Updater.swift` owns HTTP, publisher verification, manual-update policy, retained installer transactions and self-tests. `update-public-key.txt` contains no private signing material.

**Packaging (`scripts/`):** `bundle.sh` builds/signs the app and runs updater self-tests. `release.sh` creates DMG/ZIP/checksums. `sign-release.py` creates signed metadata/assets. `sparkle-tools.sh` obtains pinned signing tools. `icons.py` regenerates committed icon sizes.

**Documentation (`docs/`, `shots/`, `.planning/`):** `README.md` is the short entry point; `docs/guide.md` contains detailed features/limits; `docs/adr/` records authority and update decisions. Historical screenshots are provenance and must not be overwritten without scope.

## Key File Locations

**Entry Points:** `src/main.rs::main` (app and early reader helpers); `native/Updater.swift::ShelfUpdater.main` (native executable).

**Configuration:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `renovate.json`, `.github/workflows/ci.yml`, `.github/workflows/release.yml`. Runtime settings are defined by `src/prefs.rs`, not checked-in user files.

**Core Logic:**
- `src/catalog.rs` includes `#[path = "scan.rs"] mod scan`; do not declare scan again in `main.rs`.
- `src/agent.rs` privately includes `broker.rs` and `provider.rs`; these share the owning agent module's types.
- `src/settings.rs` privately includes `settings_updates.rs` as `updates`.
- `src/pi-reader.mjs` is embedded with `include_str!` and copied to each Pi run (`src/provider.rs`).

**Testing:** Co-located `#[cfg(test)] mod tests`; `tests/opencode_continuation.rs`; Swift `selfTest()` in `native/Updater.swift`.

## Naming Conventions

**Files:** Rust snake_case (`settings_updates.rs`), Swift `Updater.swift`, JS `.mjs`, shell `.sh`, and uppercase Markdown map names.

**Directories:** Responsibility-based source roots. Runtime runs use unique `chat-<time>-<pid>-<sequence>` names; broker directories use short names under `/tmp` to fit macOS socket limits (`src/prefs.rs`, `src/broker.rs`).

## Where to Add New Code

**New Feature:** Put selection/panel behavior in `src/main.rs`, folder actions in `src/palette.rs`, preferences UI in `src/settings.rs`, and scan rules in `src/scan.rs`. Extend existing contract tests beside each owner.

**New Component/Module:** Provider flags/configuration belong in `src/provider.rs`; common lifecycle/protocol behavior in `src/agent.rs`; read authority in `src/broker.rs`. Native update validation belongs in `native/Updater.swift`, not UI polling.

**Utilities:** Reuse canonical preference/path/launcher helpers in `src/prefs.rs`. Prefer changing the owner over adding one-use wrapper modules.

## Special Directories

**`target/`:** Generated Cargo output, `release/Shelf.app`, release assets and pinned signing tools. Ignored by `.gitignore`; not committed.

**`.cargo-home/`:** Optional local Cargo cache, ignored by `.gitignore`.

**`.amp/in/`:** Local thread review files, not a release input or an application storage directory.

**`~/Library/Application Support/Shelf/`:** Runtime `settings.txt`, `sizes.tsv`, `Runs/` and `Updates/`. Not repository fixtures. Never remove real conversations/preferences during tests.

---

*Structure analysis: 2026-11-09*

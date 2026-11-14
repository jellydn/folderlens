# Technology Stack

**Analysis Date:** 2026-11-14

Source: `main` at `0c68acb`, package `folderlens` 0.3.7 (`Cargo.toml`).

## Languages

**Primary:**
- Rust, edition 2021, toolchain 1.99.1 with `clippy` and `rustfmt` (`Cargo.toml`, `rust-toolchain.toml`). All app code is in `src/*.rs`. `src/catalog.rs` includes `src/scan.rs` with `#[path]`; `src/main.rs` has no `mod scan` (`AGENTS.md`).

**Secondary:**
- Swift (no version pin) for the separate update helper `native/Updater.swift`. `scripts/bundle.sh` compiles it with `xcrun swiftc -parse-as-library -O -target arm64-apple-macosx14.0`.
- JavaScript ES module for the Pi reader extension, `src/pi-reader.mjs`. It is embedded with `include_str!` and written into each Pi run folder (`src/provider.rs`). JSDoc types point to `@earendil-works/pi-coding-agent`; no JS toolchain is checked in (`AGENTS.md`).
- Python 3 for release signing (`scripts/sign-release.py`) and icon generation (`scripts/icons.py`).
- POSIX `sh` for packaging (`scripts/bundle.sh`, `scripts/release.sh`, `scripts/sparkle-tools.sh`). Run launchers `Ask.command` are `zsh` scripts (`src/prefs.rs`).

## Runtime

**Environment:**
- Native macOS accessory app (`LSUIElement`) with a menu-bar status item (`scripts/bundle.sh`, `src/status.rs`). No server and no web runtime.
- Rune draws the GPU UI. AppKit is called directly through `objc_msgSend` and `block2` blocks (`NSStatusBar`, `NSEvent`, `NSScreen`) in `src/status.rs`.
- The same `folderlens` binary also runs headless helper modes before the UI starts: `--folderlens-reader` (stdio MCP server), `--folderlens-read` (one read for Pi) and `--folderlens-opencode-resume` (`src/agent.rs` `reader_helper`, `src/broker.rs`, `src/provider.rs`).
- `FolderLensUpdater` is a second executable in `FolderLens.app/Contents/MacOS/`, started by `src/update.rs`.
- Node.js runs only `tests/pi-reader.mjs` in CI (`node --test`, `.github/workflows/ci.yml`); no Node version is pinned.

**Package Manager:**
- Cargo. `Cargo.lock` is committed (lockfile format version 4, 434 packages). CI and scripts use `--locked` (`.github/workflows/ci.yml`, `scripts/bundle.sh`).
- `uv run --with pillow scripts/icons.py` for icon regeneration (`scripts/icons.py`). No npm/pnpm/pip lockfile.
- Lockfile: present (`Cargo.lock`).

## Frameworks

**Core:**
- Rune (git dependency, crate version 0.0.1) from `https://github.com/anomalyco/rune`, pinned to rev `77bcc60037b115b56f85f57f1818b66eb846937d` in `Cargo.toml` and `Cargo.lock`. Views use `El`, `col`/`row`, `canvas`, `App`, `AppSpec`, `WindowSpec`, `MacWindowOptions`, and `rune::is_headless()` (`src/main.rs`, `src/update.rs`). Tests drive `rune::Runtime` headless (`src/settings_updates.rs`).
- Swift frameworks in the updater: Foundation, AppKit, Darwin, CryptoKit (`Curve25519.Signing`) (`native/Updater.swift`).

**Testing:**
- Rust built-in test harness: unit tests in `src/*.rs`, plus `tests/opencode_continuation.rs`, which runs the built binary through `CARGO_BIN_EXE_folderlens`.
- Provider tests use `/bin/sh` stub executables, never a real CLI or the network (`src/agent.rs` tests, `AGENTS.md`).
- `node:test` with `node:assert/strict` for the Pi reader boundary (`tests/pi-reader.mjs`).
- Updater self-tests: `FolderLensUpdater --self-test`, run at every bundle build (`scripts/bundle.sh`, `native/Updater.swift`).

**Build/Dev:**
- Cargo, rustfmt, Clippy (`cargo fmt --check`, `cargo clippy --locked --all-targets`) (`.github/workflows/ci.yml`).
- Apple tools: `xcrun swiftc`, `plutil`, `codesign` (ad-hoc, `--sign -`), `lipo`, `ditto`, `hdiutil`, `shasum` (`scripts/bundle.sh`, `scripts/release.sh`); `iconutil` and `qlmanage` (`scripts/icons.py`).
- Sparkle 2.10.1 `sign_update` only, downloaded and checked against a pinned SHA-256 in `scripts/sparkle-tools.sh`. Sparkle is not linked into the app.
- Headless screenshots: `cargo run -- --theme dark --scene <scene> --snap <png>` (`AGENTS.md`).

## Key Dependencies

**Critical:**
- `rune` (rev above): the whole UI, window and event model (`Cargo.toml`).
- `serde_json = "1"` (resolved 1.0.154): ACP and Pi RPC messages, MCP replies, per-run OpenCode/Copilot config, and the `agent_models` setting (`src/agent.rs`, `src/broker.rs`, `src/provider.rs`, `src/prefs.rs`, `Cargo.lock`).

**Infrastructure:**
- macOS-only `block2 = "0.6.2"`: AppKit event-monitor blocks (`Cargo.toml`, `src/status.rs`). `Cargo.lock` also has `block2` 0.5.1 and `objc2` 0.5.3/0.6.5 as transitive packages.
- macOS-only `libc = "0.2"` (resolved 0.2.191): `fcntl` non-blocking pipes, `kill` of process groups, `PATH_MAX` path lookups (`src/provider.rs`, `src/broker.rs`, `Cargo.lock`).
- Mach `host_statistics` / `mach_host_self` via raw `extern` for CPU ticks (`src/catalog.rs`).

## Configuration

**Environment:**
- `HOME` locates `~/Library/Application Support/FolderLens/` (`src/prefs.rs` `user_path`, `src/scan.rs`). `PATH` is searched for agent CLIs without running them (`src/prefs.rs` `find_bin`).
- Settings file: `settings.txt`, plain `key=value` lines. Keys: `agent_id`, `agent_bin`, `agent_models` (JSON map), `allow_start`, `send_paths`, `gauge_memory`, `gauge_cpu`, `shortcut_hints`, `notch_position`, `notch_display`, `theme`, `update_checks`, `update_downloads`, `update_prereleases`, `keep_hours`, and repeated `kept` lines for protected roots (`src/prefs.rs`). Legacy key `notch_below` is still read.
- Defaults: `allow_start` and `send_paths` off; `update_checks` on; `update_downloads` and `update_prereleases` off (`src/prefs.rs` `Default`, `src/settings_updates.rs` test).
- Legacy `~/Library/Application Support/Shelf/settings.txt` is copied once (mode 0600, no overwrite) when the FolderLens file is absent; the legacy file stays (`src/prefs.rs` `load_from`).
- Under `cfg(test)`, `prefs::path_for_state()` returns a temp file, so tests never touch the real settings (`src/prefs.rs`).
- Test-only env var `FOLDERLENS_STATUS_TEST_CHILD` isolates a status-item test in a child process (`src/status.rs`).
- Provider credentials stay in each CLI's own store; FolderLens reads none.

**Build:**
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`.
- `scripts/bundle.sh` sets `MACOSX_DEPLOYMENT_TARGET=14.0` and writes `Info.plist` (bundle ID `io.github.jellydn.folderlens`, `LSMinimumSystemVersion` 14.0, `FolderLensVersion` = full Cargo version; `-beta.N` becomes `bN` in `CFBundleVersion`).
- `scripts/release.sh` requires `GITHUB_REF_NAME` to equal `v<Cargo version>` on tag builds.
- `native/update-public-key.txt`: the committed Ed25519 public key (base64, 32 bytes), copied into `Contents/Resources/` (`scripts/bundle.sh`, `native/Updater.swift`).
- `.github/workflows/ci.yml`, `.github/workflows/release.yml`; Renovate with `config:recommended` (`renovate.json`) updates `Cargo.lock` and the Rune `rev` (`AGENTS.md`).
- `.agents/setup` installs the same toolchain pin for Amp orbs.

## Platform Requirements

**Development:**
- macOS with Xcode command-line tools and the pinned Rust toolchain. Full tests need macOS: one live disk/gauge test reads the real Mac (`AGENTS.md`, `src/main.rs`), and the code uses Mach, AppKit and macOS descriptor APIs.
- `node` for `tests/pi-reader.mjs`; `python3` and `uv` only for signing and icons.

**Production:**
- Apple Silicon (arm64 only; `lipo -verify_arch arm64` in `scripts/release.sh`), macOS 14+ (`scripts/bundle.sh`).
- Distributed as `FolderLens-<version>-macos-arm64.dmg` and `.zip` from GitHub Releases, or the `jellydn/tap/folderlens` Homebrew cask (`scripts/release.sh`, `README.md`).
- Builds are ad-hoc signed, not Developer ID signed and not notarized. Publisher Ed25519 signatures do not replace Gatekeeper (`scripts/bundle.sh`, `docs/adr/0002-publisher-verified-updates.md`, `AGENTS.md`).
- Sizes shown in the app are estimates (allocated blocks), not reclaimable space (`src/scan.rs`, `AGENTS.md`).

---

*Stack analysis: 2026-11-14*

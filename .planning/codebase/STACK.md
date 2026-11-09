# Technology Stack

**Analysis Date:** 2026-11-09

## Languages

**Primary:** Rust, edition 2021 (`Cargo.toml`), with toolchain 1.99.1 (`rust-toolchain.toml`).

**Secondary:** Swift for the native updater (`native/Updater.swift`); JavaScript ES module for the Pi extension (`src/pi-reader.mjs`); Python 3 and POSIX shell for packaging, signing and icons (`scripts/`).

## Runtime

**Environment:** Native macOS accessory app. Rune runs the GPU UI; AppKit supplies the status item. Reader and continuation helpers exit before UI startup (`src/main.rs`). No web runtime or server.

**Package Manager:** Cargo with committed `Cargo.lock`. The Pi extension uses its installed provider SDK, not a checked-in Node package workspace. Icon regeneration uses `uv run --with pillow scripts/icons.py`.

## Frameworks

**Core:** Rune, pinned to `aea4606de517a925bac382d1f1d4229bc61e5ff9` in `Cargo.toml`. Views use `El`, `AppSpec`, `WindowSpec` and a typed `Runtime`.

**Testing:** Rust's built-in harness; Rune headless layout/input tests; a production-executable continuation test (`tests/opencode_continuation.rs`); native updater self-tests (`native/Updater.swift`).

**Build/Dev:** Cargo, rustfmt and Clippy. Apple tools include `swiftc`, `plutil`, `codesign`, `ditto`, `lipo` and `hdiutil` (`scripts/bundle.sh`, `scripts/release.sh`). CI uses `macos-26`.

## Key Dependencies

**Critical:** `serde_json = "1"` for ACP/RPC, MCP and per-run provider configuration; macOS-only `block2 = "0.6.2"` for AppKit event blocks and `libc = "0.2"` for pipes, sockets and descriptor checks (`Cargo.toml`). Exact resolutions live in `Cargo.lock`.

**Infrastructure:** Mach CPU statistics (`src/catalog.rs`); IOKit/CoreFoundation GPU statistics (`src/gpu.rs`); raw Objective-C/AppKit status routing (`src/status.rs`); Foundation, AppKit, Darwin and CryptoKit updater (`native/Updater.swift`).

Sparkle 2.10.1 supplies the pinned Ed25519 signing executable only. It is not the installed updater framework (`scripts/sparkle-tools.sh`).

## Configuration

**Environment:** `HOME` locates `Library/Application Support/Shelf`; `PATH` locates configured CLIs. Preferences are plain text, with model choices encoded as JSON (`src/prefs.rs`). Provider credentials remain provider-managed.

**Build:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `scripts/bundle.sh`, `.github/workflows/ci.yml` and `.github/workflows/release.yml`. `MACOSX_DEPLOYMENT_TARGET=14.0` is set during bundling. Renovate maintains dependencies (`renovate.json`).

`native/update-public-key.txt` is the public publisher pin. `SPARKLE_PRIVATE_KEY` is available only to the signing job. Do not copy credentials into this map.

## Platform Requirements

**Development:** macOS with the pinned Rust toolchain and Apple command-line tools. Full source/tests use Mach and macOS descriptor APIs; Linux is not a supported full-test target.

**Production:** Apple Silicon, macOS 14+. Bundle ID `io.github.jellydn.shelf`; `LSUIElement` keeps it out of the Dock (`scripts/bundle.sh`). Releases are ad-hoc signed, not Developer ID signed or notarized. Publisher verification does not replace Gatekeeper.

---

*Stack analysis: 2026-11-09*

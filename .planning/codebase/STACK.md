# Technology Stack

**Analysis Date:** 2026-11-07

## Languages
**Primary:** Rust 2021 edition; toolchain 1.99.1 in `rust-toolchain.toml`.
**Secondary:** Swift for `native/Updater.swift`; JavaScript for `src/pi-reader.mjs`; Python and POSIX shell for release scripts.

## Runtime
**Environment:** Native macOS application, no web server.
**Package Manager:** Cargo with committed `Cargo.lock`; no repository JavaScript package manager configuration.

## Frameworks
**Core:** Rune GPU UI, revision `53c7f60f51ce12bd5da53145d9d86ce1552324e4` in `Cargo.toml`.
**Testing:** Rust built-in test harness; Rune headless Runtime; Swift updater self-tests.
**Build/Dev:** Cargo, rustfmt, Clippy, Apple command-line tools and Swift compiler.

## Key Dependencies
**Critical:** `serde_json` 1 for provider protocols; `block2` 0.6.2 for native callbacks; `libc` 0.2 for descriptor-scoped reads.
**Infrastructure:** AppKit, CryptoKit and Foundation in `native/Updater.swift`; IOKit/CoreFoundation in `src/gpu.rs`.
Sparkle 2.10.1 supplies release signing tools only; it is not Shelf's installer framework (`scripts/sparkle-tools.sh`).

## Configuration
**Environment:** User settings in Application Support, owned by `src/prefs.rs`; provider login remains the provider's responsibility.
**Build:** `Cargo.toml`, `rust-toolchain.toml`, `scripts/bundle.sh`, `.github/workflows/ci.yml` and `release.yml`.
Only the public publisher key is source-controlled; `SPARKLE_PRIVATE_KEY` is a Shelf Actions secret.

## Platform Requirements
**Development:** macOS with Rust and Apple command-line tools. Linux is not a supported full-test environment.
**Production:** Apple Silicon, macOS 14 or newer; app bundle ID `io.github.jellydn.shelf`.
Artifacts are ad-hoc signed, not Developer ID signed or notarized; publisher signatures do not replace Gatekeeper.

*Stack analysis: 2026-11-07*

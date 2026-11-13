# External Integrations

**Analysis Date:** 2026-11-09

## APIs & External Services

**Agent processes:** `src/provider.rs` adapts `grok`, `agy`, `copilot`, `pi` and `opencode`. Grok/Antigravity/Copilot/OpenCode use newline-delimited ACP; Pi uses RPC (`src/agent.rs`). OpenCode is gated to 2.0.23. Other detected tools have no adapter and fail visibly.
- Client: Rust process pipes and `serde_json`; no hosted inference SDK in Shelf.
- Auth: CLI-managed accounts/keys. Shelf does not log in, supply inference credit or promise free use.
- Consent: `allow_start` is required for each manual review; `send_paths` separately controls eligible path sharing (`src/prefs.rs`, `src/palette.rs`).

**GitHub updates:** `native/Updater.swift` discovers up to 100 releases at `api.github.com/repos/jellydn/shelf/releases?per_page=100`. It constructs fixed `github.com/jellydn/shelf/releases/download/<tag>/` URLs, then verifies metadata and archive content with the pinned publisher key. No scan or agent data is sent.

**System services:** Finder/Terminal launch through `open`; folder selection uses `osascript`; Full Disk Access opens System Settings. Scan/gauge inputs include `ps`, `diskutil`, `sysctl`, `vm_stat` and Mach (`src/prefs.rs`, `src/palette.rs`, `src/catalog.rs`).

## Data Storage

**Databases:** None. Preferences live in `~/Library/Application Support/Shelf/settings.txt` (`src/prefs.rs`). Model choices are stored per provider, or per custom executable.

**File Storage:** `Runs/chat-…/` holds `context.md`, `Ask.command`, redacted `agent.log`, and success-only `analysis.md`/`session.txt`. Pi adds `sessions/` and its extension; OpenCode adds private configuration directories (`src/agent.rs`, `src/provider.rs`). Run directories are mode 0700 and persist until the user removes them.

**Caching:** `sizes.tsv` stores versioned size/newest-time/root-mtime data (`src/scan.rs`). `Updates/` holds verified archives, a daily-check timestamp and a lock file (`native/Updater.swift`). These are local files, not cloud storage.

## Authentication & Identity

**Auth Provider:** Existing CLI setup. Blank models can use paid CLI defaults; Copilot blank selects Auto; OpenCode requires an explicit model. Exact advertised model checks precede prompts (`src/provider.rs`, `src/agent.rs`).

Shelf publisher identity comes from `native/update-public-key.txt`. Ad-hoc Apple code signatures and publisher Ed25519 signatures serve different purposes. All current installs fail the direct-install provenance gate (`native/Updater.swift`).

## Monitoring & Observability

**Error Tracking:** No remote telemetry or error-reporting service is configured.

**Logs:** Per-run redacted agent diagnostics, inline status/errors with Retry, and local updater notices. Do not log local paths, credentials or signing secrets (`src/agent.rs`, `native/Updater.swift`).

## CI/CD & Deployment

**Hosting:** GitHub Releases for `jellydn/shelf`. The stable Homebrew cask is maintained separately in `jellydn/homebrew-tap`; beta releases do not update it (`AGENTS.md`).

**CI Pipeline:** `.github/workflows/ci.yml` checks formatting, Clippy, tests and packaging on `macos-26`. `.github/workflows/release.yml` does the same for matching `v*` tags, then signs metadata/ZIP/DMG and independently verifies them before publication. Only the current tag's section of `RELEASE_NOTES.md` is published; a mismatched heading fails. Prerelease tags set `prerelease: true` and `make_latest: false`; stable tags set latest.

## Environment Configuration

**Required env vars:** Release signing needs `SPARKLE_PRIVATE_KEY`. Runtime uses `HOME` and `PATH`. Per-run controls include `NO_COLOR`, `GROK_FOLDER_TRUST`, `SHELF_READER_EXECUTABLE`, `SHELF_READER_SOCKET`, `SHELF_EXPECTED_MODEL` and private `OPENCODE_*` overrides (`src/provider.rs`). Unsafe Copilot overrides are removed. `HOMEBREW_PREFIX` helps cask detection but cannot prove direct installation.

**Secrets location:** Dedicated Shelf Actions signing secret and each provider's own credential store. No private key or provider credential is committed.

## Webhooks & Callbacks

**Incoming:** Private `/tmp/shelf-read-…/read.sock`, bounded JSON forwarding and stdio MCP helpers (`src/broker.rs`); Pi explicit extension callbacks (`src/pi-reader.mjs`); AppKit status callbacks (`src/status.rs`). No public webhook or HTTP listener.

**Outgoing:** ACP/RPC messages to configured local processes, GitHub HTTPS update requests, and explicit Finder/Terminal actions. No automatic model request or cleanup command.

---

*Integration audit: 2026-11-09*

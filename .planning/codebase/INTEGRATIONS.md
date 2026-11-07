# External Integrations

**Analysis Date:** 2026-11-07

## APIs & External Services
**Agents:** Grok/Agy/Copilot/OpenCode use ACP; Pi uses RPC (`src/agent.rs`, `src/provider.rs`).
Provider credentials and inference accounts are managed by the CLI, not Shelf. Free labels do not guarantee free account usage.
**Updates:** GitHub release discovery over HTTPS, followed by pinned Ed25519 verification (`native/Updater.swift`).

## Data Storage
**Databases:** None. `src/prefs.rs` owns plain-text settings.
**File Storage:** Private local run folders hold context, reports, session IDs and launchers. They persist until the user removes them.
**Caching:** `src/scan.rs` uses a size cache; the updater uses a private Updates directory and persistent daily-check timestamp.

## Authentication & Identity
**Auth Provider:** Existing CLI login/account/key setup. Shelf asks for agent-start and optional path-sharing permission.
Publisher identity comes from `native/update-public-key.txt`, not unsigned GitHub metadata.

## Monitoring & Observability
**Error Tracking:** No remote error-tracking integration in this repository.
**Logs:** Bounded redacted agent diagnostics; raw local paths/credentials must not appear in errors (`src/agent.rs`).

## CI/CD & Deployment
**Hosting:** `jellydn/shelf` GitHub Releases; stable cask in `jellydn/homebrew-tap`.
**CI Pipeline:** Formatting, Clippy, full tests and packaging on macos-26; tag workflow also signs/verifies before publication.
The release workflow publishes prerelease tags as not latest (`.github/workflows/release.yml`).

## Environment Configuration
**Required env vars:** Signing job requires `SPARKLE_PRIVATE_KEY`; provider-specific private overrides are set per run in `src/provider.rs`.
**Secrets location:** Shelf Actions secret for publishing; provider-managed authentication for inference. Never copy Oak's signing key.

## Webhooks & Callbacks
**Incoming:** Private Unix socket and stdio MCP/JSON helpers, not a network listener (`src/broker.rs`).
**Outgoing:** Agent process messages and GitHub HTTPS requests; no automatic model requests.

*Integration audit: 2026-11-07*

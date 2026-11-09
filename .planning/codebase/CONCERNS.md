# Codebase Concerns

**Analysis Date:** 2026-11-09

Fresh source audit of the whole repository. Findings below distinguish fixed defects, deliberate policy, source limits and unverified native reports. A risk is not proof of a failing native path.

## Tech Debt

**Provider schema maintenance:**
- Issue: Five adapters have different model/mode/session contracts; OpenCode accepts only verified 2.0.23.
- Files: `src/provider.rs`, `src/agent.rs`, `src/pi-reader.mjs`.
- Impact: CLI upgrades can reject startup or continuation. This is fail-closed behavior, not a reason to remove version checks.
- Fix approach: Validate each new schema with harmless stubs and native metadata checks before broadening support.

## Known Bugs

**Fixed in this refresh — agent transport resource limits:**
- Symptoms: Before this change, an agent could send more than 2 MiB without a newline and leave the review waiting. Whole-line allocation preceded the report/log limits; the protocol queue was unbounded. Empty diagnostic lines also escaped the byte counter because newline bytes were not counted.
- Files: `src/agent.rs::analyze` and its co-located regression tests.
- Evidence: A harmless `grok` stub emitted 2,097,153 bytes without a newline and held the pipe open. The new regression failed on unchanged production code after its three-second deadline.
- Resolution: Protocol lines are capped before JSON parsing at 2 MiB, with an eight-message queue. Diagnostic lines are capped before redaction; truncated data is discarded and drained without allocation. The 16 KiB log budget includes newlines. Failure saves no completed report/session; the 256 KiB report limit is unchanged.

**Reported Settings→tray reopening issue — cause unverified:**
- Symptoms: Earlier native runs reported that the tray could stop reopening after Settings. A separate diagnostic also failed its initial tray-open control, so a Settings-specific cause is not established.
- Files: `src/status.rs`, `src/settings.rs`; report/limits in `docs/guide.md` and `RELEASE_NOTES.md`.
- Trigger: Native interaction, not reproduced by headless navigation or isolated callback tests.
- Workaround: No verified fix is claimed. Keep explicit Close/Escape controls and gather a controlled native trace before changing lifecycle policy.

**Historical external/version-specific failures:** `AGENTS.md` records a Grok socket-symlink sandbox refusal. `src/agent.rs` surfaces it without disabling protections; it was not rechecked against a real CLI in this audit. `RELEASE_NOTES.md` records beta.1's archive timeout; current `native/Updater.swift::ArchiveDownload` uses a bounded native download task. Old immutable releases are not replaced.

## Security Considerations

**Read authority:** Risk is unintended folder exposure. `src/broker.rs` freezes original grants, intersects current scope, rejects symlink escapes/special files and rechecks opened descriptors. `src/palette.rs::ask_access` blocks always-protected, kept, held, in-use and unselected paths. Retain real filesystem/revocation tests; prompts alone do not enforce this boundary.

**Provider boundary differences:** Copilot/OpenCode/Pi use only Shelf's broker controls. Grok/Antigravity Terminal continuation uses native plan/sandbox controls and does not enforce the same live selected-only boundary (`src/provider.rs`). None is an independent OS sandbox; do not claim uniform tool enforcement or cleanup safety.

**Persistent shared context:** Run folders can contain explicitly shared paths and externally sent context (`src/prefs.rs`, `src/agent.rs`). They are private but retained. Users must remove them explicitly; do not auto-delete chats to address retention risk.

**Publisher and Apple trust:** Pinned signatures, expiry/channel/version checks and manual-only unknown-origin policy fail closed (`native/Updater.swift`). Dedicated key loss/compromise requires separate recovery review. Builds are ad-hoc signed, not notarized (`scripts/bundle.sh`); do not bypass Gatekeeper or replace the key silently.

## Performance Bottlenecks

**Filesystem work:** `src/scan.rs` recursively measures candidates with up to eight workers; project discovery prunes known build/dependency names and caps depth. Root-mtime cache entries are estimates, not recursive change monitoring. First/live snapshots can be slow. `src/main.rs` keeps initial scan/gauges off the UI thread; explicit Measure folder again in `src/palette.rs::analyse` still measures synchronously. No latency benchmark was run, so a measured performance regression is not claimed. Moving manual remeasurement to a worker needs a separate lifecycle/state design.

## Fragile Areas

**Native window lifecycle:** `src/status.rs` uses raw Objective-C ABI and macOS 27 expanded-interface callbacks, with an older-system action path. Current policy keeps outside clicks and transient session ends open; Close/Escape/tray toggle dismiss. Callback/policy tests do not prove AppKit routing. Change only with controlled native reproduction and checks on both paths.

**Installer ownership and transaction:** `native/Updater.swift::requireDirectInstallation` rejects every current distribution before consent/replacement. This is deliberate safety policy, not a missing preference. Retained swap/rollback fixtures do not prove recovery from a post-launch crash. Positive provenance needs separate review; missing cask receipts, writable paths and copied markers are insufficient.

## Scaling Limits

**Reader:** 512 KiB UTF-8, 2,000 returned lines and 1,024 directory entries; one broker connection is serviced at a time (`src/broker.rs`). These are privacy/resource boundaries, not targets to increase without review.

**Agent:** Ten-minute review deadline, 256 KiB report, 2 MiB protocol line, eight queued protocol messages, 16 KiB redacted log (`src/agent.rs`). Descendant-held output pipes can outlive the direct child; cancellation/version/broker tests cover selected cases, not arbitrary provider process trees.

**Updates:** First 100 release records, 2 MiB discovery response, 16 KiB metadata, 512 MiB archive maximum; daily automatic throttle (`native/Updater.swift`). Pagination is not implemented. This is a documented preview limit, not evidence of a missed current release.

## Dependencies at Risk

**Rune:** Git revision is pinned in `Cargo.toml`/`Cargo.lock`; Renovate proposes updates (`renovate.json`). Native window/input changes need macOS packaging and rendered/input checks, not only compilation. Do not absorb unrelated dependency PRs into this audit.

**Provider accounts/models:** Exact IDs, free availability, quotas and privacy can change outside Shelf (`src/agent.rs::model_note`, `src/provider.rs`). Manual start consent and no paid fallback remain required. CLI defaults are not proof of free inference.

## Missing Critical Features

**No cleanup/delete engine:** Intentional: `src/main.rs` only records an eligible choice. This is a prototype, not a production cleanup utility. Do not add deletion as an audit fix.

**Preview conveniences:** No persistent download-ready indicator or automatic rollback pruning (`native/Updater.swift`, `src/settings_updates.rs`). A later manual check gives download guidance. `src/palette.rs` stores an alerts-paused notice/deadline, but no periodic agent-analysis scheduler exists. None authorizes automatic paid requests or deleting old backups.

## Test Coverage Gaps

**Native agents — high:** Stubs validate protocol contracts, not real model-turn tool routing or Terminal continuation (`src/agent.rs`, `tests/opencode_continuation.rs`, `src/pi-reader.mjs`). No real model request is needed for this audit; cost/data consent remains separate.

**Native tray — high:** Headless layouts and isolated callbacks do not verify tray initial-open, Settings→tray or macOS 27 session delivery (`src/status.rs`). The known report remains explicit.

**Native updater — high before enabling replacement:** Publisher/checksum/bundle verification is separate from actual install/restart/Gatekeeper behavior (`native/Updater.swift`). Current replacement is blocked. Use disposable fixtures, never the owner's installed app.

**Measurements — medium:** Fixture scans and a live Mac check do not establish freshness or responsiveness on every home tree (`src/scan.rs`, `src/catalog.rs`). Do not call estimates reclaimable space or recent-change gates live monitoring.

---

*Concerns audit: 2026-11-09*

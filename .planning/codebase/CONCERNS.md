# Codebase Concerns

**Analysis Date:** 2026-11-07

## Tech Debt
**Provider surface:** CLI schemas differ and change; `src/provider.rs` deliberately accepts OpenCode 2.0.23 only. Verify new schemas before extending that gate.

## Known Bugs
**Grok sandbox startup:** Installed CLI reports a socket-symlink deny-list failure (`src/agent.rs` diagnostics). Shelf surfaces the failure; there is no approved sandbox bypass.
**Published beta.1 download:** Its native per-byte archive transfer timed out during post-release verification. PR #10 switches archives to a bounded native file transfer; disposable checks downloaded and verified the published ZIP without installation. The published beta.1 assets do not contain this fix; manual installation remains available.

## Security Considerations
**Reader boundary:** `src/broker.rs` freezes original grants, intersects current eligible state and checks opened descriptors. Keep revocation/symlink/special-file tests; prompt instructions alone are insufficient.
**Continuation difference:** Grok/Agy Terminal uses native plan/sandbox controls, not the selected-only broker (`src/provider.rs`). Do not claim equal enforcement across all providers.
**Publisher identity:** `native/Updater.swift` pins a dedicated key and fails closed. Key loss or compromise requires an explicit recovery/reissue plan, not unsigned fallback. Metadata expires after a year.
**Platform trust:** Ad-hoc signing/no notarization means Gatekeeper may require Open Anyway; publisher verification is separate. Do not clear quarantine or disable Gatekeeper.

## Performance Bottlenecks
**Scan cost:** `src/scan.rs` can be slow on large home trees; size caching and background workers prevent UI-thread scans. Snapshot commands still load live state.

## Fragile Areas
**Native lifecycle:** `src/status.rs` uses raw AppKit callbacks and macOS 27 expanded interfaces; geometry tests are not complete native event coverage.
**Installer transaction:** `native/Updater.swift` validates then stages/replaces/restarts. Harmless rollback fixtures pass; a full native install/restart has not been exercised.
**Installation ownership:** Standard Caskroom layouts and `HOMEBREW_PREFIX` are detected for guidance, not as proof of direct installation. Unknown origin fails closed before consent and replacement, including custom/moved casks without a GUI prefix. Current packages have no trustworthy direct-install provenance, so native replacement is disabled for every current installation. Any future positive-ownership mechanism needs separate review; neither a writable preference nor a copied bundle marker is sufficient.
**Download readiness/backups:** Automatic downloads need a later manual check for manual-install guidance; no persistent ready indicator exists. Rollback copies are retained, not automatically pruned. Do not remove real backups merely to satisfy a review comment.

## Scaling Limits
**Reads:** 512 KiB UTF-8, 2,000 lines or 1,024 directory entries (`src/broker.rs`). Increasing limits requires deliberate resource/privacy review.
**Updates:** Release discovery examines up to 100 releases; archives are bounded to 512 MiB (`native/Updater.swift`).

## Dependencies at Risk
**Rune:** Git-pinned revision in `Cargo.toml`; native API changes need macOS packaging and UI verification.
**Provider models:** Free offerings/quotas/privacy may change. Validate exact advertised access and retain manual start permission (`README.md`).

## Missing Critical Features
**Interval analysis:** Configurable local scan/alert pause versus paid agent calls remains deferred to [issue #5](https://github.com/jellydn/shelf/issues/5); updater timing is not agent analysis timing.

## Test Coverage Gaps
**Native agents:** Metadata-only probes and stubs are not live model-turn/tool-routing/Terminal proof (`src/agent.rs`). High priority before claiming production parity.
**Native updater:** Published asset verification and automatic-download checks must be reported separately from actual replacement/restart. Never test on an existing user installation.

*Concerns audit: 2026-11-07*

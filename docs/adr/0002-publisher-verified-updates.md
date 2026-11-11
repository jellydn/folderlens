# 2. Verify publisher updates and require install approval

Date: 2026-11-07

## Status
Accepted by the owner for the prerelease. No Apple Developer account is available.
Amended 2026-11-08: unknown-origin installations are manual-update-only. One current-state beta.2 prerelease is authorized; promotion and subsequent releases remain paused.
Amended 2026-11-09: the owner authorizes stable 0.2.0 and current documentation updates. The manual-update-only rule and all publisher, consent and verification requirements remain unchanged.
Amended 2026-11-11: for FolderLens 0.3.3, the owner selects Check → Download and verify → Reveal in Finder, with manual final replacement. The installation-ownership boundary remains unchanged.

## Context
Shelf needs update checks and optional downloads without automatic install/restart or a paid Apple account. Oak provides a working Ed25519/signing-secret pattern, but Sparkle's automatic-download path can install on quit, which conflicts with the approved policy.

## Decision
Use Oak's pinned Sparkle 2.10.1 signing tools with a **dedicated Shelf key**, not Oak's key. Commit only the public pin (`native/update-public-key.txt`); keep private material in Shelf Actions `SPARKLE_PRIVATE_KEY`. Sign metadata and assets in `.github/workflows/release.yml`, then independently verify before publication.

Use `native/Updater.swift` as a separate native helper launched by `src/update.rs`, not Sparkle's automatic installer. Check daily, permit automatic downloads only by opt-in, default to stable, and require an opt-in for prereleases. Every install/restart needs separate explicit approval; quitting is not approval. Homebrew-managed apps use `brew upgrade`, never self-replacement; the stable cask excludes beta tags.

Construct fixed GitHub release URLs, verify pinned signed metadata (tag/channel/version/expiry), and verify ZIP size/hash before use. Validate bundle identity/full version/key continuity and strict ad-hoc code signatures; stage on the destination filesystem and keep a rollback copy. Reject missing/invalid publisher proof and unsafe/nonwritable/translocated locations. Do not clear quarantine or bypass Gatekeeper. Publish `v0.2.0-beta.1` from the PR commit as prerelease and not latest; this does not authorize merging the PR.

Use native file download callbacks with a signed-size cancellation bound, then exact size/hash verification. Explicitly quarantine the downloaded archive and staged app. The retained installer exchanges staged/installed bundles atomically so interruption cannot leave the installed path absent; it retains the previous app and rolls back a reported launch failure. This does not prove recovery from an app that crashes after `open` returns.

Require positive direct-install ownership before both installation consent and replacement. Failure to find a Homebrew receipt is not proof of direct ownership: Finder may omit a custom prefix, and moving a cask app can leave stale artifact links. Unknown origin is manual-update-only, with Homebrew guidance and a verified ZIP/manual-release route. Current distributions provide no trustworthy direct-install provenance, so all current installations are blocked from native replacement. Keep the consent/transaction code and fixture coverage, but do not enable it with a writable preference or copied bundle marker. A future provenance mechanism requires separate review before any direct installation can qualify.

For current bundles, a manual check offers the verified ZIP through a separate Reveal in Finder action instead of reaching installation consent. Recheck archive integrity at this presentation boundary. Automatic downloads never open Finder, replace the app or grant installation. Detected Homebrew installations receive upgrade guidance before any archive download.

## Considered Options
- Sparkle automatic installation is mature but its install-on-quit path violates the required consent boundary.
- TLS and unsigned checksums do not establish the pinned publisher identity.
- Developer ID signing/notarization would improve Apple trust, but requires an account the owner does not have.
- Updating the stable Homebrew cask to previews would silently change its release channel.
- Scanning arbitrary prefixes cannot prove absence of Homebrew ownership; bundle markers, path heuristics and user preferences can survive a cask move. Fail closed instead of treating absence as positive proof.

## Consequences
### Positive
- Publisher authentication works without Apple paid credentials and remains distinct from macOS platform trust.
- Automatic downloads cannot silently install, and stable Homebrew users retain their distribution path.
### Negative
- A custom helper adds verification/transaction maintenance and needs full native replacement testing beyond fixtures.
- All current installations require manual updates. This trades convenience for the approved ownership boundary; signed downloads remain available without enabling replacement.
- Ad-hoc builds are not notarized; macOS may require Privacy & Security → Open Anyway. No Gatekeeper bypass is provided.
- Metadata expires after one year. Lost/compromised keys require explicit secure recovery; there is no unsigned fallback.

See `scripts/sign-release.py`, `scripts/sparkle-tools.sh`, `RELEASE_NOTES.md` and `.planning/codebase/TESTING.md` for publication and verification limits.

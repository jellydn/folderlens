# 2. Verify publisher updates and require install approval

Date: 2026-11-07

## Status
Accepted by the owner for the prerelease. No Apple Developer account is available.

## Context
Shelf needs update checks and optional downloads without automatic install/restart or a paid Apple account. Oak provides a working Ed25519/signing-secret pattern, but Sparkle's automatic-download path can install on quit, which conflicts with the approved policy.

## Decision
Use Oak's pinned Sparkle 2.10.1 signing tools with a **dedicated Shelf key**, not Oak's key. Commit only the public pin (`native/update-public-key.txt`); keep private material in Shelf Actions `SPARKLE_PRIVATE_KEY`. Sign metadata and assets in `.github/workflows/release.yml`, then independently verify before publication.

Use `native/Updater.swift` as a separate native helper launched by `src/update.rs`, not Sparkle's automatic installer. Check daily, permit automatic downloads only by opt-in, default to stable, and require an opt-in for prereleases. Every install/restart needs separate explicit approval; quitting is not approval. Homebrew-managed apps use `brew upgrade`, never self-replacement; the stable cask excludes beta tags.

Construct fixed GitHub release URLs, verify pinned signed metadata (tag/channel/version/expiry), and verify ZIP size/hash before use. Validate bundle identity/full version/key continuity and strict ad-hoc code signatures; stage on the destination filesystem and keep a rollback copy. Reject missing/invalid publisher proof and unsafe/nonwritable/translocated locations. Do not clear quarantine or bypass Gatekeeper. Publish `v0.2.0-beta.1` from the PR commit as prerelease and not latest; this does not authorize merging the PR.

## Considered Options
- Sparkle automatic installation is mature but its install-on-quit path violates the required consent boundary.
- TLS and unsigned checksums do not establish the pinned publisher identity.
- Developer ID signing/notarization would improve Apple trust, but requires an account the owner does not have.
- Updating the stable Homebrew cask to previews would silently change its release channel.

## Consequences
### Positive
- Publisher authentication works without Apple paid credentials and remains distinct from macOS platform trust.
- Automatic downloads cannot silently install, and stable Homebrew users retain their distribution path.
### Negative
- A custom helper adds verification/transaction maintenance and needs full native replacement testing beyond fixtures.
- Ad-hoc builds are not notarized; macOS may require Privacy & Security → Open Anyway. No Gatekeeper bypass is provided.
- Metadata expires after one year. Lost/compromised keys require explicit secure recovery; there is no unsigned fallback.

See `scripts/sign-release.py`, `scripts/sparkle-tools.sh`, `RELEASE_NOTES.md` and `.planning/codebase/TESTING.md` for publication and verification limits.

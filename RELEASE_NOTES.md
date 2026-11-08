## Shelf 0.2.0-beta.3 — Review ledger preview

**Experimental WIP PoC. Record choice (formerly Confirm) records a selection in memory; it does not delete files or run cleanup.** Footprints are estimates, not reclaimed space or proof of cleanup safety.

- First independent Review ledger implementation from PR #13: reasons/exclusions beside a non-deleting receipt, compact system readings, responsive stacking and clearer folder actions/Protected/Access settings. Design review is still in progress; this prerelease does not merge the PR or declare the presentation review complete.
- Short README, with detailed agent/update guidance and historical screenshots retained in `docs/guide.md`.
- Existing scoped broker, protection rules and fail-closed updater remain. All current installations are manual-update-only; signed checks and opt-in downloads remain available. Homebrew installs must use Homebrew.

### Credit and scope

[Levo](https://omggg.wtf/levo/) was the direct UI/layout/interaction reference for Shelf's original presentation in beta.2 and earlier. Please support or buy Levo through its official site. Shelf is not affiliated with or endorsed by Levo. Attribution neither resolves the earlier copied-presentation concern nor establishes equivalent cleanup safety; original assets/history are preserved.

This is the specifically authorized beta from the latest PR's exact verified commit, **prerelease and not latest stable**, not a main merge. Existing tags/assets and the stable Homebrew tap are unchanged. Promotion and later releases remain paused.

### Assets and verification limits

Apple Silicon, macOS 14+. DMG and ZIP contain the same ad-hoc signed app. Check `SHA256SUMS`; signed update metadata, ZIP and DMG are verified with Shelf's pinned Ed25519 publisher key. Builds are **not Developer ID signed or notarized**. Use normal macOS approval if required; no quarantine removal or Gatekeeper bypass is provided.

Rust tests, GPU-rendered headless UI, updater self-tests and package/signature checks cover the implementation. Native tray callbacks, real model/tool/Terminal turns and actual installation/restart/Gatekeeper behavior remain unverified. Agent access is manual/opt-in; provider costs/data policies apply, and provider controls are not an independent OS sandbox. No real cleanup, paid model prompt or installed-app replacement was performed for this preview.

## Shelf 0.2.0-beta.2 — current-state prerelease

**Experimental WIP PoC, not a finished cleanup utility.** Cleanup levels preview selections and estimated space. **Confirm records a choice in memory and does not delete files or run cleanup.**

### Levo credit and presentation status

[Levo](https://omggg.wtf/levo/) was the direct UI, layout and interaction-behavior reference for Shelf's current presentation. Please support Levo and consider buying it through its official site. Shelf is not affiliated with or endorsed by Levo. This attribution does not resolve the copied-presentation concern or establish Levo-equivalent cleanup safety.

An independent Shelf presentation is **not complete** and is not included in this release. This is the one authorized current-state prerelease, **not latest stable**. Promotion and subsequent releases remain paused. Existing beta.1 assets and provenance are unchanged; the stable Homebrew tap remains on its stable channel.

### Changes and update ownership

- Painter-designed app icon in the bundle/README, with a matching favicon package for documentation sites.
- Pointer exit no longer dismisses the panel. Outside-click policy, second-click toggle, Escape and macOS 27 delegate callbacks are retained. Policy tests pass; native tray interaction remains unverified.
- Bounded native archive download, explicit quarantine, atomic bundle exchange and disposable rollback checks. No real app replacement/restart or Gatekeeper approval was exercised.
- Cancellation-aware prompt/version writes/reads, exact OpenCode continuation version check, post-mode model validation, Pi continuation model guard and stronger broker revocation/shutdown.
- Unknown installation origin is now **manual-update-only**, including custom-prefix Homebrew apps launched without `HOMEBREW_PREFIX`, moved cask apps and aliases. Current packages provide no reliable positive direct-install provenance, so native replacement is disabled for all current installations, even with approval. Verified checks and opt-in downloads remain available; a manual check gives guidance. If Homebrew installed Shelf, use `brew upgrade --cask jellydn/tap/shelf`; otherwise install the verified ZIP or DMG manually.
- Non-blocking preview limits: no persistent download-ready indicator, first-100 release discovery and retained old rollback copies. No real backups were pruned.

### Installation, signatures and verification limits

Apple Silicon, macOS 14 or later. Download the beta.2 DMG or ZIP explicitly and check `SHA256SUMS`. Metadata, ZIP and DMG carry signatures verified against Shelf's pinned dedicated Ed25519 publisher key. This is distinct from Apple trust: builds are **ad-hoc signed, not Developer ID signed or notarized**. macOS may require **Privacy & Security → Open Anyway**. No Apple Developer account, quarantine removal or Gatekeeper bypass is used.

Provider/model settings, bounded selected-folder tools and exact-session launchers are implemented, but protocol stubs and metadata probes do not verify native model turns, tool routing during a model turn or Terminal continuation. Tray event routing, actual replacement/restart, Gatekeeper approval and recovery from a post-launch crash remain unverified. Native replacement is blocked by the ownership gate, not advertised as a verified install path. Publisher verification, download fixtures and rollback fixtures are separate checks, not end-to-end proof of those paths.

## Shelf 0.2.0-beta.1 — prerelease

Preview of provider/model support and publisher-verified updates for Apple Silicon Macs, macOS 14 or later. This release is **not latest stable** and is not installed by the stable Homebrew cask.

**Post-release download limitation:** the native updater's per-byte archive transfer timed out during verification. Download the DMG or ZIP from this release manually if the in-app download fails. PR #10 contains a native file-transfer fix, tested against these signed assets, but the published beta.1 tag/assets have not been replaced. Full app replacement/restart remains unverified.

- Ask Agent adapters for GitHub Copilot CLI, Pi and OpenCode, alongside Grok/Antigravity. OpenCode requires verified version 2.0.23.
- Per-agent model choices in Settings, exact saved-session launchers, current-selection read broker, inline error recovery and privacy/account/cost notes. Free-model access can require an account/key and has limits; blank defaults can be paid. There are no automatic agent requests.
- Settings → Updates: daily checks, opt-in downloads, stable default and opt-in prereleases. Installation/restart always requires separate confirmation; nothing installs on quit. Homebrew installations use `brew upgrade` and are never self-replaced.
- Dedicated pinned Ed25519 publisher signatures on update metadata/assets, using Oak's signing-tool/Actions-secret pattern with a separate Shelf key. No unsigned fallback; metadata expires one year after signing.

**Confirm only records a choice. Shelf never deletes files or runs cleanup.**

### Install and security

Download the DMG or ZIP explicitly from this prerelease. The app is ad-hoc signed, **not Developer ID signed or notarized**; no Apple Developer account was used. Publisher update signatures authenticate releases but do not replace Apple's trust decisions. macOS may require **System Settings → Privacy & Security → Open Anyway**. Shelf never removes quarantine or disables Gatekeeper. Intel builds are not provided.

Signed metadata and ZIP size/SHA-256 are checked before installation, followed by bundle identity/version/key continuity and strict code-signature validation. Installation requires a writable non-translocated app location. A previous app copy is retained for rollback; settings/chats are preserved.

### Verification limits

Harmless protocol stubs cover provider failures, exact sessions, model validation and broker boundaries. Native metadata checks verified Copilot plan/Auto, Pi model state and OpenCode private-mode/exact-model selection without a model prompt. Real native model turns, tool routing during a turn and Terminal continuation remain unverified. Grok's existing sandbox startup issue is reported, not bypassed. Grok/Antigravity Terminal continuation retains native plan/sandbox rather than the new broker.

Updater tests cover signature/metadata/archive rejection, versions/channels, Homebrew detection and policy defaults. Native installation/restart is a separate verification boundary; see the PR for actual exercised paths. Do not treat a successful signature check as proof of a completed application replacement.

---

## Shelf 0.1.0

First public release for Apple Silicon Macs running macOS 14 or later.

- Menu-bar cleanup review with live memory, CPU, GPU, and disk-space readings.
- Background disk scans and pointer-outside panel dismissal, including macOS 27 expanded-interface support.
- Stable Shelf.app bundle, Settings, protected paths, manual selection, and Quit Shelf.
- Manual Ask Agent analysis for configured Grok and Antigravity CLI, saved selection context, progress/cancellation, inline error recovery, and exact-session Terminal launchers.
- Optional read-only selected-folder callbacks; errors and analysis never change your selection.

**Confirm only records your choice. Shelf does not delete files or run cleanup.** There are no automatic paid agent requests.

### Install

Download the DMG, open it, and drag Shelf.app to Applications. The ZIP contains the same app. Verify downloads against SHA256SUMS.

Homebrew: `brew install --cask jellydn/tap/shelf`.

If Homebrew requires third-party cask trust, review the tap and run `brew trust --cask jellydn/tap/shelf` before retrying. This trusts only Shelf, not the whole tap.

These builds are ad-hoc signed, not Developer ID signed or notarized. macOS may require **System Settings → Privacy & Security → Open Anyway** after the first launch attempt. Intel builds are not included.

### Ask Agent limits

Only Grok and Antigravity CLI have in-app adapters; unsupported providers fail explicitly. Harmless stubs verify protocol handling, selection/privacy gates, read-only callbacks, failures, and saved-session launch arguments. Antigravity initialization/session creation was checked without a model prompt. Real model analysis, native folder-tool routing, and native chat continuation have not been verified end to end.

On the development Mac, Grok refuses its read-only sandbox because a runtime socket symlink resolves outside its deny list. Shelf reports this error instead of disabling the sandbox. Terminal continuation uses the CLI's own plan/sandbox controls, not Shelf's ACP read filter. Destructive work always needs separate approval.

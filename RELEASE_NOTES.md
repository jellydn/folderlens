## FolderLens 0.3.7 — Local summary without AI

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI stays optional and manual through the existing CLI adapters.

- Add **Local summary · no AI** to Agent help. It shows when no agent is chosen or agent-start permission is off, or when the last agent review failed, and the selection is not empty. It lists the item count and total, the top three kinds and the three largest folders by estimated size, and the estimate. Sizes are estimates, not reclaimed space.
- The summary is computed on this Mac from the current ticks. It makes no request, and it does not change ticks, estimates, protections or Record choice. A ready, running or successful agent review hides it.
- Keep "Choose an agent in Settings.", the error Details, Agent settings and Retry unchanged.
- Add the synthetic `no-agent-example` panel scene. Update the README and guide.

Verification covers the full macOS Rust/Pi suites, formatting, Clippy (no new warnings), a regression test for exact summary text with a locked tick and size order that differs from row order, visibility when an agent is ready or failed, unchanged ticks and estimates, and inspected 720- and 360-point renders of the no-agent and failed-review states. The default Review screenshots are unchanged because the summary is below the fold.

Native pointer/keyboard routing, physical-notch and multi-display behavior, real AI turns/tool routing/Terminal continuation and actual manual app replacement/restart/Gatekeeper behavior remain unverified. No owner's installed app is replaced during verification. All current installations remain manual-update-only; Homebrew users update through Homebrew, whose cask can lag publication. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.6 — System usage shown once

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI stays optional and manual through the existing CLI adapters.

- Show CPU, Disk and Memory usage in three cards at the top of Review, using one catalog snapshot. Disk shows used percentage and free space. CPU and Memory retain their General settings toggles.
- Remove the duplicate footer readings, GPU reading, GPU preference and background GPU sampling. Quit remains available in the footer. Existing settings still load; the retired GPU preference is ignored.
- Search later GitHub release pages during update discovery so a page full of prereleases cannot hide a stable update. Bound discovery to ten pages and report an unavailable check, rather than an incorrect up-to-date result, if that bound is reached.
- Allow ten seconds for the agent test stub to write a complete pid file under load, while keeping the two-second provider shutdown check unchanged.
- Refresh the README Review screenshot and description, and trim agent guidance to project-specific constraints.

Verification covers the full macOS Rust/Pi suites, formatting, Clippy, updater self-tests including second-page discovery and bounded pagination, CPU/Disk/Memory values and visibility preferences at 360–960 points, inspected Review renders, and ad-hoc arm64 DMG/ZIP packaging.

Native pointer/keyboard routing, physical-notch and multi-display behavior, real AI turns/tool routing/Terminal continuation and actual manual app replacement/restart/Gatekeeper behavior remain unverified. No owner's installed app is replaced during verification. All current installations remain manual-update-only; Homebrew users update through Homebrew, whose cask can lag publication. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.5 — Native Top center placement

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI stays optional and manual through the existing CLI adapters.

- Fix Top center on notched displays: use the selected screen's full frame and native camera safe-area height instead of always adding a panel row below the visible work area. Compact aligns its bottom with the camera row's bottom; expanded starts at the full display's top. This follows Oak's native anchor without reading Oak settings or detecting its windows. Overlap with another app on the center row is allowed; no collision avoidance is added.
- Keep menu-bar clearance on non-notched built-in and external displays, side positions and Lower center. Preserve saved display UUIDs, disconnect/reconnect fallback, all four positions, themes, compact/expanded sizes and collapse/tray fallback. Scan-only and manual-AI permissions are unchanged.
- Update Settings help, README, guide, architecture notes and current Settings screenshots. Explain that a physical camera can obscure Top center content; Lower center and side positions remain available. This is a placement fix, not a camera-cut-out content layout.

The camera-row regression failed before the fix and passes afterward. Verification covers the full macOS Rust/Pi suites, formatting, Clippy, compact/expanded geometry, safe-area height boundaries, offset and Dock-reduced screens, display selection/fallback and inspected standard/narrow/light Settings renders. A temporary in-process camera-metadata override exercises actual AppKit placement for all four positions and both sizes; programmatic resizing is not a native expand-button test. Actual Oak 0.5.55 uses the same compact bottom anchor under that override.

The verification Mac has one virtual display, not physical-notch or multi-display hardware. Camera occlusion, physical display selection/hot-plug, mixed display scales, native pointer/keyboard expansion, Spaces/full-screen behavior and VoiceOver remain unverified. The owner's actual screen telemetry was not available. Real AI turns/Terminal continuation, Finder dialogs and manual app replacement/restart/Gatekeeper behavior also remain unverified. No owner's installed app is replaced during verification. All current installations remain manual-update-only; Homebrew users update through Homebrew, whose six-hour cask check can lag publication. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.4 — Choose a notch display

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI stays optional and manual through the existing CLI adapters.

- Add Settings → General → Notch display: follow the primary display automatically or choose a named connected display. The choice belongs to FolderLens, with no Oak process/settings dependency.
- Save the display UUID independently of its name or list order. A disconnected choice falls back to primary without being forgotten, then returns when it reconnects. Settings explains the fallback; choosing Primary clears the saved choice. Old settings retain automatic primary placement. If macOS cannot provide a UUID, a session ID permits selection but may not survive reconnect/restart.
- Apply all four positions and compact/expanded work-area checks to the chosen display. Keep automatic collapse when expanded cannot fit and the tray fallback if compact cannot fit. Restore space-hidden notches when a fitting display returns, without reopening explicit dismissals. Preserve saved themes, position and agent permissions; display-only changes preserve Record choice.
- Refresh the README, guide and current General/theme screenshots; add connected/disconnected display examples. No cleanup, automatic AI or installer-policy change is added.

Verification covers the full macOS Rust/Pi suites, formatting, Clippy, UUID preference round trips and old-file defaults, same-name/reordered/disconnected/reconnected displays, offset and Dock-reduced screen geometry for both sizes and all positions, and accessible display controls at standard/minimum Settings sizes. Synthetic connected, missing-display, narrow and light-theme renders were inspected. A native isolated-HOME probe verifies compact bounds for explicit and missing display IDs on a single virtual display.

Actual selection across physical displays, physical unplug/reconnect, mixed display scales, native expansion/pointer/keyboard routing and VoiceOver remain unverified. Tests and synthetic renders are not multi-display hardware proof. Real AI turns/Terminal continuation, Finder dialogs and actual manual app replacement/restart/Gatekeeper behavior also remain unverified. No owner's installed app is replaced during verification. All current installations remain manual-update-only; Homebrew users update through Homebrew. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.3 — Shorter notch, saved themes and verified update downloads

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI stays optional and manual through the existing CLI adapters.

- Reduce the compact notch from 300×36 to 300×32 points. Keep the metric fonts, 22×28 expand control, 480×230 expanded panel and four independent screen positions.
- Add seven saved themes in Settings → General: Default, Kanagawa, Kanagawa Lotus, Dracula, Alucard (Light), Tokyo Night and Tokyo Night Day. Apply the app-local palette across the notch, Review, actions and Settings, without reading Oak settings. Missing or unknown themes use Default; existing placement and permissions remain unchanged. Theme-only changes preserve Record choice. Settings switches without a color fade; other windows reload on the next preference poll.
- Complete the manual update route: Check for Updates → Download and verify → Reveal in Finder. Keep pinned publisher metadata and ZIP size/hash checks, quarantine, channel selection, Homebrew guidance and the existing installation-ownership gate. Final app replacement is manual; the app does not replace itself, restart itself or install on quit. Preferences and chats remain separate.
- Update the README, guide and current notch, General, theme and Updates screenshots. No automatic AI, paid fallback or cleanup path is added.

Verification covers the full macOS Rust/Pi suites, formatting, Clippy, preference migration and theme round trips, recorded-choice preservation, compact metric/expand bounds, keyboard-accessible theme choices at standard/minimum Settings sizes, solid-palette text contrast, signed updater discovery/integrity/ownership and rollback fixtures, and ad-hoc arm64 DMG/ZIP packaging. All seven palettes have headless renders for compact/expanded notch, Review and Settings; representative light and running/scanning states were inspected.

Native pointer/keyboard routing, VoiceOver, physical-notch/multi-display behavior, live light/dark interaction contrast, Finder dialog interaction, real AI turns/Terminal continuation and actual manual app replacement/restart/Gatekeeper behavior remain unverified. Headless checks are not native click proof; no owner's installed app is replaced during verification. All current installations remain manual-update-only. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.2 — Screen position choices

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI stays optional and manual through the existing CLI adapters.

- Add Settings → General → Notch placement choices: Top left, Top center, Top right and Lower center. These positions are independent of Oak or any other app: no app detection, assumed window widths, reserved center slot or automatic collision handling.
- Anchor edge positions 8 points from the visible work area's side. Center positions use the full primary display's center, with origins clamped to the work area. Both compact and expanded sizes reposition after expansion or display changes. Collapse if the expanded panel cannot fit; hide the notch and keep the tray fallback if compact cannot fit.
- Preserve the existing default Lower center and its 41-point top inset. Old `notch_below=1` settings migrate to Lower center, `0` to Top center; an explicit valid `notch_position` wins regardless of legacy-key order. Unknown values retain the legacy choice or default. Existing agent settings and permissions are unchanged.
- Keep the 300×36 compact readings, visible Quit, scan-only intervals and manual agent adapters. Update README, guide and the General screenshot. No deletion path, direct AI API or automatic paid request is added.

Verification covers the full macOS Rust/Pi suites, migration/round-trip cases, all four Settings choices at 760×560 and 360×420, compact/expanded geometry on offset, Dock-reduced, exact-fit and undersized work areas, formatting, Clippy, updater fixtures and ad-hoc arm64 DMG/ZIP packaging. Actual compact Top left and Top right shared a row with Oak 0.5.55 without overlap on the verification display; Top center reproduced overlap and restoring Top right cleared it without restarting. This does not promise non-overlap with arbitrary windows.

Native pointer/keyboard routing, automatic collapse/tray fallback during live display shrinkage, native expansion, physical-notch and multi-display hardware, real AI turns/Terminal continuation and actual installation/restart/Gatekeeper behavior remain unverified. Screen recording and event posting were unavailable for coexistence checks. All current installations remain manual-update-only. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.1 — Smaller notch, Oak coexistence and visible Quit

**Stable patch for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** AI remains optional and manual through the existing CLI adapters.

- Reduce the compact notch from 360×36 to 300×36 points while keeping candidate estimates, disk free/total and CPU/memory readings. Tighten spacing and the expand button; use TB for large compact disk readings.
- Leave Oak's 33-point top row plus an 8-point gap clear by default, including with existing settings. Expansion grows downwards. General → Leave top row for Oak can restore the top row when running alone. This is fixed placement, not app detection or automatic collision handling for arbitrary windows.
- Add visible Quit controls to the expanded notch and Review's fixed readings footer, accessible through the tray fallback. Actions → Quit FolderLens and Command-Q remain available. The native quit hook stops interval scheduling, disconnects scan requests and drops the manual agent job/reader, waiting up to two seconds for provider kill/reap completion. In-process scan/gauge workers end with the app, not a background service. Close and Use tray still only hide windows.
- Update the README, guide and current screenshots. No deletion path, automatic AI request, direct AI API or paid fallback is added.

Checks cover the full macOS Rust/Pi suites, formatting, Clippy, compact/expanded and minimum-size UI bounds, both visible Quit actions, timer/request-channel shutdown, updater fixtures, ad-hoc bundle signatures, arm64 architecture and DMG/ZIP packaging. The overlap regression failed against 0.3.0 geometry. Actual Oak 0.5.55 and FolderLens ran together with an 8-point gap; the top-row opt-out reproduced overlap, and restoring the setting removed it without restarting.

Native pointer/keyboard routing, physical-notch and multi-display behavior, real AI turns/Terminal continuation and actual install/restart/Gatekeeper behavior remain unverified. Native screen recording and event posting are unavailable on the verification Mac; headless button/layout tests are not native click proof. All current installations remain manual-update-only. Builds are **ad-hoc signed, not Developer ID signed or notarized**.

## FolderLens 0.3.0 — Notch and local checks

**Stable release for Apple Silicon, macOS 14+. Record choice never deletes files or runs cleanup.** Candidate sizes are review estimates, not reclaimed space.

- Start with an Oak-style, single-line 360×36 notch below the primary display's camera/menu-bar area. Folder, disk, CPU and memory icons accompany candidate estimates, free/total disk space and live usage. Expand for controls; the menu-bar archivebox remains a fallback to full Review. CPU/memory visibility follows General settings.
- Run local, non-AI scans every 30 minutes or 1 hour, or use Scan now. Intervals start paused after launch, never overlap or replay missed scans, and wait a full interval after completion. Pause stops future scans while an active scan finishes. Refresh remeasures nested changes, preserves explicit choices by folder identity, reapplies exclusions and live reader scope, and clears recorded choices. Failed scans retain previous results.
- Keep AI optional and manual through existing CLI adapters. No direct AI API, automatic model call, paid fallback, cleanup or deletion path is added. Reader/session and publisher-update safety contracts remain unchanged.
- Cache the pinned Rust toolchain and locked dependencies in fresh Amp orbs; resume performs no installation. Native verification still requires a Mac satellite.

Verification covers the macOS Rust suite, Pi reader regression, formatting, Clippy, synthetic compact/expanded renders, updater fixtures, arm64 bundle signatures and DMG/ZIP packaging. Release publication independently verifies publisher signatures; downloaded assets must match their published digests.

Native pointer/keyboard routing, physical-notch and multi-display behavior, tooltips, real model/tool turns, native Terminal continuation, and actual install/restart/Gatekeeper behavior remain unverified. Headless layout/callback checks do not establish these flows or resolve the earlier Settings→tray report. Builds are **ad-hoc signed, not Developer ID signed or notarized**. All installations remain manual-update-only; Homebrew users update through Homebrew.

## FolderLens 0.2.1 — First FolderLens release

**Stable release for Apple Silicon, macOS 14+.** The DMG and ZIP contain `FolderLens.app` with bundle ID `io.github.jellydn.folderlens`. Shelf is now named FolderLens in the app, agent reader contracts and release assets. Published 0.2.0 and earlier releases remain Shelf; their names and assets below are unchanged. **Record choice never deletes files or runs cleanup.**

- Import legacy Shelf settings once when FolderLens settings are absent, preserving exclusions, recent-change holds, provider/model choices and permissions. Existing FolderLens settings win; originals and saved conversations are not removed. Folder sizes are remeasured.
- Ship the existing unlettered icon under the new asset names. Align Pi's reader environment, tool and helper names with its provider, and align publisher signing with FolderLens release filenames. Keep the existing publisher key.
- The first rename upgrade is manual: the shipped Shelf updater cannot accept FolderLens-named archives. See [migration instructions](https://github.com/jellydn/folderlens/blob/v0.2.1/docs/guide.md#migrating-from-shelf) for direct-download and Homebrew users. After the tap updates, `brew update` and `brew upgrade --cask jellydn/tap/folderlens` install `FolderLens.app`. Back up settings/chats; do not use `--zap`. Direct-download users can install alongside Shelf and remove the old app themselves after checking migration.
- The new bundle ID may need a new macOS Full Disk Access grant. Old Shelf launchers retain their original executable and controls. No cleanup/delete path, automatic replacement or Gatekeeper bypass is added.

Release checks cover formatting, Clippy, the full macOS Rust suite, the Pi reader regression, updater self-tests, arm64 bundle signatures and DMG/ZIP packaging. Published metadata, ZIP and DMG must verify with the unchanged Ed25519 publisher key; re-download and check `SHA256SUMS` and GitHub asset digests.

Existing-user migration, native update replacement/restart, Gatekeeper approval, real model/tool turns and native Terminal continuation remain unverified. An isolated cask installation checks package placement, not these flows. Headless renders, harmless provider stubs and updater fixtures do not prove them. Builds remain **ad-hoc signed, not Developer ID signed or notarized**. All installations remain manual-update-only. The known Settings→tray report is not fixed by this rename.

## Shelf 0.2.0 — Quick review

**Stable release for Apple Silicon, macOS 14+.** Review folder selections, reasons and exclusions from the menu bar. **Record choice saves a selection in memory; it never deletes files or runs cleanup.** Sizes are estimates, not reclaimed space.

- Quick review has compact and wide layouts, visible protection reasons, optional Agent help, fixed recording/system readings and explicit Close controls. Main actions, secondary actions and warnings have distinct button styles, with keyboard focus and Space activation.
- Manual Ask Agent supports per-provider model settings, explicit start/path-sharing permissions, a bounded selected-folder reader, progress/Cancel, errors/Retry and success-only saved chat. Provider costs and data policies apply; no provider starts automatically.
- Agent protocol lines are bounded to 2 MiB before parsing, the message queue holds at most eight messages, and diagnostic allocation/log budgets are bounded. Regression checks cover oversized unterminated output and blank-line flooding. Completed reports retain their 256 KiB cap.
- All seven codebase map documents were regenerated from current source on 2026-11-09. The README and guide now describe stable 0.2.0 and its limits.
- Daily publisher-verified update checks and opt-in downloads remain available. All current installs are **manual-update-only**; Homebrew installs use Homebrew. There is no automatic replacement or install-on-quit.

### Verification and known limits

Release checks cover formatting, Clippy, the full macOS Rust suite, native updater self-tests, packaging, arm64 architecture and code signatures. Dedicated Shelf Ed25519 signatures protect the metadata and archives. Published assets must also be downloaded again and verified against `SHA256SUMS`, GitHub asset digests and the committed publisher public key.

**Settings→tray reopening remains an unresolved native report.** Its cause and a native fix are not verified. Headless layouts and callback tests do not establish full native tray behavior. Real model/tool turns, native Terminal continuation and actual install/restart/Gatekeeper behavior remain unverified. Provider controls are not an OS sandbox. No real model request, installed-app replacement or cleanup is part of release verification.

Builds are **ad-hoc signed, not Developer ID signed or notarized**. Use normal macOS approval if required; Shelf does not bypass Gatekeeper. The stable label does not remove these limits.

### Manual test checklist

1. Test initial tray-open, Settings→tray, outside clicks, Close, Escape and tray toggle separately. Report failed steps and macOS version.
2. Choose eligible folders and Record choice. Nothing is deleted. Change the selection: the record clears. Protected/in-use folders stay excluded.
3. Check compact/wide layouts, keyboard focus/Space, model Apply and explicit request/path-sharing permissions. No agent starts automatically.
4. Only with separate provider cost/data consent, test analysis, Cancel, errors/Retry and saved-chat continuation. Stubs do not establish native model-turn coverage.

## Shelf 0.2.0-beta.6 — Fresh source map and bounded agent transport

**Experimental prerelease, not latest. Record choice saves a selection in memory; it never deletes files or runs cleanup.** This release uses current main after PR #13 merged. Stable/Homebrew and earlier release assets are unchanged.

- Rebuilt all seven `.planning/codebase/` documents from current source, dated 2026-11-09. The map covers current Quick review, provider/session boundaries, native updates, tests and evidence-backed concerns. It no longer describes outside clicks as a dismissal control.
- Fixed unbounded agent protocol-line allocation and message buffering. Lines now fail above 2 MiB before JSON parsing, even without a newline; the queue holds at most eight messages. A harmless CLI regression failed before the fix and passed afterward. The completed-report cap stays at 256 KiB; failures save no completed report or session.
- Bounded diagnostic-line allocation before redaction. Truncated lines are discarded, remaining diagnostics drain without growing memory, and newline bytes count toward the 16 KiB log budget. Blank-line flooding is covered by the failure/retry regression.
- Includes the button alignment, action priorities, keyboard recording and Rune revision already on main after beta.5. These are not changes to the selected-folder reader, model/cost consent or non-deleting record behavior.

### Verification and limits

Release checks cover formatting, Clippy, the full macOS Rust suite, native updater self-tests, packaging, architecture and code signatures. Publication requires dedicated Shelf Ed25519 signatures; published assets must also be downloaded again and checked against `SHA256SUMS` and the committed publisher public key.

**Settings→tray reopening remains an unresolved native report.** Its cause is not established; this release does not claim a fix. Headless layouts and callback tests are not complete native tray proof. Real model/tool turns, native Terminal continuation and actual install/restart/Gatekeeper behavior remain unverified. No paid model prompt, installed-app replacement or cleanup is part of release verification.

All current installs remain **manual-update-only**. Publisher-verified checks and opt-in downloads remain available; Homebrew installs use Homebrew. Apple Silicon, macOS 14+. Builds are **ad-hoc signed, not Developer ID signed or notarized**. Use normal macOS approval if required; Shelf does not bypass Gatekeeper.

### Manual test checklist

1. Test tray initial-open, Settings→tray, outside clicks, Close, Escape and tray toggle separately. Report failed steps and macOS version; do not infer a Settings-specific cause from a failed initial-open control.
2. Choose eligible folders and Record choice. No files are deleted. Change the selection: the recorded indicator clears. Protected/in-use folders stay excluded.
3. Check compact/wide review and Settings, keyboard focus/Space, model Apply and request/path-sharing permissions. No agent starts automatically.
4. Only with separate provider cost/data consent, request analysis and test Cancel, errors/Retry and successful saved-chat continuation. Protocol stubs do not establish native model-turn coverage.

[Levo](https://omggg.wtf/levo/) was the direct reference for Shelf's original presentation. Shelf is unaffiliated and not endorsed. Retained history and attribution do not establish cleanup safety. Earlier release notes follow unchanged.

## Shelf 0.2.0-beta.5 — Persistent Quick review

**Experimental prerelease, not latest. Record choice saves a selection in memory; it never deletes files or runs cleanup.** PR #13 remains unmerged. Stable/Homebrew and earlier releases are unchanged.

- Outside clicks and transient expanded-interface session endings no longer request panel dismissal. Close, Escape and the tray toggle remain explicit dismissal controls; Escape closes Actions first.
- Starting a new tray session toggles a still-visible panel instead of reopening it. Wrapped exclusion reasons have content-sized rows at the compact/wide boundary. Both regression checks failed before their fixes and pass afterward; native event routing remains unverified.
- The update helper orders out its alert and returns focus to its presenter without reopening explicitly closed windows. Policy and native-callback regressions pass; the complete native popup flow remains unverified.
- Cleaner Quick review and Settings: shorter labels, less repeated copy, explicit Close controls and provider-specific model guidance under advanced setup. Model Apply, permissions, protection, progress/Cancel, errors/Retry, run files and success-only saved chat remain available.
- Current headless screenshots cover the features in the concise README and linked guide. Agent progress, errors and saved-chat images are fixtures, not real model turns.

### Manual test checklist

1. Open Shelf, then click another app. Shelf should stay open. Close an update notice: Shelf should stay visible and usable. Test Close, Escape and the tray toggle separately; Escape closes Actions first. Do not install an update to test its notice.
2. Open Settings and close it, then click the tray. **Settings→tray reopening is a separate unresolved issue**; report exact steps and macOS version if it fails. This release does not claim to fix it.
3. Resize compact/wide, scroll and use keyboard Actions. Record choice and readings should stay reachable. Record an eligible selection, then change it: the recorded indicator clears and no files are deleted.
4. Check Settings → Agent / Access: your provider/model, Apply, advanced setup, request and selected-path permissions. Switches save immediately; model edits need Apply. No request starts automatically.
5. Only if you accept your provider's costs/data policy, request analysis and check progress/Cancel, error/details/Retry, run files and successful saved-chat continuation. No real model/tool/Terminal turn was verified for this release.

Apple Silicon, macOS 14+. Verify `SHA256SUMS` and publisher signatures. DMG/ZIP contain an **ad-hoc signed app, not Developer ID signed or notarized**; normal macOS approval may be required. All current installs remain manual-update-only. Real install/restart/Gatekeeper behavior is unverified. No installed app, preferences, model account or cleanup was changed during verification.

[Levo](https://omggg.wtf/levo/) was the direct reference for Shelf's original presentation. Please support Levo. Shelf is unaffiliated and not endorsed; retained attribution/history do not establish equivalent cleanup safety. Earlier release notes follow.

## Shelf 0.2.0-beta.4 — Quick review preview

**Experimental WIP, prerelease/not latest. Record choice saves a selection in memory; it never deletes files or runs cleanup.** Estimates are not reclaimed space. PR #13 remains unmerged; the stable Homebrew tap and earlier releases are unchanged.

- Quick review: simple folder reasons, optional Agent help, fixed recording/readings, compact/wide layouts and a 280-point minimum height.
- Settings: model and request/path-sharing permissions upfront, fixed Apply model, advanced commands on demand, progress/cancel/error/retry and success-only saved-chat discovery retained.
- Group actions show current exclusions. Provider/model support, bounded reader, protection rules and fail-closed/manual-only update policy remain unchanged.

**Known issue:** Settings can leave the tray unable to reopen the panel. Native testing produced mixed initial-open controls; no cause or fix is claimed. Real native model/tool/Terminal turns and installation/restart remain unverified. Provider costs/data policies apply; controls are not an OS sandbox.

### Manual test checklist

1. Open Shelf from the tray, then Settings → tray. If reopening fails, report the steps and macOS version; this is a known issue, not a passing check.
2. Resize compact/wide and scroll. Record choice and system readings should remain visible; Actions supports keyboard navigation.
3. In Agent settings, choose your configured provider/model and Apply. Check that the model persists, invalid flags show an error, and nothing starts automatically. Keep path sharing off unless you choose otherwise.
4. Select eligible folders and Record choice. Nothing is deleted. Change the selection: the recorded indicator must clear. Protected folders stay excluded.
5. Inspect Agent setup and advanced controls. Only if you accept the provider's costs/data policy, manually request analysis and check progress/Cancel, error/details/Retry, run files and success-only saved chat. Do not treat a model answer as cleanup approval.

### Download, credit and limits

Apple Silicon, macOS 14+. DMG/ZIP contain an ad-hoc signed app, **not Developer ID signed or notarized**. Verify `SHA256SUMS`; metadata and both archives carry signatures checked against Shelf's pinned Ed25519 publisher key. Normal macOS approval may be required; no Gatekeeper bypass is provided.

[Levo](https://omggg.wtf/levo/) was the direct reference for Shelf's original UI/layout/interactions. Please support Levo through its official site. Shelf is not affiliated with or endorsed by Levo; attribution does not resolve the earlier presentation concern or establish equivalent cleanup safety. Historical assets and release notes remain below.

## Shelf 0.2.0-beta.3 — Review ledger preview

**Experimental WIP PoC. Record choice (formerly Confirm) records a selection in memory; it does not delete files or run cleanup.** Footprints are estimates, not reclaimed space or proof of cleanup safety.

- First independent Review ledger implementation from PR #13: reasons/exclusions beside a non-deleting receipt, compact system readings, responsive stacking and clearer folder actions/Protected/Access settings. Design review is still in progress; this prerelease does not merge the PR or declare the presentation review complete.
- Short README, with detailed agent/update guidance and historical screenshots retained in `docs/guide.md`.
- Review follow-ups: resized Actions scroll to the filter and reveal keyboard selections; single-folder actions use the current selection reason; exclusion settings explain that removing a rule can restore preset/manual selection without deletion.
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

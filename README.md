# FolderLens

<img src="assets/folderlens-icon.png" width="96" height="96" alt="FolderLens app icon">

A macOS menu-bar app for reviewing folder selections, reasons and exclusions, with optional agent analysis.

**Record choice records a selection in memory. It never deletes files or runs cleanup.** Sizes are estimates, not reclaimed space.

FolderLens is the new name for Shelf. The published **0.2.0** release is still Shelf; the rename is unreleased. Quick review, folder protections, manual agent help and publisher-verified update checks remain available. The [codebase map](.planning/codebase/ARCHITECTURE.md) documents the pre-rename source.

![Quick review preview with synthetic example data](shots/review-ledger.png)

## Install

Apple Silicon, macOS 14+. Get the Shelf-named DMG or ZIP containing `Shelf.app` from the [0.2.0 stable release](https://github.com/jellydn/folderlens/releases/tag/v0.2.0) and check `SHA256SUMS`. Builds are ad-hoc signed, **not notarized**; macOS may require **Privacy & Security → Open Anyway**. Publisher signatures are not Apple approval.

Or install the stable release through Homebrew. The renamed `folderlens` cask currently installs `Shelf.app` from 0.2.0. The cask checks for new stable versions every six hours, so it can lag behind a new release.

```sh
brew install --cask jellydn/tap/folderlens
```

For Homebrew trust or source builds, see the [guide](docs/guide.md#install). Existing Shelf users must follow the [migration guide](docs/guide.md#migrating-from-shelf); the old updater cannot accept FolderLens-named releases.

## Use

1. Click the menu-bar archivebox. Choose a preset or eligible folders; review reasons and exclusions.
2. **Record choice** records without deleting. Selection changes clear the record.
3. For **Ask Agent**, configure **Settings → Agent / Access**. Requests are manual; provider costs and data policies apply.

**Actions** opens folder controls and Settings. The panel stays open across outside clicks and update dialogs. Use **Close**, Escape or the menu-bar button to hide it; Escape closes Actions first.

## Feature gallery

These current Rune renders use example Agent states, not real model turns or native popup verification. [Full gallery](docs/guide.md#feature-gallery-beta5) covers review/recording, folder actions, General, provider/model Apply, selected-path access, protection, Agent progress/Cancel, errors/Retry, saved chat and Updates. Filled buttons mark the main action; secondary actions are outlined, and removing an exclusion uses a warning style. Record choice never deletes.

| Agent setup and model | Updates and manual-install policy |
| --- | --- |
| ![Provider, model and explicit permissions](shots/quick-agent-settings.png) | ![Update controls and signing limits](shots/quick-updates.png) |

## Limits

- Folder sharing is opt-in and selected-only. Context may leave your Mac and persists locally. Provider controls are not an OS sandbox.
- All current installs are **manual-update-only**; verified checks/downloads remain available. Homebrew users update through Homebrew.
- Tests/headless renders are not native end-to-end proof. Tray routing, model/tool turns, Terminal continuation and real install/restart/Gatekeeper behavior remain unverified. Cleanup safety is not established.
- Settings→tray reopening remains a separate known issue; the popup changes do not establish a fix for it.

## Details

[Full guide and historical screenshots](docs/guide.md) · [Agent boundary](docs/adr/0001-scoped-agent-sessions.md) · [Update policy](docs/adr/0002-publisher-verified-updates.md) · [Release notes](RELEASE_NOTES.md)

## Build and test

```sh
sh scripts/bundle.sh
cargo test --locked
```

Built with Rust and Rune. [MIT license](LICENSE).

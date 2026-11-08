# Shelf

<img src="assets/shelf-icon.png" width="96" height="96" alt="Shelf app icon">

A macOS menu-bar prototype for reviewing folder selections, reasons and exclusions, with optional agent analysis.

**Experimental WIP. Record choice (formerly Confirm) records a selection in memory. It never deletes files or runs cleanup.** Sizes are estimates, not reclaimed space. [PR #13](https://github.com/jellydn/shelf/pull/13)'s Review ledger is a first independent presentation, still under review.

Quick review below is an unreleased preview, not included in beta.3.

![Unreleased Quick review with synthetic example data](shots/review-ledger.png)

## Install

Apple Silicon, macOS 14+. Download the [beta.3 DMG or ZIP](https://github.com/jellydn/shelf/releases/tag/v0.2.0-beta.3) and check `SHA256SUMS`. Builds are ad-hoc signed, **not notarized**; macOS may require **Privacy & Security → Open Anyway**. Publisher signatures are not Apple approval.

For the older stable version, not this beta:

```sh
brew install --cask jellydn/tap/shelf
```

For Homebrew trust or source builds, see the [guide](docs/guide.md#install).

## Use

1. Click the menu-bar archivebox. Choose a preset or eligible folders; review reasons and exclusions.
2. **Record choice** records without deleting. Selection changes clear the record.
3. For **Ask Agent**, configure **Settings → Agent / Access**. Requests are manual; provider costs and data policies apply.

**Actions** opens folder controls and Settings. Outside click, Escape or a second menu-bar click closes the panel; pointer exit does not.

## Limits

- Folder sharing is opt-in and selected-only. Context may leave your Mac and persists locally. Provider controls are not an OS sandbox.
- All current installs are **manual-update-only**; verified checks/downloads remain available. Homebrew users update through Homebrew.
- Tests/headless renders are not native end-to-end proof. Tray routing, model/tool turns, Terminal continuation and real install/restart/Gatekeeper behavior remain unverified. Cleanup safety is not established.

## Credit and details

[Levo](https://omggg.wtf/levo/) was the direct UI/layout/behavior reference for beta.2 and earlier. Please support or buy Levo. Shelf is unaffiliated and not endorsed; credit does not resolve the earlier copied-presentation concern or establish equivalent safety.

[Full guide and historical screenshots](docs/guide.md) · [Agent boundary](docs/adr/0001-scoped-agent-sessions.md) · [Update policy](docs/adr/0002-publisher-verified-updates.md) · [Release notes](RELEASE_NOTES.md)

## Build and test

```sh
sh scripts/bundle.sh
cargo test --locked
```

Built with Rust and Rune. [MIT license](LICENSE).

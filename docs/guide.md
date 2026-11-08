# Shelf prototype guide

Detailed implementation, usage and verification notes. Start with the [short README](../README.md).

<img src="../assets/shelf-icon.png" width="128" height="128" alt="Shelf: a folder in an archive tray on a shelf">

> Experimental WIP proof of concept for reviewing cleanup selections. Record choice (formerly Confirm) records a choice; it does not delete files or run cleanup.

## Current status and Levo credit

[Levo](https://omggg.wtf/levo/) was the direct reference for Shelf's original UI, layout and interaction behavior, not just a general inspiration. That presentation remains in the published beta.2 and earlier releases. If you like this idea, please support Levo and consider buying it through its official site. Shelf is not affiliated with or endorsed by Levo.

Shelf is an experimental **WIP PoC**, not a finished cleanup utility or a claim of Levo-equivalent cleanup safety. Careful, Balanced and Aggressive presets preview which items are selected and their estimated footprint. **Record choice only records a choice in memory; Shelf does not delete the selected files.** Changing the selection or its exclusions clears the recorded indicator. An empty selection cannot be recorded.

The code in [PR #10](https://github.com/jellydn/shelf/pull/10) implements manual Ask Agent adapters, per-agent model settings, a bounded selected-folder reader, saved-session launchers and signed-update checks. Protocol stubs, headless UI tests and limited native metadata/download checks exercise these implementations. They do **not** establish end-to-end native model analysis, tool routing during a model turn, Terminal continuation, tray event routing, or actual update installation/restart and Gatekeeper behavior. The sections below describe this PR's code, which includes follow-ups not present in the published beta.1 assets.

**Beta.3 previews the owner's chosen Review ledger direction**, with reasons and exclusions beside a non-deleting selection receipt and separate agent access. This first implementation is still under design review; beta.2 does not contain it. This one beta release does not authorize a merge, promotion or later releases. Attribution alone does not resolve the concern about the copied presentation. Earlier screenshots, releases and history are retained as provenance, not evidence of completed design or safety review.

## Review ledger (beta.3 preview)

The panel opens from the menu bar. Moving the pointer outside does not close it. Click outside, press Escape, or click the menu-bar button again to dismiss it. Escape closes an open action list before closing the panel.

The wide ledger compares candidates with a selection receipt. Below 620 points it stacks these sections in one scrollable column. Live system readings stay in the footer instead of leading the decision. These panel examples use synthetic data and make no scan, model request or file change.

![Review ledger, synthetic example](../shots/review-ledger.png)

![Recorded choice, no deletion](../shots/review-receipt.png)

Folder actions show the target and its reason. Ask Agent still uses the eligible selection, not the action target. Protected settings explain user exclusions, recent-change holds and built-in exclusions separately; their rules are unchanged.

![Folder and review actions](../shots/review-actions.png)

![Exclusion rules, example retained folder](../shots/exclusion-rules.png)

Agents on `PATH` and model controls remain in Settings.

![Agent settings](../shots/review-agent-settings.png)

Historical presentation: [original panel](../shots/tray.png), [original Protected settings](../shots/settings-protected.png), [earlier Agent settings](../shots/settings-agent.png). These files remain unchanged.

## Brand assets

The bundled app uses [Shelf.icns](../assets/Shelf.icns), generated from the Painter-designed [app icon](../assets/shelf-icon.png). The matching small-size [favicon](../assets/favicon.svg) uses the same teal, amber and cream shapes. [ICO](../assets/favicon.ico), [PNG](../assets/favicon.png), Apple touch/PWA icons and [web manifest](../assets/site.webmanifest) are in `assets/` for documentation sites; GitHub controls its own browser favicon. Regenerate with `uv run --with pillow scripts/icons.py` on macOS. Normal builds copy the committed ICNS and need no image-generation tools.

## Implemented prototype features

- Menu-bar review ledger with compact live memory, CPU, GPU and free-space readings
- Candidate selection reasons, exclusions and an estimated-footprint receipt; no reclaimed-space claim
- Careful, Balanced, and Aggressive presets, plus a Selected override
- Record choice keeps an in-memory choice, not a cleanup instruction
- Actions for the row under the pointer: Finder, keep, unlock, and the rest
- Protected folders you add, a “keep what changed within” hold, and built-in locked paths
- AI tools detected on `PATH`, from the same command list as [ai-launcher](https://github.com/jellydn/ai-launcher)
- Ask Agent sends the current ticked selection, sizes, and cleanup request to the tool chosen in Settings, only after you grant access. Folder paths are sent only when path sharing is enabled; destructive work needs approval.
- Settings and the panel are never open together

## Ask Agent (experimental)

Each request creates a private folder under `~/Library/Application Support/Shelf/Runs/` with `Ask.command` and `context.md`. Earlier runs are kept, not overwritten. The context records the eligible ticked items at the time of the request; later selection changes do not change that snapshot.

Ask Agent runs the configured CLI on a background thread. Grok, Antigravity, GitHub Copilot CLI and OpenCode use ACP; Pi uses RPC. Other agents show an unsupported-provider message; Shelf does not switch agents or guess their flags. The panel shows elapsed progress and Cancel, then the review or an inline error. Details shows redacted diagnostics, with Agent settings and Retry. Errors do not clear scan results or change ticks.

In **Settings → Agent**, enter a model ID and select **Apply**. Shelf remembers a separate choice for each agent (and each custom executable). Switching agents restores that agent's saved model. Grok, Antigravity, Copilot and Pi receive provider-supported `--model` arguments. OpenCode receives private per-run configuration and an ACP model selection, not a nonexistent top-level model flag. Shelf checks advertised model access before prompting; an unavailable explicit model fails instead of silently choosing another model. Blank keeps the CLI default, except Copilot uses `auto` and OpenCode requires an explicit model.

| Agent | Interface and model example | Access and cost limits |
| --- | --- | --- |
| Grok / Antigravity | ACP plan mode; optional model ID | Your existing CLI login, default model, plan and sandbox apply. |
| GitHub Copilot CLI | ACP plan mode; `auto` | [Copilot Free](https://docs.github.com/en/copilot/concepts/billing/usage-based-billing-for-individuals) has a limited allowance and Auto-only model access. Explicit models depend on your plan and credits; Auto is not a promise of free use on a paid account. |
| Pi | RPC; `openrouter/openrouter/free` | Pi provides no inference entitlement. [OpenRouter Free](https://openrouter.ai/openrouter/free) needs your account/key; rate limits, routing and each model's data policy apply. Blank may use a paid CLI default. |
| OpenCode | ACP, verified configuration schema **2.0.23 only**; `opencode/glm-5.3-flash-free` or `opencode/deepseek-v4.1-flash-free` | [OpenCode's free models](https://opencode.ai/docs/zen) are temporary and account/availability limits apply. Privacy terms differ by model. Shelf pins the chosen model for title/compaction as well and does not select a paid fallback. Other CLI versions fail closed until their configuration is verified. |

No shared provider configuration is changed. Use the provider's own login/key setup; do not enter credentials in Shelf's model field. Sending context to an agent can send it to an external model provider, including when the model is free. Check your provider's data policy and budget before starting a review.

With path sharing off, Shelf sends only the saved selection context. With sharing on, Shelf permits bounded text reads and directory listings for eligible selected folders. Its reader rejects writes, commands, unselected paths, protected/kept/held/in-use paths, and symlink escapes. Reads are limited to 512 KiB of UTF-8 text, 2,000 lines, or 1,024 directory entries. Selection or permission changes can only reduce the original read scope.

Copilot and OpenCode use Shelf's private MCP reader; Pi loads only the explicit Shelf reader extension, with native tools and discovered extensions disabled. These adapters deny native ACP read/permission requests so they cannot bypass the broker. OpenCode waits for its private read-only mode to finish loading before selecting a model or sending a prompt. The private reader permits only original ∩ current eligible selection. Its socket closes on cancellation, failure, replacement or app exit. Saved chat can continue without Shelf, but these folder tools fail closed; reopening Shelf does not restore old grants. Grok/Antigravity analysis uses the same bounded reader through ACP callbacks; their Terminal continuation still relies on their native plan/sandbox controls. Provider controls and callbacks are not an independent operating-system sandbox.

After a successful review, Shelf saves `analysis.md` and `session.txt`. Terminal opens `Ask.command` to resume that exact provider session in the same run folder, with the same model and tool restrictions. Pi must finish its settled event and save its exact chat under the run's private `sessions/` directory. Before success, the launcher refuses to open an unrelated fresh chat. Shelf rechecks the agent, model and start permission before opening Terminal. The saved context is a snapshot, not a live selection.

OpenCode continuations recheck the verified CLI version even when Shelf is closed. Pi blocks continuation prompts if the restored model differs from an explicit saved choice. Cancellation and successful review finalization have one ordering point: cancellation that wins prevents a successful report/session launcher from being saved.

Agent-start permission still applies. Requests remain manual; there are no automatic paid calls. Destructive work requires separate approval, and Shelf itself never deletes files. Run files can contain shared paths and remain on disk until you remove them. On this Mac, the installed Grok read-only sandbox currently refuses startup because of a socket-symlink deny-list error; Shelf reports it rather than disabling the sandbox. Harmless native metadata checks verified Copilot's plan/Auto selection, Pi's model state, and OpenCode's private-mode loading and exact model selection without sending a model prompt. Live model analysis, native tool routing during a model turn, and native Terminal continuation have not been verified; protocol stubs cover the handoff and failures.

## Updates

Settings → Updates checks GitHub releases daily while the bundled app runs. Stable releases are the default; **Include prereleases** is opt-in. **Download automatically** is off by default and requires daily checks to be enabled. Manual checks remain available. Updates send only an app/version request to GitHub, never scan results, selected paths, saved chats or agent credentials.

Shelf verifies signed `update.json` metadata against its pinned, dedicated Ed25519 public key, then verifies the ZIP's signed size and SHA-256 before extraction. Expired metadata, missing/wrong signatures, wrong channel/version, corrupted downloads and unverified bundles fail closed. Every **Install and Restart** action requires a separate native confirmation; a download or quitting Shelf never authorizes installation. Finish agent reviews first. A previous app copy is kept next to the installation for rollback. App settings and chats are not removed.

Detected Homebrew installations are not self-replaced: use `brew upgrade --cask jellydn/tap/shelf`. The stable tap does not follow prereleases. Source/debug binaries do not run an updater; use the bundled app.

**Unknown installation origin is manual-update-only.** A missing cask receipt, a writable Applications path or a copied bundle marker is not proof of a direct installation. This also blocks native replacement for custom-prefix casks launched by Finder without `HOMEBREW_PREFIX`, moved cask apps and aliases. Current packages have no reliable direct-install provenance, so **native Install and Restart is unavailable for all current installations**, even with user approval. Only a future positively established eligible direct installation may reach the separate confirmation and tested installer; no preference bypass exists.

Publisher-verified checks and opt-in downloads still work. After an automatic download, use **Check for Updates…** for manual-install guidance; verified ZIPs are in `~/Library/Application Support/Shelf/Updates`. If Homebrew installed Shelf, update through Homebrew rather than replacing the app manually. Otherwise install the verified ZIP or release DMG yourself, subject to normal macOS approval. Discovery currently examines the first 100 releases. There is no persistent download-ready indicator. Old rollback copies are retained rather than automatically deleted. These non-blocking preview limits are recorded in PR #10.

**Published beta.1 limitation:** its native archive transfer timed out during post-release verification. Use the release's DMG or ZIP for manual installation if an update download fails. PR #10 replaces the per-byte archive loop with a native file transfer; this fix is not in the published beta.1 assets. Signed-asset verification passed, but real app replacement/restart remains unverified.

Signing follows [Oak's](https://github.com/jellydn/oak) per-app public-key/Actions-secret pattern and pinned Sparkle signing tools. Shelf uses its **own** `SPARKLE_PRIVATE_KEY` secret, not Oak's key. Unlike Sparkle's silent install-on-quit mode, Shelf separates downloads from an explicitly approved native installation. Release publication requires valid publisher signatures on metadata, ZIP and DMG. No Apple Developer account is required for these signatures, but they are **not Developer ID signing or notarization**. First installation or a later restart may need macOS **Privacy & Security → Open Anyway**; Shelf never clears quarantine or disables Gatekeeper. Translocated or non-writable installations also require manual installation; moving the app to a writable location does not establish ownership.

Update metadata expires one year after signing. Publish a new release or explicitly refresh and re-sign its metadata before expiry. Keep the pinned key stable; losing the signing key requires a separately verified manual installation, not an unsigned fallback. Install/restart behavior and native agent model turns must not be described as verified until exercised on the real path; test coverage is recorded in each PR/release.

## Prerequisites

- macOS
- Rust, pinned in `rust-toolchain.toml`

## Install

For Apple Silicon Macs running macOS 14 or later:

```sh
brew install --cask jellydn/tap/shelf
```

If Homebrew refuses to load an untrusted third-party cask, review `jellydn/homebrew-tap` and run `brew trust --cask jellydn/tap/shelf`, then retry installation. This trusts only Shelf, not every entry in the tap. Older Homebrew versions may not require this step.

Or download the DMG or ZIP from [Releases](https://github.com/jellydn/shelf/releases/latest). The app is ad-hoc signed, not notarised. If macOS blocks the first launch, use **System Settings → Privacy & Security → Open Anyway**. Intel builds are not provided yet.

To build from source:

```sh
git clone https://github.com/jellydn/shelf
cd shelf
sh scripts/bundle.sh
open target/release/Shelf.app
```

Copy `target/release/Shelf.app` to Applications if you want to keep it there. The bundle uses `io.github.jellydn.shelf` as its stable identifier and `LSUIElement` to stay out of the Dock. The build script signs it ad hoc for local use; it is not a notarised release.

The archivebox icon appears before the disk scan finishes. Click it to open the panel. Click it again, click outside, or press Escape to close it; moving the pointer outside no longer dismisses it in this PR. On macOS 27, the panel uses AppKit's expanded-interface session; older systems use the button action. Native event routing remains unverified.

The first scan and system gauge commands run on background threads. Memory, CPU, and GPU update about once a second. GPU load comes from the driver's IOKit performance statistics, with renderer/tiler values as a fallback when device utilisation is unavailable. A driver that exposes no load still shows 0%.

For development, `cargo run --release` still works without building a bundle.

CI checks formatting, Clippy, and the full test suite, then uploads packaged test apps. A `v*` tag matching `Cargo.toml` runs the release workflow and publishes an Apple Silicon DMG, ZIP, and `SHA256SUMS`. Packaging follows [Oak's](https://github.com/jellydn/oak) ad-hoc sign/verify-before-archive approach. Update `RELEASE_NOTES.md` before each release. The Homebrew tap checks for published stable releases every six hours and can also be updated manually.

Open Settings from Actions, or with Command-,. The panel closes. Click the menu-bar icon to close Settings and open the panel.

Choose **Quit Shelf** in Actions, or press Command-Q in the panel or Settings, to exit the app.

## Run tests

```sh
cargo test
```

A headless picture of the panel:

```sh
cargo run -- --theme dark --scene ledger-example --snap shots/review-ledger.png
```

## Architecture and decisions

Start with the [architecture map](../.planning/codebase/ARCHITECTURE.md). The same directory covers the stack, integrations, structure, conventions, testing and known concerns. [Agent guidance](../AGENTS.md) remains the operating reference.

- [Scoped agent sessions and per-agent model configuration](adr/0001-scoped-agent-sessions.md)
- [Publisher-verified updates, channels and install approval](adr/0002-publisher-verified-updates.md)

## Author

👤 **Huynh Duc Dung**

- Website: https://productsway.com/
- Twitter: [@jellydn](https://twitter.com/jellydn)
- Github: [@jellydn](https://github.com/jellydn)

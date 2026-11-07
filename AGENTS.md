# Shelf

macOS menu-bar app that reviews what a cleanup level would tick. Rust + Rune (GPU UI); macOS only.

**Confirm never deletes.** It records the choice in memory. There is no delete path, and tests assert the catalog is unchanged. Do not add one.

The codebase map starts at [.planning/codebase/ARCHITECTURE.md](.planning/codebase/ARCHITECTURE.md). Accepted boundary decisions are [scoped agent sessions](docs/adr/0001-scoped-agent-sessions.md) and [publisher-verified updates](docs/adr/0002-publisher-verified-updates.md). Keep these aligned when changing their contracts; this file remains the command/safety reference.

## Commands

- `cargo run --release` — live app (menu-bar archivebox icon).
- `cargo test --locked` — full tests, including one live disk/gauge check (headless; no display needed).
- `cargo test --locked untick_holds` — one test by name filter.
- `cargo fmt --check && cargo clippy --locked --all-targets` — format check and lint, also run in CI.
- `sh scripts/bundle.sh` — build `target/release/Shelf.app` with stable ID `io.github.jellydn.shelf` and `LSUIElement`.
- `sh scripts/release.sh` — sign/verify and package Apple Silicon release assets; no upload.
- `cargo run -- --theme dark --scene balanced --snap shots/balanced.png` — headless panel shot.
- `cargo run -- --window settings --theme dark --scene agent --snap shots/settings-agent.png` — headless Settings shot.

Panel scenes: `scanning`, `careful`, `balanced`, `aggressive`, `asking`, `reviewed`, `agent-failed`, `agent-error-details`, `confirmed`, `menu`, `selected`.
Settings scenes: `general`, `agent`, `agent-copilot`, `agent-pi`, `agent-opencode`, `agent-model-error`, `access`, `protected`.
`--list-windows` shows `main` and `settings`; `--list-scenes` prints the selected window's scenes. Snapshots run `live_state()`, so they scan the real Mac.

## Layout and quirks

- `src/main.rs`: `State`, panel view, level-to-ticks logic, keys, agent result/error views, `AppSpec` and panel scenes. Startup shows a pending catalog immediately; workers load the disk scan and update gauges through Rune's UI queue. Do not move system commands into UI polling.
- `src/palette.rs`: per-row action list, action-list height, Finder/ask/unlock actions. Ask uses the eligible ticked selection, not the hovered row; it also builds the allowed/blocked read scope.
- `src/agent.rs`: background ACP/RPC jobs, elapsed progress, cancellation/deadline, bounded reports, redacted diagnostics, and exact saved-session continuation.
- `src/provider.rs`: private submodule of `agent.rs`; provider-specific launch/resume arguments, model validation, private OpenCode configuration and startup-mode readiness. Never put shared provider configuration writes here.
- `src/broker.rs`: private submodule of `agent.rs`; original ∩ current eligible read scope, descriptor-checked reads, private Unix-socket reader, headless MCP and JSON helpers. Helpers run before Rune or a live scan starts.
- `src/pi-reader.mjs`: Pi's explicit reader extension; only the Shelf tool can run. No JS toolchain or config is checked in; validate changes against the installed Pi SDK (types/lint/Biome).
- `src/catalog.rs`: catalog model, levels, thresholds, live gauges (`diskutil`, `sysctl`, `vm_stat`, Mach host stats). **It includes `scan.rs` as a private submodule** via `#[path = "scan.rs"] mod scan;`. Add `mod scan` there, not in `main.rs`.
- `src/gpu.rs`: typed IOKit performance statistics, with renderer/tiler fallback. No available statistic may legitimately read as 0%.
- `src/scan.rs`: hint rules, folder scan/measure, size cache.
- `src/prefs.rs`: plain-text settings, protected roots, agent `PATH` detection, private run folders, quoted executable launchers, and test temp paths. Its `TOOLS` list detects many ai-launcher CLIs; only five have adapters (see `provider.rs`).
- `src/settings.rs`: separate Settings window with its own `AppSpec` and scenes.
- `src/update.rs`: bundled native updater launch and background wake-up; no UI-thread network/process waits. `native/Updater.swift` owns signed metadata/archive verification, daily throttle, native consent and rollback. Headless snapshots/tests never start checks.
- `src/status.rs`: raw Objective-C `NSStatusItem`; Rune has no status item. macOS 27 uses an expanded-interface delegate/session; older systems use the button action. Pointer exit never dismisses. External mouse-down dismisses outside the panel/button, while a second status-button click toggles it. Escape closes actions first, then the panel. Pure dismissal/geometry helpers are tested; native event routing remains a separate check.
- Tests live in `#[cfg(test)] mod tests` at the bottom of each file. UI tests drive Rune's headless `Runtime` against `catalog::fixture()`. Provider tests use generated `/bin/sh` stubs; never call a real CLI or the network in tests.
- `catalog::load_live()` scans `$HOME` and caches sizes at `~/Library/Application Support/Shelf/sizes.tsv`. Delete that file to force a remeasure. First `cargo run`/`--snap` is slow.
- `prefs::path_for_state()` returns a unique temp file under `cfg(test)`. Tests must not read or write the real `~/Library/Application Support/Shelf/settings.txt`.
- Settings and the panel are never visible together: opening Settings calls `status::conceal()`, and a status-item click hides Settings.

## Invariants

- `prefs::ALWAYS` roots (Mail, Messages, iCloud, CloudStorage, Keychains, SSH, GnuPG, AWS, Kubernetes) ignore Unlock and stay locked.
- Unlock is session-only (`State::unlocked`); restart clears it. Kept folders and the "keep what changed within" hold can be unlocked for the session; in-use paths stay locked.
- A panel scene that opens the action list must update `launch_size()` in `main.rs`; otherwise the headless window is tray-height and clips the list.

## Ask Agent safety and sessions

- Each explicit request snapshots scan/selection context into a new private `~/Library/Application Support/Shelf/Runs/chat-…/context.md` plus `Ask.command`. Never overwrite earlier runs.
- Keep the configured provider. Only `grok`, `agy`, `copilot`, `pi` and `opencode` have adapters; other detected CLIs fail visibly rather than switching tools or guessing flags. Grok/Antigravity/Copilot use ACP plan mode; Pi uses RPC with native tools/discovery disabled; OpenCode uses a private read-only ACP mode.
- Model choices persist per agent (custom choices are keyed by executable) and survive provider switches. Use `--model` where supported, not command-string interpolation. OpenCode 2.0.23 requires private per-run configuration and ACP selection; reject other versions until their schemas are verified. Wait for its `config_option_update` containing `shelf-readonly` before prompting, not a fixed sleep. Check advertised exact model access; do not silently use fuzzy matches or paid fallbacks. Blank may use the CLI default except Copilot uses Auto and OpenCode requires an explicit model.
- Require agent-start permission before a review. Model/account access is not proof of free inference. Copilot Free is allowance-limited and Auto-only; Pi grants no inference access; OpenRouter Free needs a user account/key; OpenCode free models may expire and privacy terms differ. No automatic paid requests.
- Path sharing off means saved context only. With sharing on, the reader permits at most 512 KiB UTF-8, 2,000 lines or 1,024 directory entries for eligible selected paths; reject writes, commands, protected/kept/held/in-use/unselected paths, special files and symlink escapes. Recheck the opened descriptor and current scope before returning data. New grants must not widen the frozen original scope.
- Copilot/OpenCode use only the private Shelf MCP reader and reject native ACP filesystem/permission requests. Pi disables native tools/discovery and loads only the explicit reader extension. Their Terminal continuation keeps these controls and the same broker socket: live reads need Shelf running with original ∩ current eligible selection. Cancel, failure, a replacement job or app exit closes the broker; saved chat remains available, but old folder grants do not revive when Shelf restarts.
- Analysis must not modify ticks, estimates, protections, or Confirm state. Failures stop progress and keep scan results, with expandable redacted details, Agent settings, and Retry. Do not put local paths or credentials in error logs.
- Save `analysis.md` and `session.txt` only after a nonempty completed review. Pi must reach `agent_settled`, not merely `agent_end`, and save its exact session file inside its run's `sessions/` directory. Rewrite `Ask.command` to resume the exact session, with quoted arguments/environment and the same model/tool controls. Before success, refuse an unrelated chat. Recheck the configured agent/model/start permission before Terminal launch.
- Grok/Antigravity Terminal continuation retains its existing native plan/sandbox controls rather than the new MCP broker. Do not claim that those native Terminal tools enforce Shelf's live selected-only boundary. Provider controls, prompt instructions and callbacks are not an independent operating-system sandbox. Destructive actions need separate approval; Shelf has no cleanup/delete path.
- Verify using harmless CLI stubs. Native metadata checks confirmed Antigravity session creation, Copilot plan/Auto selection, Pi model state, and OpenCode private-mode loading/exact-model selection without a model prompt. Real model analysis, tool routing during a native model turn, and native chat continuation remain unverified. Installed Grok currently refuses its read-only sandbox on a socket-symlink deny-list error; do not disable protections or change shared runner settings to bypass it.
- Run folders may contain explicitly shared paths and persist until the user removes them. Do not auto-delete settings or conversations during verification or uninstall work.

## Tests and releases

- `the_live_index_feeds_this_mac` is the only live test: it checks real disk/gauge readings without assuming a particular disk capacity or home-folder contents. It requires macOS. Keep new tests on `fixture_state()`.
- `shots/*.png` are the README screenshots; regenerate them after panel changes with `--theme dark`.
- `Cargo.lock` is committed and Rune is pinned by `rev` in `Cargo.toml`; let Renovate update it.

## Release

- Updates follow Oak's Ed25519 signing-tools/Actions-secret pattern with a **dedicated Shelf key**, never Oak's key. Only `native/update-public-key.txt` is committed; `SPARKLE_PRIVATE_KEY` exists only in Shelf Actions secrets. `scripts/sparkle-tools.sh` verifies the pinned official tools archive, and `scripts/sign-release.py` signs metadata, ZIP and DMG through stdin without private logs. The release helper independently verifies signatures against the committed public key before publication.
- `native/Updater.swift` compiles during bundling and its harmless `--self-test` must pass. `--verify-release <assets> <public-key-file>` verifies real release output without starting UI or installation. Never test replacement against the owner's installed app. Use disposable fixture directories for verification.
- Update policy: daily automatic checks; downloads off by default; stable default, prereleases opt-in; explicit native approval before install/restart; no install-on-quit. Homebrew uses `brew upgrade`, never self-replacement. Missing/invalid/expired publisher signatures fail closed. Do not use a Sparkle automatic-install toggle: it can install on quit, contrary to this policy. No Gatekeeper/quarantine bypass, Developer ID or notarization claim.
- Publisher verification does not require an Apple Developer account. Ad-hoc builds may need macOS Open Anyway; non-writable/translocated apps require manual installation. Keep key continuity, version/channel checks and rollback. Metadata expires after one year and must be re-signed or superseded. Native real-install/restart and agent model-turn limitations must be explicit in PR/release notes.
- Prerelease tags (for example `v0.2.0-beta.1`) publish with `prerelease: true` and `make_latest: false` from the exact PR commit, not an implicit main merge. Do not move the stable Homebrew cask to prereleases. `ShelfVersion` holds full SemVer; bundle display/build versions use macOS-compatible numeric/beta forms.
- CI runs formatting, Clippy, the full tests, and app packaging on `macos-26`.
- `sh scripts/release.sh` creates an ad-hoc signed Apple Silicon DMG, ZIP, and `SHA256SUMS` under `target/release/assets`.
- `assets/Shelf.icns` is the app icon; `assets/favicon.svg` and raster sizes are for documentation sites, not the monochrome menu-bar symbol. Regenerate with `uv run --with pillow scripts/icons.py` on macOS; ordinary builds only copy committed assets.
- Push a `v*` tag matching `Cargo.toml` to publish those assets. Update `RELEASE_NOTES.md` before tagging. Builds are not notarized.
- Branch/PR builds package test apps without requiring a release tag; tag builds must validate their version. Do not try to override GitHub's reserved `GITHUB_*` variables.
- Release assets are `Shelf-<version>-macos-arm64.dmg`, `Shelf-<version>-macos-arm64.zip`, and `SHA256SUMS`. Minimum macOS is 14. Verify extracted/mounted bundle signatures and architecture, then re-download published assets and compare their digests. Ad-hoc signing is not Developer ID signing or notarization.
- The Homebrew cask is `Casks/shelf.rb` in `jellydn/homebrew-tap`, installed with `brew install --cask jellydn/tap/shelf`. Its updater reads stable published releases, verifies the downloaded DMG against GitHub's digest, and updates only this cask. The tap workflow checks every six hours and supports manual dispatch; it needs no cross-repository secret.
- Homebrew versions with third-party trust enforcement require the user's cask-scoped `brew trust --cask jellydn/tap/shelf`. Do not trust the entire tap or alter shared runner trust during verification. Use a disposable `XDG_CONFIG_HOME` and `HOMEBREW_CACHE` for isolated trust/audit/fetch checks; do not launch cleanup or overwrite an installed app.

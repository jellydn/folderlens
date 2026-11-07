# Shelf

macOS menu-bar app that reviews what a cleanup level would tick. Rust + Rune (GPU UI); macOS only.

**Confirm never deletes.** It records the choice in memory. There is no delete path, and tests assert the catalog is unchanged. Do not add one.

## Commands

- `cargo run --release` — live app (menu-bar archivebox icon).
- `cargo test --locked` — full tests, including one live disk/gauge check (headless; no display needed).
- `cargo test untick_holds` — one test by name filter.
- `cargo fmt --check && cargo clippy --locked --all-targets` — format check and lint, also run in CI.
- `sh scripts/bundle.sh` — build `target/release/Shelf.app` with stable ID `io.github.jellydn.shelf` and `LSUIElement`.
- `sh scripts/release.sh` — sign/verify and package Apple Silicon release assets; no upload.
- `cargo run -- --scene balanced --snap shots/balanced.png` — headless panel shot.
- `cargo run -- --window settings --scene agent --snap shots/settings-agent.png` — headless Settings shot.

Panel scenes: `scanning`, `careful`, `balanced`, `aggressive`, `asking`, `reviewed`, `agent-failed`, `agent-error-details`, `confirmed`, `menu`, `selected`.
Settings scenes: `general`, `agent`, `access`, `protected`.
`--list-scenes` prints them for the selected window. Snapshots run `live_state()`, so they scan the real Mac.

## Layout and quirks

- `src/main.rs`: `State`, panel view, level-to-ticks logic, keys, agent result/error views, `AppSpec` and panel scenes. Startup shows a pending catalog immediately; workers load the disk scan and update gauges through Rune's UI queue. Do not move system commands into UI polling.
- `src/palette.rs`: per-row action list, action-list height, Finder/ask/unlock actions. Ask uses the eligible ticked selection, not the hovered row; it also builds the allowed/blocked read scope.
- `src/agent.rs`: background ACP jobs for Grok/Antigravity, elapsed progress, cancellation/deadline, read-only callbacks, bounded reports, redacted diagnostics, and saved provider-session continuation.
- `src/catalog.rs`: catalog model, levels, thresholds, live gauges (`diskutil`, `sysctl`, `vm_stat`, Mach host stats). **It includes `scan.rs` as a private submodule** via `#[path = "scan.rs"] mod scan;`. Add `mod scan` there, not in `main.rs`.
- `src/gpu.rs`: typed IOKit performance statistics, with renderer/tiler fallback. No available statistic may legitimately read as 0%.
- `src/scan.rs`: hint rules, folder scan/measure, size cache.
- `src/prefs.rs`: plain-text settings, protected roots, agent `PATH` detection, private run folders, quoted executable launchers, and test temp paths.
- `src/settings.rs`: separate Settings window with its own `AppSpec` and scenes.
- `src/status.rs`: raw Objective-C `NSStatusItem`; Rune has no status item. macOS 27 uses an expanded-interface delegate/session; older systems use the button action. Pointer tracking preserves the button-to-panel crossing and dismisses outside the panel. Pure geometry helpers are unit tested.
- Tests live in `#[cfg(test)] mod tests` at the bottom of each file. UI tests drive Rune's headless `Runtime` against `catalog::fixture()`.
- `catalog::load_live()` scans `$HOME` and caches sizes at `~/Library/Application Support/Shelf/sizes.tsv`. Delete that file to force a remeasure. First `cargo run`/`--snap` is slow.
- `prefs::path_for_state()` returns a unique temp file under `cfg(test)`. Tests must not read or write the real `~/Library/Application Support/Shelf/settings.txt`.
- Settings and the panel are never visible together: opening Settings calls `status::conceal()`, and a status-item click hides Settings.

## Invariants

- `prefs::ALWAYS` roots (Mail, Messages, iCloud, CloudStorage, Keychains, SSH, GnuPG, AWS, Kubernetes) ignore Unlock and stay locked.
- Unlock is session-only (`State::unlocked`); restart clears it. Kept folders and the "keep what changed within" hold can be unlocked for the session; in-use paths stay locked.
- A panel scene that opens the action list must update `launch_size()` in `main.rs`; otherwise the headless window is tray-height and clips the list.

## Ask Agent safety and sessions

- Each explicit request snapshots scan/selection context into a new private `~/Library/Application Support/Shelf/Runs/chat-…/context.md` plus `Ask.command`. Never modify Levo's reference files or overwrite earlier runs.
- Keep the configured provider. Only Grok and Antigravity have ACP adapters; unsupported tools fail visibly rather than switching tools or guessing common CLI flags.
- Require agent-start permission and advertised ACP plan mode before sending a review prompt. No automatic paid requests. Interval/pause work remains deferred to GitHub issue #5.
- Path sharing off means saved context only. With sharing on, callbacks permit bounded UTF-8 reads/listings for eligible selected paths; reject writes, commands, protected/kept/held/in-use/unselected paths, and symlink escapes. Recheck access during work; newly granted access must not widen the original scope.
- Analysis must not modify ticks, estimates, protections, or Confirm state. Failures stop progress and keep scan results, with expandable redacted details, Agent settings, and Retry. Do not put local paths or credentials in error logs.
- Save `analysis.md` and `session.txt` only after a nonempty completed review. Then rewrite `Ask.command` to resume that exact session ID in its run folder. Before success, refuse to open an unrelated chat. Recheck the configured agent/start permission before Terminal launch.
- Terminal is governed by the provider's own plan/sandbox controls, not Shelf's ACP read filter. Prompt instructions and callbacks are not an independent operating-system sandbox. Destructive actions need separate approval; Shelf has no cleanup/delete path.
- Verify using harmless CLI stubs. Native initialization/session creation worked for Antigravity without a model prompt. Real model analysis, native file-tool routing, and native chat continuation remain unverified. Installed Grok currently refuses its read-only sandbox on a socket-symlink deny-list error; do not disable protections or change shared runner settings to bypass it.
- Run folders may contain explicitly shared paths and persist until the user removes them. Do not auto-delete settings or conversations during verification or uninstall work.

## Tests and releases

- `the_live_index_feeds_this_mac` is the only live test: it checks real disk/gauge readings without assuming a particular disk capacity or home-folder contents. It requires macOS. Keep new tests on `fixture_state()`.
- `shots/*.png` are the README screenshots; regenerate them after panel changes.
- `Cargo.lock` is committed and Rune is pinned by `rev` in `Cargo.toml`; let Renovate update it.

## Release

- CI runs formatting, Clippy, the full tests, and app packaging on `macos-26`.
- `sh scripts/release.sh` creates an ad-hoc signed Apple Silicon DMG, ZIP, and `SHA256SUMS` under `target/release/assets`.
- Push a `v*` tag matching `Cargo.toml` to publish those assets. Update `RELEASE_NOTES.md` before tagging. Builds are not notarized.
- Branch/PR builds package test apps without requiring a release tag; tag builds must validate their version. Do not try to override GitHub's reserved `GITHUB_*` variables.
- Release assets are `Shelf-<version>-macos-arm64.dmg`, `Shelf-<version>-macos-arm64.zip`, and `SHA256SUMS`. Minimum macOS is 14. Verify extracted/mounted bundle signatures and architecture, then re-download published assets and compare their digests. Ad-hoc signing is not Developer ID signing or notarization.
- The Homebrew cask is `Casks/shelf.rb` in `jellydn/homebrew-tap`, installed with `brew install --cask jellydn/tap/shelf`. Its updater reads stable published releases, verifies the downloaded DMG against GitHub's digest, and updates only this cask. The tap workflow checks every six hours and supports manual dispatch; it needs no cross-repository secret.

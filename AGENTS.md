# Shelf

macOS menu-bar app that reviews what a cleanup level would tick. Rust + Rune (GPU UI); macOS only.

**Confirm never deletes.** It records the choice in memory. There is no delete path, and tests assert the catalog is unchanged. Do not add one.

## Commands

- `cargo run --release` — live app (menu-bar archivebox icon).
- `cargo test` — all tests (headless; no display needed).
- `cargo test untick_holds` — one test by name filter.
- `cargo fmt && cargo clippy --all-targets` — format and lint. There is no CI; run these yourself.
- `cargo run -- --scene balanced --snap shots/balanced.png` — headless panel shot.
- `cargo run -- --window settings --scene agent --snap shots/settings-agent.png` — headless Settings shot.

Panel scenes: `careful`, `balanced`, `aggressive`, `confirmed`, `menu`, `selected`.
Settings scenes: `general`, `agent`, `access`, `protected`.
`--list-scenes` prints them for the selected window. Snapshots run `live_state()`, so they scan the real Mac.

## Layout and quirks

- `src/main.rs`: `State`, panel view, level-to-ticks logic, keys, `AppSpec` and panel scenes.
- `src/palette.rs`: per-row action list, action-list height, Finder/ask/unlock actions.
- `src/catalog.rs`: catalog model, levels, thresholds, live gauges (`diskutil`, `sysctl`, `vm_stat`, Mach host stats). **It includes `scan.rs` as a private submodule** via `#[path = "scan.rs"] mod scan;`. Add `mod scan` there, not in `main.rs`.
- `src/scan.rs`: hint rules, folder scan/measure, size cache.
- `src/prefs.rs`: plain-text settings, protected roots, agent `PATH` detection, test temp path.
- `src/settings.rs`: separate Settings window with its own `AppSpec` and scenes.
- `src/status.rs`: raw Objective-C `NSStatusItem`; Rune has no status item. The pure `popover_origin` helper is the testable part.
- Tests live in `#[cfg(test)] mod tests` at the bottom of each file. UI tests drive Rune's headless `Runtime` against `catalog::fixture()`.
- `catalog::load_live()` scans `$HOME` and caches sizes at `~/Library/Application Support/Shelf/sizes.tsv`. Delete that file to force a remeasure. First `cargo run`/`--snap` is slow.
- `prefs::path_for_state()` returns a unique temp file under `cfg(test)`. Tests must not read or write the real `~/Library/Application Support/Shelf/settings.txt`.
- Settings and the panel are never visible together: opening Settings calls `status::conceal()`, and a status-item click hides Settings.

## Invariants

- `prefs::ALWAYS` roots (Mail, Messages, iCloud, CloudStorage, Keychains, SSH, GnuPG, AWS, Kubernetes) ignore Unlock and stay locked.
- Unlock is session-only (`State::unlocked`); restart clears it. Kept folders and the "keep what changed within" hold can be unlocked for the session; in-use paths stay locked.
- A panel scene that opens the action list must update `launch_size()` in `main.rs`; otherwise the headless window is tray-height and clips the list.

## Tests and releases

- `the_live_index_feeds_this_mac` is the only live test: it checks real disk/gauge readings without assuming a particular disk capacity or home-folder contents. It requires macOS. Keep new tests on `fixture_state()`.
- `shots/*.png` are the README screenshots; regenerate them after panel changes.
- `Cargo.lock` is committed and Rune is pinned by `rev` in `Cargo.toml`; let Renovate update it.

## Release

- CI runs formatting, Clippy, the full tests, and app packaging on `macos-26`.
- `sh scripts/release.sh` creates an ad-hoc signed Apple Silicon DMG, ZIP, and `SHA256SUMS` under `target/release/assets`.
- Push a `v*` tag matching `Cargo.toml` to publish those assets. Update `RELEASE_NOTES.md` before tagging. Builds are not notarized.
- The Homebrew cask lives in `jellydn/homebrew-tap`; its updater reads the published release and verifies its asset digest.

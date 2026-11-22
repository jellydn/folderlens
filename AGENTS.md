# FolderLens

macOS menu-bar app that reviews what a cleanup level would tick. Rust + Rune (GPU UI); macOS only.

Record choice (formerly Confirm) never deletes. It records a nonempty eligible selection in memory, and tests assert the catalog is unchanged. The only file-moving path allowed is Move to Trash under [ADR 4](docs/adr/0004-move-to-trash-with-confirmation.md): user-started, FolderLens-confirmed, protections first, Trash only. Never add permanent deletion, emptying the Trash, or a path an agent or AI label can start.

The codebase map starts at [.planning/codebase/ARCHITECTURE.md](.planning/codebase/ARCHITECTURE.md). Accepted boundary decisions are [scoped agent sessions](docs/adr/0001-scoped-agent-sessions.md), [publisher-verified updates](docs/adr/0002-publisher-verified-updates.md), [review without an agent](docs/adr/0003-review-works-without-ai.md), [move to Trash with confirmation](docs/adr/0004-move-to-trash-with-confirmation.md) and [Local AI and advisory triage](docs/adr/0005-local-ai-and-advisory-triage.md). ADR 4 is implemented in `src/trash.rs` (sheet and recheck in `src/main.rs`: `trash_sheet`, `still_eligible`, `move_to_trash`); ADR 5 is implemented: the Local AI provider, consent, the quick switch, private Pi/OpenCode runs and advisory triage labels. Keep them aligned when you change their contracts.

## Commands

- `cargo run --release` — live app (menu-bar archivebox icon).
- `cargo test --locked` — full tests, headless; one live disk/gauge test needs macOS.
- `node --test tests/pi-reader.mjs` — Pi reader boundary with a harmless helper.
- `cargo fmt --check && cargo clippy --locked --all-targets` — also run in CI.
- `sh scripts/bundle.sh` — build `target/release/FolderLens.app`.
- `sh scripts/release.sh` — sign, verify and package release assets under `target/release/assets`; no upload.
- `cargo run -- --theme dark --scene ledger-example --snap shots/review-ledger.png` — headless panel shot. Add `--window settings` for Settings; `--list-scenes` lists scenes. `*-example` and `agent-error-details` use synthetic data; other panel scenes scan the real Mac.

## Quirks

- `catalog.rs` includes `scan.rs` with `#[path]`. Do not add `mod scan` to `main.rs`.
- Workers load scans and gauges through Rune's UI queue. Do not move system commands into UI polling.
- A panel scene that opens the action list must update `launch_size()`, or the headless window clips the list.
- The first live scan is slow. Delete `~/Library/Application Support/FolderLens/sizes.tsv` to force a remeasure.
- Tests must not touch the real `settings.txt`; `prefs::path_for_state()` gives a temp file under `cfg(test)`. Keep new tests on `fixture_state()`. Provider tests use `/bin/sh` stubs; never call a real CLI or the network.
- `src/pi-reader.mjs` has no checked-in JS toolchain. Validate it against the installed Pi SDK (types, lint, Biome).
- Legacy Shelf settings import once when FolderLens settings are absent. Keep legacy settings and conversations intact.
- After panel changes, regenerate `shots/review-*.png` and `shots/exclusion-rules.png` with `--theme dark`. Keep the original screenshots linked in README as provenance.

## Folder safety

- `prefs::ALWAYS` roots ignore Unlock. Unlock is session-only; in-use paths stay locked.
- Do not claim reclaimable space; sizes are estimates.

## Ask Agent safety

- Each request writes a new private run folder under `~/Library/Application Support/FolderLens/Runs/`. Never overwrite earlier runs, and never auto-delete runs, settings or conversations, including during verification.
- Keep the configured provider. Only `grok`, `agy`, `copilot`, `pi` and `opencode` have CLI adapters, plus the Local AI provider (`agent_id=local`); other CLIs fail visibly. Do not guess flags or switch tools.
- Local AI (`src/local_ai.rs`): loopback only unless `local_allow_network`; public addresses are refused in the resolver, not only at validation. No proxy, no redirects. The API key lives only in the Keychain (`src/keychain.rs`), read on workers, and is sent only over https or to loopback (a 100.64/10 address is not proof of Tailscale). Local AI gets names, sizes and FolderLens notes (full paths only with path sharing on), never file contents or tools, and writes no run folder. Consent keys are JSON `[provider, model]` pairs; a running job ends when its consent key changes or is revoked. Tests use a loopback stub server, never a real model.
- Private runs (`src/private_cli.rs`, `private_cli` switch): Pi `-p --no-session --no-tools --offline`, OpenCode `run --standalone` with `OPENCODE_DB=:memory:`, `XDG_DATA_HOME` and config in a 0700 temp folder, every tool and permission denied. Prompt on stdin, the version gate (Pi 1.5.2, OpenCode 2.3.0 only, separate from the chat adapter's 2.0.23) runs with the same private environment before anything is sent, the temp folder is removed. The guard snapshots the CLI data folders before `--version` and after the run; a change records `adapter_disabled` and refuses Ask for that agent until **Turn on again**. Never delete the CLI's files. No run folder or Terminal continuation for private runs.
- Triage (`src/triage.rs`): only Local AI and private runs can sort. The answer must be JSON lines or one array of `{"i","label","reason"}`; any other text, unknown label, out-of-range or repeated index fails the whole sort. Labels live in `State.triage` by path, clear on a rescan, never change ticks, and show only on eligible rows (`selection_reason` checks `can_tick` first). Do not tune the prompt to the `--eval-triage` fixtures.
- Nothing is sent before consent for the exact `consent_key()` (provider + model, plus host outside this Mac).
- Pass models with `--model` where supported, never by command-string interpolation. Do not use fuzzy model matches or paid fallbacks. OpenCode: only 2.0.23 is verified; wait for its `config_option_update` with `folderlens-readonly`, not a fixed sleep.
- Require agent-start permission before a review. Model or account access is not proof of free inference. No automatic paid requests.
- Path sharing off means saved context only. With sharing on, reads are limited to eligible selected paths (512 KiB, 2,000 lines, 1,024 entries) and the original ∩ current scope; new grants never widen it, and old grants never revive after restart.
- Analysis must not change ticks, estimates, protections or the recorded choice. Do not put local paths or credentials in error logs.
- Save `analysis.md` and `session.txt` only after a nonempty completed review. Pi must reach `agent_settled`, not `agent_end`.
- Provider controls are not an OS sandbox. Grok/Antigravity Terminal continuation does not use the FolderLens broker; do not claim it enforces the selected-only boundary. Installed Grok refuses its sandbox on a socket-symlink error; do not disable protections or change runner settings to bypass it.
- Real model turns, tool routing during a turn and native chat continuation remain unverified. Say so in PR and release notes.

## Releases

- Updates use a dedicated FolderLens Ed25519 key, never Oak's. Only `native/update-public-key.txt` is committed; the private key lives only in Actions secrets.
- Update policy: daily checks, downloads off by default, prereleases opt-in, explicit approval before install, no install-on-quit, no Sparkle automatic-install toggle. Missing or invalid signatures fail closed. Homebrew installs use `brew upgrade`, never self-replacement.
- Unknown installation origin is manual-update-only. Do not add a preference bypass for `requireDirectInstallation`.
- Never test replacement against the owner's installed app; use disposable fixture directories. For Homebrew checks use a disposable `XDG_CONFIG_HOME` and `HOMEBREW_CACHE`, and do not trust the whole tap.
- Builds are ad-hoc signed, not notarized. Do not claim Developer ID, notarization or a Gatekeeper bypass.
- Push a `v*` tag matching `Cargo.toml` to publish; update `RELEASE_NOTES.md` first. Prerelease tags publish with `prerelease: true` and `make_latest: false`, and the stable Homebrew cask never moves to prereleases.
- Do not republish 0.2.0 (still Shelf) or rewrite historical release notes.
- Let Renovate update `Cargo.lock` and the pinned Rune `rev`.

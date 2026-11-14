# External Integrations

**Analysis Date:** 2026-11-14

Source: `main` at `0c68acb`, FolderLens 0.3.7 (`Cargo.toml`). Accepted boundaries: `docs/adr/0001-scoped-agent-sessions.md`, `docs/adr/0002-publisher-verified-updates.md`, `docs/adr/0003-review-works-without-ai.md`.

## APIs & External Services

**Agent CLIs (local processes, manual Ask Agent only):**
- Detection: `PATH` lookup of 30 command names from the ai-launcher catalog. FolderLens does not run a tool to detect it (`src/prefs.rs` `TOOLS`, `find_bin`).
- Adapters exist only for `grok`, `agy`, `copilot`, `pi` and `opencode`, chosen by executable file name. Any other CLI fails visibly; FolderLens never switches agents (`src/provider.rs` `Provider::for_bin`).
- Per-adapter launch (`src/provider.rs` `Launch::prepare`). Every launch sets `NO_COLOR=1` and runs in the private run folder:
  - Grok: `agent stdio` over ACP; `--permission-mode plan --sandbox read-only`; env `GROK_FOLDER_TRUST=0`; continuation `--resume`.
  - Antigravity (`agy`): `acp`; `--mode plan --sandbox`; continuation `--conversation`.
  - Copilot: `--acp`; `--mode plan --sandbox --no-auto-update --no-custom-instructions --disable-builtin-mcps --no-bash-env --sync-session=off`, only tool `folderlens-read_selected` via `--additional-mcp-config` (FolderLens's own stdio MCP reader); removes `COPILOT_ALLOW_ALL` and `COPILOT_ASSISTED_APPROVAL`; blank model sends `--model auto`; continuation `--resume`.
  - Pi: `--mode rpc` (JSON lines, not ACP); `--no-builtin-tools --no-extensions --no-mcp --no-skills --no-prompt-templates --no-context-files --no-approve --offline`, `--tools folderlens_read_selected`, one explicit `--extension` (`folderlens-reader.mjs`) and `--session-dir <run>/sessions`; continuation `--session`.
  - OpenCode: `acp`. First runs `opencode --version` (3 s bound, process group killed after) and accepts only `2.0.23`. Writes a private `opencode.json` (deny-all permissions except `folderlens_read_selected`, agent `folderlens-readonly`, updates/sharing/snapshots off, local MCP reader). Continuation `--standalone --session`, wrapped in `folderlens --folderlens-opencode-resume`, which rechecks the version before `exec` (`src/provider.rs` `resume_helper`, `tests/opencode_continuation.rs`).
- Models: one ID, at most 256 chars, no leading `-`, no whitespace. Pi and OpenCode need `provider/id`; OpenCode needs an explicit model. Passed with `--model` (OpenCode: in its config and `session/set_config_option`), never by command-string interpolation. No fuzzy matches, no paid fallback (`src/provider.rs` `validate_model`).
- ACP client (`src/agent.rs` `acp_review`): `initialize` with `protocolVersion` 1, `fs.readTextFile` only when not broker-only, `writeTextFile: false`, `terminal: false`. Uses `authenticate` only for an offered `cached_token` method. Requires an advertised plan mode (Copilot: `https://agentclientprotocol.com/protocol/session-modes#plan`) and an advertised exact model before `session/prompt`. OpenCode waits up to 5 s for `config_option_update` with `folderlens-readonly`, not a fixed sleep (`src/provider.rs` `wait_for_private_opencode_mode`).
- Agent requests: `session/request_permission` gets `allow_once` only for `read`/`search` with every location inside the allowed scope; everything else is `cancelled`. Writes, terminal and other methods get JSON-RPC error `-32603` (`src/agent.rs` `incoming`).
- Pi RPC (`src/agent.rs` `pi_review`): `get_state` must report the exact configured model; then `prompt`; success needs the prompt `response` and `agent_settled` (not `agent_end`), and a saved session file inside `<run>/sessions/`. Output limit 256 KiB.
- Review deadline 600 s; cancellation kills and reaps the provider (`src/agent.rs`).
- SDK/Client: Rust `std::process` pipes plus `serde_json`; Pi extension uses the installed Pi SDK (`@earendil-works/pi-coding-agent` types) and Node `child_process.spawnSync` (`src/pi-reader.mjs`). No hosted inference SDK.
- Auth: each CLI's own login or key. FolderLens supplies no inference credit; model or account access is not proof of free use. Model notes in `src/agent.rs` `model_note` say limits and possible charges apply.
- Consent: `allow_start` must be on before a review; `send_paths` separately controls whether eligible selected paths are shared (`src/prefs.rs` `prepare_ask`, `src/agent.rs` `open_chat`). Both default off. No automatic paid requests; local summary and scans need no agent (`docs/adr/0003-review-works-without-ai.md`).
- Not verified: real model turns, tool routing during a turn, and native chat continuation (`AGENTS.md`, `README.md`). Grok/Antigravity Terminal continuation does not use the FolderLens broker (`AGENTS.md`). Provider flags are not an OS sandbox.

**GitHub Releases (update helper `native/Updater.swift`):**
- Discovery: `https://api.github.com/repos/jellydn/folderlens/releases?per_page=100&page=N`, up to 10 pages, 2 MiB per page; an incomplete list is an error, not "up to date" (`availableReleasePages`).
- Download base is constructed, never taken from API data: `https://github.com/jellydn/folderlens/releases/download/<tag>/`. Fetches `update.json` (≤16 KiB), `update.json.sig` (64 bytes) and `FolderLens-<version>-macos-arm64.zip` (≤512 MiB, signed size bound).
- Client: ephemeral `URLSession`, no cookie or credential storage, 30 s request / 300 s resource timeouts, `User-Agent: FolderLens/<version>`. No scan, path or agent data is sent.
- Verification: Ed25519 (`Curve25519.Signing`) against the bundled `Contents/Resources/update-public-key.txt`; manifest `schema` 1, `tag`, `version`, `channel` (`stable`/`prerelease`), `expires`, `asset`, `size`, `sha256`; then exact ZIP size and SHA-256. Missing or invalid signatures fail closed.
- After download: quarantine with `/usr/bin/xattr -w com.apple.quarantine`, then an explicit **Reveal in Finder** (`NSWorkspace.activateFileViewerSelecting`). Final replacement is manual. `requireDirectInstallation` always throws (`unknownOrigin` or `unsupportedInstall`), so the retained install path (`ditto`, `codesign --verify`, `renamex_np` swap, `open`) is not reachable for current installs. Do not add a bypass (`AGENTS.md`).
- Started from `src/update.rs`: `--manual` on **Check for Updates…**, `--automatic` hourly from a background thread (not headless). The helper itself throttles automatic checks to 24 h and reads `update_checks`, `update_downloads`, `update_prereleases` from `settings.txt` (`native/Updater.swift` `check`, `src/settings_updates.rs`).

**Homebrew tap:**
- Cask `jellydn/tap/folderlens`, maintained in the separate `jellydn/homebrew-tap` repository, not in this repo (`README.md`, `docs/guide.md`). The stable cask checks for new stable releases every six hours and never moves to prereleases (`README.md`, `AGENTS.md`).
- Detection: resolved bundle under `Caskroom`, or a `folderlens/*/FolderLens.app` link in `/opt/homebrew/Caskroom`, `/usr/local/Caskroom` or `$HOMEBREW_PREFIX/Caskroom` (`native/Updater.swift` `isHomebrew`). Homebrew apps get `brew upgrade --cask jellydn/tap/folderlens` guidance before any download; never self-replacement.
- `brew trust --cask jellydn/tap/folderlens` trusts only this cask, not the whole tap (`docs/guide.md`).

**macOS system commands and APIs:**
- Scan and gauges, from worker threads (not UI polling): `ps -ax -o comm=` and `ps -ax -o command=` (in-use detection), `diskutil info /`, `sysctl -n hw.memsize`, `vm_stat`, Mach `host_statistics` (`src/catalog.rs`).
- Folder picker: `osascript` "choose folder" (disabled under `cfg(test)`) (`src/prefs.rs` `choose_folder`).
- `open` for: Full Disk Access pane `x-apple.systempreferences:…Privacy_AllFiles` (`src/prefs.rs`), reveal a selected item `open -R` (`src/palette.rs`), the run folder in Finder (`src/main.rs`), and the saved chat `Ask.command` in Terminal after a successful review with the same agent and model (`src/agent.rs` `open_chat`).
- AppKit status item, event monitors and screens via `objc_msgSend` and `block2` (`src/status.rs`).
- No delete or cleanup command exists. Record choice only records a selection in memory (`AGENTS.md`).

## Data Storage

**Databases:**
- None.
- Connection: not applicable.
- Client: plain files under `~/Library/Application Support/FolderLens/` (`src/prefs.rs` `user_path`).

**File Storage:**
- Local filesystem only.
- `settings.txt`: `key=value` preferences, atomic write via `settings.txt.tmp` (`src/prefs.rs`). One-time import from `~/Library/Application Support/Shelf/settings.txt`; the legacy file is kept.
- `Runs/chat-<nanos>-<pid>-<seq>/`: created exclusively with mode 0700, never overwritten or auto-deleted (`src/prefs.rs` `prepare_run`). Contents: `context.md`, `Ask.command` (0700; fails until a successful review, then rewritten by `resume_run`), redacted `agent.log`, and `analysis.md` + `session.txt` only after a nonempty completed review (`src/agent.rs` `analyze`). Pi adds `folderlens-reader.mjs` and `sessions/`; OpenCode adds `opencode.json`, `config/`, `home/` (`src/provider.rs`).
- `Updates/` (0700): verified ZIPs, `last-check`, `running.lock` (`flock`, 0600) (`native/Updater.swift`).

**Caching:**
- `sizes.tsv`: versioned (`v2`) size / newest-time / root-mtime cache from allocated blocks. Delete it to force a remeasure (`src/scan.rs`, `AGENTS.md`). Sizes are estimates, not reclaimable space.

## Authentication & Identity

**Auth Provider:**
- None of its own. Agent accounts belong to each CLI. FolderLens never logs in and does not handle API keys; the model field rejects flags and tells users not to enter keys (`src/provider.rs`).
- Implementation: publisher identity for updates is the dedicated FolderLens Ed25519 key, never Oak's. Only `native/update-public-key.txt` is committed (`AGENTS.md`, `docs/adr/0002-publisher-verified-updates.md`). Apple code signatures are ad-hoc (`codesign --sign -`), not Developer ID, not notarized (`scripts/bundle.sh`).

## Monitoring & Observability

**Error Tracking:**
- None. No telemetry or crash reporting.

**Logs:**
- Per-run `agent.log` from provider stderr, at most 16 KiB. Lines with credential words (`token`, `secret`, `api_key`, `bearer`, `sk-`, `ghp_`, `xai-`, …), `@`, or path separators are withheld (`src/agent.rs` `redact_diagnostics`).
- Inline status, error Details and Retry in the panel; native alerts from the updater. The signing script prints no private diagnostics (`scripts/sign-release.py`).

## CI/CD & Deployment

**Hosting:**
- GitHub Releases for `jellydn/folderlens`: `FolderLens-<version>-macos-arm64.dmg`, `.zip`, `SHA256SUMS`, `*.sig`, `update.json` (`.github/workflows/release.yml`). Homebrew cask in `jellydn/homebrew-tap`.

**CI Pipeline:**
- GitHub Actions on `macos-26`; actions pinned by commit SHA (`actions/checkout` v7, `Swatinem/rust-cache` v2, `actions/upload-artifact` v7, `softprops/action-gh-release` v3).
- `.github/workflows/ci.yml` (push to `main`, pull requests, manual; `contents: read`): `cargo fmt --check`, `cargo clippy --locked --all-targets`, `cargo test --locked`, `node --test tests/pi-reader.mjs`, `sh scripts/release.sh`; uploads the packaged test assets (no publisher signatures) for 7 days.
- `.github/workflows/release.yml` (`v*` tags; `contents: write`): same checks; `scripts/release.sh` (tag must equal `v<Cargo version>`); `scripts/sparkle-tools.sh` fetches Sparkle 2.10.1 `sign_update` with a pinned SHA-256; `scripts/sign-release.py` writes `update.json` (expires in 365 days) and signs it, the ZIP and the DMG; `FolderLensUpdater --verify-release` checks them before publication. Notes are the first `RELEASE_NOTES.md` section, whose heading must start `## FolderLens <version> — `. Tags with `-` publish `prerelease: true`, `make_latest: false`.
- Renovate (`renovate.json`) updates `Cargo.lock` and the Rune `rev`.

## Environment Configuration

**Required env vars:**
- Runtime: `HOME`, `PATH` (`src/prefs.rs`). `HOMEBREW_PREFIX` is optional and only adds a Caskroom to check (`native/Updater.swift`).
- Set per run for agents: `NO_COLOR`, `GROK_FOLDER_TRUST`, `FOLDERLENS_READER_EXECUTABLE`, `FOLDERLENS_READER_SOCKET`, `FOLDERLENS_EXPECTED_MODEL`, `OPENCODE_CONFIG`, `OPENCODE_CONFIG_DIR`, `OPENCODE_TEST_HOME`, `OPENCODE_CONFIG_PROJECT_DISABLE`. Removed: `COPILOT_ALLOW_ALL`, `COPILOT_ASSISTED_APPROVAL`, `OPENCODE_CONFIG_CONTENT` (`src/provider.rs`).
- Release: `SPARKLE_PRIVATE_KEY` (signing step only), `GITHUB_REF_TYPE`, `GITHUB_REF_NAME` (`.github/workflows/release.yml`, `scripts/release.sh`, `scripts/sign-release.py`).
- Tests: `FOLDERLENS_STATUS_TEST_CHILD` (`src/status.rs`). Homebrew checks use a disposable `XDG_CONFIG_HOME` and `HOMEBREW_CACHE` (`AGENTS.md`).

**Secrets location:**
- `SPARKLE_PRIVATE_KEY` in the FolderLens repository's GitHub Actions secrets only (`.github/workflows/release.yml`, `docs/adr/0002-publisher-verified-updates.md`). Provider credentials stay in each CLI's own store. Nothing secret is committed.

## Webhooks & Callbacks

**Incoming:**
- No HTTP listener or public webhook.
- Private Unix socket `/tmp/folderlens-read-<pid>-<stamp>-<seq>/read.sock` (dir 0700; short path because macOS limits socket paths to 104 bytes), removed when the broker stops (`src/broker.rs` `Broker::start`).
- Stdio MCP server `folderlens --folderlens-reader <socket>`: `protocolVersion` `2024-11-05`, one tool `read_selected` (read-only annotations); other tools get `-32601` (`src/broker.rs` `mcp_response`). Pi uses `folderlens --folderlens-read <socket>` through `spawnSync` (5 s timeout, 4 MiB buffer) (`src/pi-reader.mjs`).
- Reads are limited to eligible selected paths in the original ∩ current scope, excluding blocked paths: 512 KiB per file, 2,000 lines, 1,024 directory entries (`src/broker.rs` `Reader::read`, `src/agent.rs` `Access`).
- Pi extension hooks: `input` blocks prompts when the restored model is not `FOLDERLENS_EXPECTED_MODEL`; `tool_call` blocks every tool except `folderlens_read_selected`; `user_bash` is disabled (`src/pi-reader.mjs`, `tests/pi-reader.mjs`).
- AppKit status-item and event-monitor callbacks (`src/status.rs`).

**Outgoing:**
- ACP / Pi RPC messages to the configured local CLI, only after the user starts a review.
- HTTPS GETs to `api.github.com` and `github.com` from the update helper; release CI also downloads Sparkle from `github.com/sparkle-project/Sparkle` (`scripts/sparkle-tools.sh`).
- `open` / `osascript` actions that the user starts. No automatic model request and no cleanup command.

---

*Integration audit: 2026-11-14*

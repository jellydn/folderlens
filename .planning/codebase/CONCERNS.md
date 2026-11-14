# Codebase Concerns

**Analysis Date:** 2026-11-14

Fresh remap of FolderLens 0.3.7 (`main` at `0c68acb`). Each finding cites current source. Items marked **Hypothesis** were not reproduced. Checks run for this audit: `cargo fmt --check` and `cargo clippy --locked --all-targets` (both clean). The full test suite, real agent CLIs, network calls and the live app were not run.

Resolved since the 2026-11-09 audit and removed here: the Clippy warnings (fixed in `0c68acb`); missing update pagination (`native/Updater.swift:61-75` now reads up to ten pages and fails closed at the bound); the agent transport limits (now in place at `src/agent.rs:282-317`).

Fixed on 2026-11-14 after this audit: tests that reach `agent::Job::start` use a temporary `Runs` folder (`src/agent.rs`); `open` children are reaped by `prefs::open` (`src/prefs.rs`); the agent prompt, crate description and palette comment say Record choice; `ready_bytes` and the stale `allow` on `Leaf.path` are gone (`src/catalog.rs`); `docs/guide.md` links 0.3.7; the ADRs carry rename amendments, and ADR 0003 records the no-agent paths. Measure folder again now runs on the scan worker (`src/main.rs`: `ScanRequest::Measure`); it is refused while a scan runs, so it needs no separate cancellation.

## Tech Debt

**`src/main.rs` holds most of the review UI and its state logic:**
- Issue: 2,628 lines. State, scan workers, tick rules (`leaf_view`, `is_ticked`, group helpers at `src/main.rs:321-509`), every panel view and about 1,000 lines of tests (`src/main.rs:1652-2628`) share one file. `src/palette.rs` and `src/notch.rs` reach back through `super::is_ticked` / `crate::leaf_view` (`src/palette.rs:470-471`, `src/notch.rs:51`).
- Files: `src/main.rs`, `src/palette.rs`, `src/notch.rs`
- Impact: Tick and eligibility rules sit next to view code, so a rule change touches the UI file. This file has the most churn.
- Fix approach: Move the tick and eligibility helpers (`src/main.rs:321-509`) into their own module. Leave view code where it is. Do this as a behavior-preserving refactor, with the existing tests as the guard.

**Long builder lines that rustfmt does not reformat:**
- Issue: `agent_help` (`src/main.rs:1175-1214`) puts whole widget chains on single lines of up to 318 characters (`src/main.rs:1208`, `1206`). `cargo fmt --check` passes because rustfmt leaves expressions it cannot fit unformatted, without reporting an error. The same happens at `src/settings.rs:735`, `775` (498 chars), `927`, `src/settings_updates.rs:24-26`, `src/broker.rs:358` (513 chars) and `src/agent.rs:49-51`, `341`.
- Files: `src/main.rs`, `src/settings.rs`, `src/settings_updates.rs`, `src/broker.rs`, `src/agent.rs`
- Impact: CI does not check the formatting of these lines. Diffs to safety copy (for example, the access text at `src/main.rs:1208`) are hard to review.
- Fix approach: Split `agent_help` into small named child builders so that rustfmt can format them. Do not add a rustfmt option only for this.

**Duplicate candidate-total computation:**
- Issue: The same total of eligible leaves is computed in `overview` (`src/main.rs:940-943`) and in `Readings::from_review` (`src/notch.rs:50-53`).
- Files: `src/main.rs`, `src/notch.rs`
- Impact: The notch total and the Review total can diverge if one formula changes.
- Fix approach: One shared helper next to the tick rules.

**Provider schema maintenance:**
- Issue: The five adapters have different flag, mode, model and session contracts (`src/provider.rs:79-182`). OpenCode is accepted only at exactly `2.0.23` (`src/provider.rs:274`). The resume launcher checks this again (`src/provider.rs:292-310`).
- Files: `src/provider.rs`, `src/agent.rs`, `src/pi-reader.mjs`
- Impact: A CLI upgrade fails closed at start or at continuation. This is the intended behavior.
- Fix approach: Add support for a new CLI version only with stub tests and native metadata checks. Do not loosen the version gates.

## Known Bugs

**Reported Settings→tray reopening issue (cause unverified):**
- Symptoms: In beta.3, the tray stopped reopening the ledger after Settings opened. A later diagnostic run also failed its initial tray-open control (`docs/guide.md:137`).
- Files: `src/status.rs` (55 `unsafe` blocks, raw Objective-C), `src/settings.rs:204-216`
- Trigger: Native interaction only. Headless tests do not reproduce it.
- Workaround: None verified. Change the window lifecycle only after a controlled native trace.

## Security Considerations

**Selected-folder reader authority:**
- Risk: Folder data outside the eligible selection could be exposed.
- Files: `src/broker.rs:45-120`, `src/agent.rs:35-40`, `src/palette.rs:495-512`
- Current mitigation: The original roots are canonicalized when the job starts (`src/broker.rs:28-42`). Every read requires original ∩ current scope (`src/broker.rs:45-52`). Files are opened with `O_NOFOLLOW|O_NONBLOCK` and checked again with `F_GETPATH` after open and after read (`src/broker.rs:64-71`, `118-120`). Directory entries are resolved with `openat` against the open descriptor (`src/broker.rs:78-94`). The limits are 512 KiB, 2,000 lines and 1,024 entries (`src/broker.rs:110-124`, `152-164`). Turning sharing off or start permission off empties the scope on the next poll or frame (`src/palette.rs:497-499`, `src/main.rs:302-307`, `786-788`). These match `AGENTS.md` and `docs/adr/0001-scoped-agent-sessions.md`.
- Recommendations: **Hypothesis:** the scope check is path-based, so a hard link inside a selected folder to a protected file can pass. `F_GETPATH` can return the link's own path. Creating such a link needs same-user write access, so the risk is low. Do not claim inode-level isolation. The directory listing does not say when it stopped at 1,024 entries (`src/broker.rs:98-101`).

**Provider boundary differences:**
- Risk: Users may assume that the selected-only boundary is the same for every provider.
- Files: `src/provider.rs:79-91`, `src/agent.rs:651-673`
- Current mitigation: Copilot, OpenCode and Pi are `broker_only` (`src/agent.rs:326-329`), and their native ACP reads are denied. Grok and Antigravity analysis uses bounded ACP callbacks through the same reader. Their Terminal continuation uses only native plan/sandbox flags. The UI says this (`src/main.rs:1208`), and so do `README.md` and `docs/guide.md:218`.
- Recommendations: Keep this text in place. These controls are not an OS sandbox.

**Saved context prompt invites cleanup in a continued chat:**
- Risk: `prepare_ask` writes "Help me clean up… propose a safe cleanup. Ask for approval before deleting files" into `context.md` (`src/prefs.rs:411`). The analysis turn adds "Do not … delete anything" (`src/agent.rs:341`), but that instruction is sent only with the first prompt. **Hypothesis:** in a Terminal continuation (outside FolderLens, with native tools for Grok and Antigravity), the saved context can lead the model to propose deletion.
- Files: `src/prefs.rs:398-422`, `src/agent.rs:341`
- Current mitigation: Native plan/read-only flags (`src/provider.rs:79-91`). FolderLens has no delete path.
- Recommendations: Owner decision. Make the saved context read-only in intent, the same as the analysis prompt. This changes the prompt contract, so update ADR 0001 too.

**Diagnostics redaction:**
- Risk: Local paths or credentials could reach `agent.log` or the error shown in the UI.
- Files: `src/agent.rs:282-298`, `src/agent.rs:687-701`, `src/agent.rs:128-131`, `src/agent.rs:157-158`
- Current mitigation: Each stderr line is redacted before it is logged. The log is capped at 16 KiB including newlines. Error results are redacted at the worker boundary. Lines that contain `/`, `\`, `~/`, `@` or credential keywords are withheld. Updater errors use fixed text (`native/Updater.swift:536-546`).
- Recommendations: The redaction is heuristic. **Hypothesis:** a bare token with no keyword (for example, a raw base64 key) passes through. Keep the line-withholding approach. Do not log raw provider JSON.

**Run-folder retention:**
- Risk: `context.md`, `analysis.md` and session files keep shared paths and provider output on disk indefinitely.
- Files: `src/prefs.rs:425-453`, `src/agent.rs:358-360`
- Current mitigation: Each run directory is created exclusively with mode `0700` (`src/prefs.rs:432-439`). Earlier runs are never overwritten. Nothing is deleted automatically, as `AGENTS.md` requires.
- Recommendations: Removal stays manual. The parent `Runs/` directory uses the default umask (`src/prefs.rs:432`); only the run directories are `0700`.

**Publisher-verified updates:**
- Risk: Key loss or compromise; Apple platform trust.
- Files: `native/Updater.swift:176-206`, `native/Updater.swift:229-236`, `native/Updater.swift:344-427`, `scripts/sign-release.py`, `scripts/release.sh:8-11`
- Current mitigation: Ed25519 signatures over `update.json`, with tag, channel, version, expiry, asset name, size and lowercase SHA-256 checked (`native/Updater.swift:165-174`). Download URLs are built from fixed strings, not taken from metadata (`native/Updater.swift:392-395`). The archive is checked again before Reveal in Finder (`native/Updater.swift:429-444`). `requireDirectInstallation` always throws (`native/Updater.swift:229-236`). The release tag must match `Cargo.toml` (`scripts/release.sh:8-11`). CI runs `--verify-release` before publication.
- Recommendations: Builds are ad-hoc signed and not notarized (`scripts/bundle.sh:35`). Key recovery needs a separate review. `install()` (`native/Updater.swift:447-487`) can be reached only from `--self-test` (`native/Updater.swift:687-695`). This is deliberate (ADR 0002), but it is code that no production path runs.

## Performance Bottlenecks

**Per-frame and per-poll recomputation of tick state:**
- Problem: `leaf_view` (`src/main.rs:321-351`) clones a `Leaf` (four `String`s), calls `prefs::always_name` and scans `unlocked` and `kept`. `always_name` calls `expand_home` (an env lookup and an allocation) for each of the nine `ALWAYS` roots, and `path_under` allocates a `format!` string (`src/prefs.rs:337-367`). `is_ticked` calls `leaf_view` again (`src/main.rs:363-372`). One `view` makes about ten full passes over the leaves: `ask_access` (`src/main.rs:787`), `overview` (`940-943`), `ticked_bytes` from `status_line` (`623`), `cleanup_line` (three more passes, `477-509`), per-group `group_reason`/`group_on`/`group_locked` (`648-650`), `agent_help` (`1176`), `local_summary` (`1226-1228`) and `record_choice` (`1307-1312` plus `ticked_bytes`). `poll` repeats `ask_access` and the notch total every 100 ms while the panel is visible (`src/main.rs:280-311`, `src/notch.rs:50-53`).
- Files: `src/main.rs`, `src/prefs.rs`, `src/notch.rs`, `src/palette.rs`
- Cause: Stateless helpers recompute the derived view for each call.
- Improvement path: Cache the expanded `ALWAYS` roots once in a `OnceLock` (smallest change). Later, compute the leaf views and tick flags once per `view` and pass them to the helpers. **Hypothesis:** no latency was measured, so this is not a measured regression.

**Settings read from disk on every poll:**
- Problem: `poll` calls `prefs::load_from` on every tick: 100 ms while visible, 1 s while hidden (`src/main.rs:281-287`). When `settings.txt` is absent, each call also tries the legacy Shelf path (`src/prefs.rs:142-149`).
- Files: `src/main.rs`, `src/prefs.rs`
- Cause: This is how changes made in Settings reach Review (`README.md` documents "next poll").
- Improvement path: Compare the file's mtime before parsing. Keep the read off any system-command path.

**Directory listing scope checks:**
- Problem: For each of up to 1,024 entries, `Reader::allows` clones the whole current `Access` and canonicalizes every selected and blocked root (`src/broker.rs:45-52`, `src/agent.rs:35-40`). Every unticked leaf is a blocked root (`src/palette.rs:503-509`).
- Files: `src/broker.rs`, `src/agent.rs`, `src/palette.rs`
- Cause: Correctness depends on current-scope checks; nothing is cached.
- Improvement path: **Hypothesis:** this is slow only with large catalogs. Measure before changing it. Do not weaken the revalidation.

## Fragile Areas

**Native window and pointer lifecycle:**
- Files: `src/status.rs` (55 `unsafe` blocks), `src/notch.rs`
- Why fragile: Raw AppKit ABI calls and macOS 27 expanded-interface callbacks, with an older action path.
- Safe modification: Reproduce natively on both paths. Headless renders do not prove event routing (`docs/guide.md:284`).
- Test coverage: Policy and callback unit tests only.

**Agent process trees:**
- Files: `src/agent.rs:248-255`, `src/agent.rs:262-271`, `src/provider.rs:224-292`
- Why fragile: The analysis child is not put in its own process group. `Session::drop` kills and reaps only the direct child. The OpenCode version probe uses `process_group(0)` and `kill(-pid)` (`src/provider.rs:233`, `284-288`). **Hypothesis:** provider descendants (MCP helpers, Node workers) can outlive cancel or Quit until their stdin closes.
- Safe modification: If you use a process group for the analysis child, cover it with `/bin/sh` stub tests like `dropping_a_job_reaps_a_provider_that_ignores_stdin_eof` (`src/agent.rs:939-981`).
- Test coverage: Direct-child reaping and version-probe cancellation only.

**Retained installer transaction:**
- Files: `native/Updater.swift:238-258`, `native/Updater.swift:447-487`
- Why fragile: The ownership gate keeps this code unreachable. It is exercised only against fixtures in `selfTest`.
- Safe modification: Keep `requireDirectInstallation` failing closed. Any provenance mechanism needs separate review (ADR 0002).
- Test coverage: Fixture swap and rollback. Recovery after a crash after `open` returns is not proven.

## Scaling Limits

**Reader:** 512 KiB UTF-8 file, 2,000 lines, 1,024 entries, a 16 KiB request and a 4 MiB response, with one connection at a time and 250 ms socket timeouts (`src/broker.rs:110-124`, `218-220`, `285-295`, `310-315`). These are privacy limits. Do not raise them without review.

**Agent:** A ten-minute deadline, a 256 KiB report, 2 MiB protocol lines, an eight-message queue and a 16 KiB redacted log (`src/agent.rs:282-317`, `332`, `428`, `637`). OpenCode private-mode readiness is bounded to 5 s (`src/provider.rs:332-334`).

**Update discovery:** Ten pages of 100 releases, with a 2 MiB limit for each page (`native/Updater.swift:66-74`, `381-385`). Bytes are appended one at a time (`native/Updater.swift:329-342`). **Hypothesis:** a page of 100 releases with full asset lists and notes can approach 2 MiB, and the check then fails closed as a download error.

**Scan:** Up to eight measuring threads (`src/scan.rs:583`), project discovery depth 7 (`src/scan.rs:448-449`), and estimates cached by root mtime. The first live scan is slow (`AGENTS.md`).

## Dependencies at Risk

**Rune (git revision):**
- Risk: `Cargo.toml:9` pins an unreleased git revision. Changes to windows and input need native checks; compilation alone is not enough.
- Impact: Rendering, pointer and notch behavior.
- Migration plan: Renovate proposes updates (`renovate.json`). Check each update with headless shots and a native run.

**Provider CLIs and accounts:**
- Risk: Model IDs, free tiers, quotas and privacy terms change outside FolderLens (`src/agent.rs:46-53`). The listed free OpenCode models are labeled temporary.
- Impact: Fail-closed errors, or stale guidance text.
- Migration plan: Keep manual start consent and do not use paid fallbacks. Update `model_note` and `docs/guide.md:207-212` together.

## Missing Critical Features

**No delete or cleanup engine:** This is intentional (`AGENTS.md`). Record choice only sets `confirmed` (`src/main.rs:1352-1356`). Do not add deletion as an audit fix.

**Developer ID signing and notarization:** Not done. No Apple Developer account is available (ADR 0002). `scripts/bundle.sh:2` and `:35` sign ad hoc only.

**Native installation:** Every current installation is manual-update-only by design (`native/Updater.swift:229-236`).

## Documentation Drift

- `tests/pi-reader.mjs:25-27` deletes `SHELF_*` variables that `src/pi-reader.mjs` no longer reads. The current `FOLDERLENS_*` variables are restored through `process.env = before`, so the test still isolates its environment.
- Comments in `src/catalog.rs:30` and `48` still say "tray" and "Analyse".

## Test Coverage Gaps

**Wall-clock assertions in agent tests:**
- What's not tested: Cancellation and reaping are checked with fixed timing limits: `elapsed() < 1 s` / `< 2 s` (`src/agent.rs:979`, `1014`, `1177`), a 3 s deadline to push 2 MiB through `head | tr` (`src/agent.rs:737`), and an 80 ms sleep (`src/agent.rs:1173`). 0.3.6 already raised one startup wait to 10 s for load (`RELEASE_NOTES.md`, `src/agent.rs:958-960`). On a failed assertion, the temp roots are not removed (`std::fs::remove_dir_all` comes after the asserts).
- Files: `src/agent.rs`, `tests/opencode_continuation.rs`
- Risk: Intermittent CI failures on a loaded `macos-26` runner.
- Priority: Medium

**Real agents and continuation:** Stubs prove the protocol contracts only. Real model turns, tool routing during a turn and Terminal continuation are unverified (`AGENTS.md`, `RELEASE_NOTES.md` 0.3.7). Priority: High (requires owner consent for cost and data).

**Native UI:** Pointer and keyboard routing, the physical notch, multiple displays and the Settings→tray report are unverified (`src/status.rs`, `docs/guide.md:137`). Priority: High.

**Native updater:** Actual manual replacement, restart and Gatekeeper behavior are unverified. Use disposable fixtures only (`native/Updater.swift:550-711`). Priority: Medium while replacement stays blocked.

**Measurements:** Fixture scans and one live macOS test (`src/main.rs:2098`) do not prove freshness or responsiveness on every home tree. Sizes are estimates (`src/scan.rs`, `src/catalog.rs`). Priority: Medium.

---

*Concerns audit: 2026-11-14*

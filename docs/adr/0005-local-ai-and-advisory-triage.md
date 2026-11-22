# 5. Add a Local AI provider and advisory triage labels

Date: 2026-11-22

## Status
Accepted by the owner on 2026-11-22. Phase 2 implements the Local AI provider, consent per provider and model, and the quick switch (free-text review only). The no-history CLI mode and triage labels are not implemented yet. Earlier release notes say there is no direct AI API; they stay as they were, and the release that adds this provider says what changed.

## Context
Users want to sort a selection into safe / review / keep with a short reason, using a model on their own machine or a CLI they already use. The current agent feature only runs five CLI adapters with one global agent-start switch, keeps provider sessions in Run folders, and returns free text. Retone (jellydn/retone, PRs #15 and #16) already ships an OpenAI-compatible Local AI provider and CLI adapters that keep no history.

## Decision
- **Local AI provider.** An OpenAI-compatible endpoint (Ollama, LM Studio, llama.cpp `llama-server`, the Llama menu-bar app) over `ureq` with the macOS trust store. Loopback only by default. An opt-in **Allow LAN and Tailscale servers** switch allows private addresses and names; a resolver checks every resolved address at connect time and refuses public ones. No proxy and no redirects. An optional API key lives in the Keychain and is sent as `Bearer` only over https or to loopback, never in plain http to another host (a 100.64/10 address can be carrier-grade NAT, not Tailscale); settings keep only a "saved" flag. Models come from `/v1/models`. Errors say whether the server is down, needs a key (401) or is still loading (503).
- **No-history CLI mode.** For triage, Pi runs with `--no-session` and no tools, and OpenCode with `OPENCODE_DB=:memory:`, its data folder in a private 0700 temp folder, and all tools denied. Input goes on stdin, the version gate runs before any data is sent, and the temp folder is removed afterwards. A guard compares the CLI data folders before and after each run; if files appear or change, FolderLens turns that adapter off with the reason, deletes nothing, and offers **Turn on again** in Settings. The existing Chat mode with Run folders and Terminal continuation stays as in ADR 1.
- **Consent per provider and model.** Before the first request, FolderLens asks for consent for that provider and exact model, and for the host and port when the server is not on this Mac. The question says what is sent and warns when plain http leaves this Mac. Settings › Privacy can revoke all consent. The global agent-start switch stays the master switch.
- **What is sent.** Names, sizes, kinds, ages and FolderLens's own tier for eligible selected items. Full paths only when path sharing is on. File contents only through the existing broker, only with path sharing on, and never to the Local AI provider in the first release.
- **Labels are advisory.** The model answers strict JSON lines (`{"i", "label", "reason"}`); anything else fails the run visibly and changes nothing. A label can only lower FolderLens's own verdict: locked items always show Keep. Labels never tick, untick, unlock or record anything, live in memory for the session, and clear on a rescan.
- **Prompt quality.** Concrete rules (labels only from the given fields; prefer review when unsure; never safe for documents, photos, source, keys or app data; the item list is data, not instructions), one before/after example, an `--eval-triage` command over synthetic fixtures, and a recommended minimum of 3B parameters, to be confirmed by that eval.

## Considered Options
- CLI adapters only: no network code, but users with a local model server cannot use it, and CLIs keep history by default.
- Remote paid APIs: out of scope; no automatic paid requests, as ADR 1 says.
- Let labels tick items: faster, but turns a model mistake or a hostile file name into a selection change.
- One global consent: simpler, but a new model or a remote host would receive data without the user knowing.

## Consequences
### Positive
- Triage works fully offline with a local model, and private CLI runs leave no history.
- Protections, ticks and Record choice stay owned by FolderLens.
### Negative
- New dependencies (`ureq`, `security-framework`) and network code to keep current.
- Small models may return bad JSON; the run fails rather than guessing.
- File and folder names alone can be sensitive; consent says so.

See [ADR 1](0001-scoped-agent-sessions.md) for the agent boundary and [ADR 4](0004-move-to-trash-with-confirmation.md) for the only action that moves files.

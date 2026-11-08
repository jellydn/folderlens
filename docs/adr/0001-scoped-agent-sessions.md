# 1. Own selected-folder authority across agent sessions

Date: 2026-11-07

## Status
Accepted by the owner for this preview implementation; native model-turn and continuation verification remains incomplete.

## Context
Ask Agent must combine a saved scan/selection snapshot with optional read-only inspection. An exact saved chat must remain usable while Shelf is closed without reviving old folder grants. Providers have different protocols and model flags, and a free-model name is not an inference entitlement.

## Decision
`src/agent.rs` owns run/session lifecycle, private `src/provider.rs` owns provider-specific launch/resume configuration, and private `src/broker.rs` owns live reads. With path sharing enabled, live authority is original ∩ current eligible selection. Protected, held, kept, in-use and unselected paths remain excluded. Descriptor checks and bounded UTF-8/listing limits apply before returning data. Cancellation/failure/replacement/app exit closes the socket; opening Shelf again does not restore old grants.

Save context immediately, but save the successful report/session and exact resume launcher only after nonempty completion. Copilot/OpenCode use the Shelf MCP reader; Pi disables ambient tools/discovery and loads the explicit reader extension. Grok/Agy analysis has bounded ACP callbacks, but their Terminal continuation keeps native plan/sandbox controls rather than this broker. These controls are not an independent OS sandbox.

Persist model choice per agent/custom executable. Use supported `--model` arguments, or OpenCode's private per-run configuration plus ACP selection, never shared configuration. Gate OpenCode to its verified 2.0.23 schema; await private-mode readiness. Reject unavailable explicit models rather than choosing a paid fallback. Agent requests remain manual. Confirm still only records a choice and never deletes.

Apply the OpenCode version gate again in its headless saved launcher, not only during the original review. Validate the post-mode model options. Pi's continuation input guard rejects a restored model that differs from an explicit saved choice. Bounded nonblocking input/version reads honor cancellation; an atomic lifecycle transition orders cancellation before successful run finalization. Broker shutdown interrupts active socket packet reads, and directory entries are resolved relative to the opened descriptor before current-scope revalidation.

## Considered Options
- Saved context only avoids folder tooling but does not meet the approved inspection requirement.
- Direct provider filesystem/shell tools give providers ambient authority and cannot enforce live selection revocation.
- Persistent folder grants simplify offline continuation but violate the approved app-lifetime boundary.
- A shared provider configuration/model default risks affecting unrelated chats and obscures paid fallbacks.

## Consequences
### Positive
- App-owned authority is testable independently of provider prompts; changing the selection cannot widen original access.
- Exact chat continuation and per-agent choices are retained without changing shared provider setup.
### Negative
- Live inspection needs Shelf running, and provider versions require explicit capability work.
- Grok/Agy Terminal is a documented weaker selected-scope boundary; real model turns/tool routing/resume still need native verification.
- Shared context persists on disk and can reach an external inference provider; free options still have account/quota/privacy limits.

See `.planning/codebase/ARCHITECTURE.md`, `README.md` and `AGENTS.md` for the implementation map and operating rules.

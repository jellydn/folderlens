# 3. Keep review useful without an agent

Date: 2026-11-14

## Status
Accepted by the owner for FolderLens 0.3.7, which shipped the local summary. Scan controls already followed this rule.

## Context
Ask Agent is optional, manual and can fail: no CLI may be chosen, agent-start permission may be off, or a provider may exit without an answer. Before 0.3.7 the Agent help card then showed only setup text or an error. Users still need a quick reading of the current selection, and FolderLens must not start a request, change the selection or imply that space was reclaimed.

## Decision
Local scans, interval scans, presets, ticks, estimates and Record choice need neither an agent nor agent-start permission. Intervals never make model requests.

When no agent can run (`agent_bin` empty or `allow_start` off), or the last review ended in an error, `local_summary` in `src/main.rs` shows **Local summary · no AI** for a nonempty selection. It shows the item count and total, the top three groups and the three largest selected folders by estimated size, and the estimate with "review only, not reclaimed" wording. It reads only `is_ticked` and catalog sizes; it does not write state, start a process, save a run or send data. A ready, running or successful agent review hides it. "Choose an agent in Settings.", error Details, Agent settings and Retry are unchanged.

## Considered Options
- Show nothing until an agent is configured: keeps the panel short but leaves the main no-AI path without guidance.
- Fall back to another agent or a free model when the chosen one fails: violates the manual, no-automatic-paid-request rule in ADR 1.
- Generate cleanup advice from rules: risks reading as a safety judgement; the summary states sizes only.

## Consequences
### Positive
- The panel stays useful with no provider, and a failed review keeps a local reading beside its error and Retry.
- The summary is testable headlessly with fixture data (`local_summary_describes_the_selection_when_no_agent_can_answer`).
### Negative
- At the default panel height the summary is below the fold in the wide layout.
- The summary repeats some figures from the status line and footer.

See [ADR 1](0001-scoped-agent-sessions.md) for the agent boundary and `docs/guide.md` for user-facing behavior.

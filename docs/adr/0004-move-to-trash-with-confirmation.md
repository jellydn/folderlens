# 4. Move a confirmed selection to the Trash, never delete

Date: 2026-11-22

## Status
Accepted by the owner on 2026-11-22. Implemented in `src/trash.rs` and the review footer's **Move to Trash…** sheet. It is the only path in FolderLens that moves files. This ADR replaces the "never deletes" rule in [ADR 1](0001-scoped-agent-sessions.md) and AGENTS.md only for the action below. Record choice is unchanged and still never deletes.

## Context
Users review a selection, may ask an agent to explain it, and then want to act on it without leaving FolderLens. Until now FolderLens recorded the choice and stopped. Permanent deletion cannot be undone, the AI can be wrong or manipulated by file names, and scans can be out of date when the user acts.

## Decision
Add one destructive action, **Move to Trash…**, with these rules:

- **Trash only.** Items move with `NSFileManager trashItemAtURL`. FolderLens never calls `rm`, `unlink` or `remove_dir_all` on user files and never empties the Trash. The user can drag items back out of the Trash in Finder (Finder's Put Back works only for the first item of a move: when `trashItemAtURL` calls follow each other with no pause, Finder does not record where the later items came from).
- **The user starts it.** Only a direct press on the action in FolderLens starts it. No agent, provider, tool, MCP route, prompt answer, schedule or interval can start it or pass items to it. AI labels ([ADR 5](0005-local-ai-and-advisory-triage.md)) never tick items and never reach this action.
- **FolderLens confirms it.** A sheet that FolderLens draws lists every item, its estimated size and the total. The destructive button is not the default, so ⏎ and Escape do not trash anything. Escape or Cancel closes the sheet and changes nothing.
- **Protections win.** ALWAYS roots (Unlock does not apply), Keep safe exclusions, holds, in-use paths and unticked leaves are left out of the list and cannot be added back. Only eligible ticked leaves reach the sheet.
- **Recheck before each move.** Just before each item moves, open it with `O_NOFOLLOW`, compare its `F_GETPATH` with the reviewed path, and check that it is still eligible and its size estimate has not changed class. An item that fails a check is skipped and reported, and is not moved.
- **Stop on failure.** The first error stops the run (the run is short and synchronous, so it has no cancel). The result lists what moved to the Trash and what did not. Nothing is retried and nothing is deleted permanently.
- **After it.** A rescan of the affected roots starts. The status says "moved to Trash", never "reclaimed" or "freed", and sizes stay estimates.
- **Tests** use temp fixtures only, never the real home folder: cancel changes nothing, a locked item never reaches the trash call, a symlink swap is refused, and a failure partway reports a partial result.

## Considered Options
- Keep record-only: safest, but users leave FolderLens and delete by hand without the protections.
- Permanent delete: frees space at once, but cannot be undone and turns an AI or scan mistake into data loss.
- Let the agent run the cleanup: gives a provider ambient write authority and makes prompt injection through names dangerous.
- Reveal in Finder only: keeps FolderLens passive but loses the protection checks at the moment of action.

## Consequences
### Positive
- Users can act on a reviewed selection with the same protections that shaped it, and can undo it from the Trash.
- The AI stays advisory; one app-owned code path owns every file move.
### Negative
- The "FolderLens never deletes files" product claim changes. README, the guide, the architecture map, UI text and release notes must say "moves to the Trash after confirmation" when the action ships. Earlier release notes stay as they were.
- Space is not freed until the user empties the Trash, and some volumes have no Trash, so items there are skipped and reported.
- New tests replace the ones that asserted no delete path exists; Record choice keeps its catalog-unchanged tests.

See [ADR 1](0001-scoped-agent-sessions.md) for agent authority and [ADR 5](0005-local-ai-and-advisory-triage.md) for AI labels.

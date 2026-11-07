## Shelf 0.1.0

First public release for Apple Silicon Macs running macOS 14 or later.

- Menu-bar cleanup review with live memory, CPU, GPU, and disk-space readings.
- Background disk scans and pointer-outside panel dismissal, including macOS 27 expanded-interface support.
- Stable Shelf.app bundle, Settings, protected paths, manual selection, and Quit Shelf.
- Manual Ask Agent analysis for configured Grok and Antigravity CLI, saved selection context, progress/cancellation, inline error recovery, and exact-session Terminal launchers.
- Optional read-only selected-folder callbacks; errors and analysis never change your selection.

**Confirm only records your choice. Shelf does not delete files or run cleanup.** There are no automatic paid agent requests.

### Install

Download the DMG, open it, and drag Shelf.app to Applications. The ZIP contains the same app. Verify downloads against SHA256SUMS.

Homebrew: `brew install --cask jellydn/tap/shelf`.

These builds are ad-hoc signed, not Developer ID signed or notarized. macOS may require **System Settings → Privacy & Security → Open Anyway** after the first launch attempt. Intel builds are not included.

### Ask Agent limits

Only Grok and Antigravity CLI have in-app adapters; unsupported providers fail explicitly. Harmless stubs verify protocol handling, selection/privacy gates, read-only callbacks, failures, and saved-session launch arguments. Antigravity initialization/session creation was checked without a model prompt. Real model analysis, native folder-tool routing, and native chat continuation have not been verified end to end.

On the development Mac, Grok refuses its read-only sandbox because a runtime socket symlink resolves outside its deny list. Shelf reports this error instead of disabling the sandbox. Terminal continuation uses the CLI's own plan/sandbox controls, not Shelf's ACP read filter. Destructive work always needs separate approval.

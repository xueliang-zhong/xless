# xless Run Memory

- Timestamp UTC: 2026-04-05T17:27:55Z
- Commit: 92ce66d

## Decisions

- Kept the pager architecture intact and focused on the highest-value gaps: ANSI sanitization, search repetition semantics, and editor handoff safety.
- Expanded ANSI parsing to preserve more real-world color output without allowing arbitrary terminal escape execution.
- Used a raw-mode guard around editor launch so failures cannot leave the terminal in a broken state.
- Parsed editor commands with shell-style quoting so common configurations like `nvim -u 'NORC profile'` work naturally.

## Failed Ideas

- A minimal CSI-only parser was not enough; it still left non-SGR escape traffic ambiguous, so I moved to explicit stripping of OSC/DCS-style sequences.
- Relying on whitespace splitting for editor commands would have broken quoted arguments and reduced compatibility.

## Metrics

- Test suite: 9 unit tests plus 1 CLI integration test passing.
- Formatting: `cargo fmt --check` passes.

## Reusable Lessons

- For terminal-facing tools, treat ANSI parsing as a sanitizer first and a renderer second.
- If a command launches external tools inside raw mode, use a guard that restores terminal state on every exit path.
- Search-repeat behavior needs explicit tests for both same-direction and opposite-direction repetition; otherwise `n`/`N` regressions are easy to miss.

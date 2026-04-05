# xless Run Memory

- Timestamp UTC: 2026-04-05T17:38:09Z
- Commit: 7a0fe14

## Decisions

- Kept the pager architecture intact and fixed the next highest-value gap: row accounting for ANSI-heavy input.
- Made layout math ignore ANSI escape scaffolding unless raw control mode is enabled, so `-F` and row-based scrolling track visible content.
- Hardened status-bar truncation to cut on character boundaries instead of byte boundaries.
- Restricted the terminal-session cleanup path so `LeaveAlternateScreen` only runs if the alternate screen was actually entered.

## Failed Ideas

- Leaving row estimation based on raw text would have kept the code smaller, but it miscounted colored `git` and `xcat` output and undermined `-F`.
- Using `String::truncate` for the status bar is unsafe for UTF-8 boundaries, so it had to be replaced with width-aware truncation.

## Metrics

- `cargo test --quiet`: 15 unit tests plus 1 CLI integration test passing.
- `cargo fmt --all --check`: clean.

## Reusable Lessons

- Search-only ANSI sanitization is not enough; paging math has to use the same visible-text model or navigation parity breaks.
- If UI text can contain non-ASCII names, truncate by display width rather than bytes.
- Terminal cleanup should be conditioned on what was actually entered, not on a blanket exit sequence.

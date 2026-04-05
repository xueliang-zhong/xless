# xless Run Memory

- Timestamp UTC: 2026-04-05T17:34:52Z
- Commit: 8ca4515

## Decisions

- Wired `-p/--pattern` into pager startup so it positions on the first visible match before raw mode begins.
- Kept interactive search semantics intact by splitting forward and backward scan helpers instead of reusing one search loop for every case.
- Fixed backward wrap search to scan from the end of the file downward, which matches less-style expectations better than the previous ascending wrap scan.
- Kept the feature slice small and testable: pager startup wiring, directional search helpers, and docs updates.

## Failed Ideas

- Reusing the interactive forward-search loop for startup search skipped the first line, so I replaced it with an inclusive startup scan.
- Leaving backward wrap as an ascending scan would sometimes land on the earliest matching line instead of the last one.

## Metrics

- `cargo test --quiet`: 13 unit tests plus 1 CLI integration test passing.
- `cargo fmt --all`: clean.

## Reusable Lessons

- Startup search and interactive search often need different inclusive/exclusive boundaries even when they share the same regex engine.
- Less-style backward wrap should scan from the tail of the buffer, not from the start, or repeat-search parity breaks subtly.
- When a feature is CLI-visible, put the docs in `docs/usage.md` and keep the README to the short pitch plus one example.

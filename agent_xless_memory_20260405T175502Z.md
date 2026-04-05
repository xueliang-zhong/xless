# xless Run Memory

- Timestamp UTC: 2026-04-05T17:55:02Z
- Commit: a7771b6

## Decisions

- Added less-style blank-line squeezing as a document-layer transformation so line counts, search, and navigation all see the same compacted view.
- Reused existing scroll helpers for less-style control keys instead of introducing a separate movement path.
- Kept the feature slice narrow: config/CLI wiring, document compaction, pager bindings, tests, and docs.

## Failed Ideas

- Squeezing blanks in the renderer would have been simpler to code, but it would have left search and status math working against a different line model.
- Treating blankness as a raw-byte check would have missed ANSI-only blank lines, so the document loader now strips ANSI before deciding whether a line is empty.

## Metrics

- `cargo test --quiet`: 25 unit tests plus 1 CLI integration test passing.
- `cargo fmt` completed cleanly.

## Reusable Lessons

- If a feature changes the visible line set, implement it before navigation/search rather than after rendering.
- Less-style key parity is easiest to maintain when the bindings call the same motion helpers as the existing keys.
- Config additions should be wired through CLI, file loading, docs, and tests in the same pass so `--dump-config` stays trustworthy.

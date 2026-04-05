# xless Run Memory

- Timestamp UTC: 2026-04-05T19:00:07Z
- Generation: 23

## Decisions

- Normalized document line endings at load time so LF, CRLF, and bare CR files all share the same line model for rendering, search, scrolling, and refresh.
- Accepted `\r` and `\n` as submit variants in pager prompt handlers so macOS zsh and Linux bash TTY paths do not depend on one terminal mapping for Enter.
- Documented the mixed line-ending behavior in the usage guide and README so the change is discoverable.

## Failed Ideas

- Leaving line-ending handling to the renderer would have kept `\r` in the data model and preserved the hidden row/accounting mismatch.
- Relying only on `KeyCode::Enter` would have kept prompt submission fragile across terminal paths that surface carriage return or newline as literal chars.

## Metrics

- `cargo test --quiet`: 46 unit tests and 4 CLI tests passing.
- `cargo clippy --quiet --all-targets -- -D warnings`: passing.
- End-to-end smoke: `timeout 120 bash -lc 'printf q | script -qfec "./target/release/xless README.md" /dev/null'` exited cleanly.

## Reusable Lessons

- Normalize line terminators once at ingestion, not during render, so every downstream feature reasons about the same byte ranges and line counts.
- Prompt handlers for terminal tools should accept the common submit variants explicitly, because cross-shell and cross-PTY behavior is not always uniform.
- A small regression test for CRLF and bare CR lines catches a whole class of redraw and navigation bugs before they reach a TTY board.

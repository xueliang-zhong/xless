# xless Run Memory

- Timestamp UTC: 2026-04-05T18:33:33Z
- Commit: 3b305e5

## Decisions

- Added `&` filtering as a less-style visible-line filter and kept the implementation centered on `DocumentSet` rebuilds instead of ad hoc render-time conditionals.
- Preserved multi-file headers when a filter still leaves visible lines in a document, so filtered views keep enough context for review workflows.
- Kept follow-mode and filter transitions stable by remapping `top_line` to the same logical document line after both filter changes and reloads.

## Failed Ideas

- Mutating render output directly would have been faster to sketch, but it would have spread filter semantics across rendering, motion, and reload code.
- Clearing the active status message immediately after applying a filter would have hidden the useful confirmation that the view changed.

## Metrics

- `cargo test --quiet`: 40 unit tests plus 3 CLI integration tests passing.
- `cargo clippy --quiet --all-targets -- -D warnings`: passing.
- End-to-end smoke test: `timeout 10s` pty-backed `cargo run --quiet -- <tempfile>` exited cleanly after sending `q`.

## Reusable Lessons

- When a pager feature changes what is visible, rebuild the line view from the source documents and keep cursor remapping in one place.
- Filter and reload flows should preserve logical line identity, not just numeric screen position, or follow mode becomes jumpy.

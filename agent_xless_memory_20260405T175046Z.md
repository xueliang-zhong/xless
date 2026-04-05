# xless Run Memory

- Timestamp UTC: 2026-04-05T17:50:46Z
- Commit: dde5092

## Decisions

- Added less-style startup command support by stripping leading `+...` args before file loading, instead of reworking the clap positional parser.
- Kept explicit `-p/--pattern` as a first-class startup search and appended it after positional startup commands so explicit pattern selection still wins.
- Cached line-number width in `DocumentSet` and cached compiled search state in `Pager` to reduce repeated redraw/search work.
- Preserved the existing ANSI-safe rendering path and only changed startup/navigation plumbing around it.

## Failed Ideas

- A full CLI parser rewrite for startup commands was unnecessary; it would have increased the surface area without improving behavior.
- Recomputing search regexes or line-number widths on every repeat/redraw would have been simpler to read, but it wasted work in the hot path.

## Metrics

- `cargo test --quiet`: 23 unit tests plus 1 CLI integration test passing.
- `cargo fmt --check`: clean after formatting.
- Feature commit: `dde5092`.

## Reusable Lessons

- For less-style startup syntax, treat leading `+...` entries as a small pre-file parse step and keep the rest of the positional file handling unchanged.
- If a startup command overlaps with an explicit CLI option, make the explicit option the final startup action so user intent remains predictable.
- Cache stable layout metadata in the document model when redraws would otherwise rescan the same data every frame.

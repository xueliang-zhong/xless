# xless Run Memory

- Timestamp UTC: 2026-04-05T17:45:12Z

## Decisions

- Kept the change narrow and made `highlight` a real configuration and CLI control instead of widening the pager architecture.
- Left ANSI SGR pass-through untouched and only gated syntax coloring for plain source text.
- Added a renderer test that uses a `.rs` temp file so syntect resolves the intended language path.
- Documented the plain-rendering override in both configuration and usage docs.

## Failed Ideas

- Testing the syntax-color path with an extensionless temp file failed because syntect treated it as plain text.
- Treating `highlight` as a no-op config field would have preserved the bug and left the docs misleading.

## Metrics

- `cargo test --quiet`: 19 unit tests plus 1 CLI integration test passing.
- `cargo fmt --all`: clean.

## Reusable Lessons

- Config flags should be wired all the way into the render path, not just parsed and documented.
- If a test depends on syntax detection, give the fixture a real extension so the branch under test is actually exercised.
- Keep syntax coloring separate from ANSI escape preservation so tool output stays color-safe while source rendering remains configurable.

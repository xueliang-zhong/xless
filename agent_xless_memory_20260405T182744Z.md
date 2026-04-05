# xless Run Memory

- Timestamp UTC: 2026-04-05T18:27:44Z
- Commit: 17aaa8857102e6973a937b13813a21881e094472

## Decisions

- Exposed the existing tab-width machinery through `-x` / `--tabs` instead of adding a parallel layout path.
- Kept tab stop handling in the current renderer and row-accounting helpers so visual output and navigation continue to agree.
- Backed the new flag with a config-dump integration test and a renderer unit test so the CLI and paint path are both covered.

## Failed Ideas

- Reworking tab handling into a separate document model would have duplicated logic already shared by rendering and scrolling.
- Only documenting the option without a test would have left the config-to-render path easy to regress later.

## Metrics

- `cargo fmt --all`: clean.
- `cargo test --quiet`: 37 unit tests plus 3 CLI integration tests passing.
- `cargo clippy --quiet --all-targets -- -D warnings`: passing.

## Reusable Lessons

- If a config field already drives the hot path, expose it through the CLI before inventing new state.
- Pair a CLI override with both a config-dump test and a render-path test so the option is validated end to end.

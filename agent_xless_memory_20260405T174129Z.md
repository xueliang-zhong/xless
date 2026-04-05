# xless Run Memory

- Timestamp UTC: 2026-04-05T17:41:29Z

## Decisions

- Kept the pager architecture intact and added a small less-parity slice instead of a broader viewport refactor.
- Made `G` land on the first line of the last screenful so bottom navigation behaves more like `less`.
- Added `m` and `'` mark navigation with in-memory line positions for quick return points during review and log inspection.
- Wired the new behavior through tests first, then updated docs and the short help text so the feature is discoverable.

## Failed Ideas

- Leaving `G` pinned to the final line was simpler, but it wasted the screen and felt less like the real pager.
- A full mark ring or cross-file bookmark system would be more powerful, but it was unnecessary complexity for this slice.

## Metrics

- `cargo test --quiet`: 18 unit tests plus 1 CLI integration test passing.
- `cargo fmt --all --check`: clean.

## Reusable Lessons

- Bottom-of-file behavior is best modeled as a screenful calculation from the tail, not as a raw last-line jump.
- Single-key navigation additions are easiest to land safely when they are backed by tiny pure helpers and explicit status text.
- If a feature changes user-visible key bindings, update the docs in the same change so the pager remains self-explaining.

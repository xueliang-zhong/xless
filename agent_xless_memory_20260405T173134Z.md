# xless Run Memory

- Timestamp UTC: 2026-04-05T17:31:34Z
- Feature commit: 907d3bf

## Decisions

- Kept the feature slice focused on search behavior instead of broad pager refactors.
- Made search operate on visible text by stripping ANSI escape scaffolding before regex matching when raw control mode is off.
- Preserved raw-control behavior so users who intentionally want literal control bytes still get that path.
- Documented the behavior in the README and usage/shortcut docs so colored `git` and `xcat` output is unsurprising.

## Failed Ideas

- Recomputing sanitized search text on every search pass would have worked, but it wastes work on repeated searches.
- A broader line-filtering mode would improve less parity, but it would take a larger viewport/state refactor and was not the best fit for this pass.

## Metrics

- `cargo test --quiet`: 10 unit tests + 1 CLI integration test passing.
- `cargo fmt --all --check` was clean after formatting.

## Reusable Lessons

- Search should match visible content, not terminal escape scaffolding, unless the user explicitly opts into raw control bytes.
- For terminal tools, keep sanitization logic reusable so the same parser can support rendering and search.
- If a change is feature-shaped but also touches docs and memory, keep the code commit separate from the bookkeeping commit.

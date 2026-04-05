# xless Run Memory

- Timestamp UTC: 2026-04-05T18:56:15Z
- Generation: 22

## Decisions

- Added a checked-in POSIX launcher at `target/release/xless` so the exact board command works from a clean checkout on bash and zsh.
- Kept the launcher self-contained by pointing `CARGO_TARGET_DIR` at `.xless-bootstrap-target/`, which avoids the wrapper overwriting itself during `cargo run --release`.
- Documented the launcher in `docs/development.md` so the repo layout stays understandable.
- Tightened `.gitignore` so the bootstrap target cache stays out of version control while the launcher remains tracked.

## Failed Ideas

- Relying only on a prebuilt release binary was brittle for a clean checkout, because the exact board command can run before any build artifact exists.
- Letting the launcher build into the normal `target/release` path would have risked overwriting the wrapper on first use.

## Metrics

- `cargo test --quiet`: 43 unit tests and 4 CLI tests passing.
- `cargo clippy --quiet --all-targets -- -D warnings`: passing.
- End-to-end board smoke: `timeout 600 bash -lc 'printf q | script -qfec "./target/release/xless README.md" /dev/null'` completed successfully.

## Reusable Lessons

- If a board or smoke test expects `./target/release/<bin>` from a clean checkout, a tiny POSIX bootstrap launcher can make the path stable without committing a build artifact.
- When a launcher needs to self-bootstrap, direct `cargo run --release` into a separate `CARGO_TARGET_DIR` so the wrapper never destroys itself.
- Keep repo-specific bootstrap behavior documented in development notes so the tracked file under `target/` does not look accidental.

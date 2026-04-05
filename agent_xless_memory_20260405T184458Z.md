# xless Run Memory

- Timestamp UTC: 2026-04-05T18:44:58Z
- Feature commit: cb469af

## Decisions

- Switched the `!` command path from word-splitting and direct execution to invoking the user's shell so pipes, redirects, globbing, and shell variables work the way `bash` and `zsh` users expect.
- Kept editor execution on direct `Command` construction so `v` remains explicit and shell-interpolation-free.
- Added a regression test that proves shell redirection and pager context variables survive the `!` launch path.
- Updated the usage, shortcuts, configuration, and README docs so the shell behavior is discoverable.

## Failed Ideas

- Using `shell_words` plus direct process execution was too narrow for less-style shell escapes; it could not handle pipes or redirection.
- Trying to fake shell behavior piecemeal would have duplicated shell parsing logic and still missed common user workflows.

## Metrics

- `cargo test --quiet`: 43 tests passing.
- `cargo clippy --quiet --all-targets -- -D warnings`: passing.
- `cargo fmt --all`: clean.
- End-to-end smoke: `timeout 10s cargo run --quiet -- --dump-config >/dev/null` completed successfully.

## Reusable Lessons

- Shell escape prompts should delegate to a real shell if you want parity with less and with common `bash`/`zsh` usage.
- Keep editor invocation separate from shell execution so the safer direct-exec path stays available where shell expansion is not needed.
- Regression tests for terminal tools should cover both environment context and shell syntax, not just simple command execution.

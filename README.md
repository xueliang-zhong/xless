# xless

`xless` is a Rust pager with less-style navigation, safe ANSI color pass-through, horizontal scrolling for chopped lines, mark/jump navigation, multi-file jumps, blank-line squeezing, shell escapes, configurable tab stops, line-ending normalization, and syntax highlighting for common source files.

Use it for code review, `git diff`, log browsing, and quick file inspection.

```bash
cargo run -- src/main.rs
git diff --color=always | cargo run --
xless -p main src/main.rs
```

Key features:

- Syntax highlighting for recognized file types.
- Safe ANSI color support by default for tool output like `git`, `xcat`, and `less -R` output, including 16-color, 256-color, and truecolor SGR sequences.
- Less-style navigation, counted motions, startup commands like `+42`, `+/pattern`, `+G`, and `+F`, startup search with `-p/--pattern`, incremental search over visible text with less-compatible `-i` / `-I` case handling, `&` line filtering, `!` shell commands that run through your shell, tab stop control with `-x` / `--tabs`, mark/jump keys, `:n` / `:p` file jumps, horizontal scrolling for chopped long lines, blank-line squeezing, and editor handoff.
- Configuration via `~/.xless/config.toml`.

See `docs/usage.md` for examples, `docs/configuration.md` for config options, and `docs/shortcuts.md` for key bindings.

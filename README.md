# xless

`xless` is a Rust pager with less-style navigation, default-safe ANSI color pass-through, and syntax highlighting for common source files.

Use it for code review, `git diff`, log browsing, and quick file inspection:

```bash
cargo run -- src/main.rs
git diff --color=always | cargo run --
```

Key features:

- Syntax highlighting for recognized file types.
- Safe ANSI color support by default for tool output like `git` and `xcat`.
- Less-style navigation, incremental search, and editor handoff.
- Configuration via `~/.xless/config.toml`.

See `docs/usage.md` for examples, `docs/configuration.md` for config options, and `docs/shortcuts.md` for key bindings.


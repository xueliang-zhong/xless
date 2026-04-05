# Usage

## Basic Invocation

Open one or more files:

```bash
xless src/main.rs
xless Cargo.toml README.md
```

Read from standard input:

```bash
git diff --color=always | xless
xcat src/lib.rs | xless
```

## Interactive Controls

- `j` / `Down` move down one line.
- `k` / `Up` move up one line.
- `f` / `Space` / `PageDown` scroll forward.
- `b` / `PageUp` scroll backward.
- `/` search forward.
- `?` search backward.
- `n` and `N` repeat the last search.
- `v` open the current file in your editor.
- `r` reload a file from disk.
- `q` quit.

## Git, Vim, and fzf

- `git diff --color=always | xless` preserves SGR colors while still allowing xless to render syntax and status information.
- Press `v` to jump into `vim`, `nvim`, or the editor configured in `~/.xless/config.toml`.
- Use xless after filtering with `fzf`, for example:

```bash
fzf --multi | xargs -r xless
```

## One-Screen Exit

If you want xless to quit automatically when the content fits on one screen, use:

```bash
xless -F file.txt
```


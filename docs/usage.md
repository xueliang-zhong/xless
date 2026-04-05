# Usage

## Basic Invocation

Open one or more files:

```bash
xless src/main.rs
xless Cargo.toml README.md
```

Start at the first match for a startup pattern:

```bash
xless -p main src/main.rs
xless -p 'fn main' Cargo.toml README.md
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
- `m` sets a mark and `'` jumps back to it.
- `v` open the current file in your editor.
- `r` reload a file from disk.
- `q` quit.

## Git, Vim, and fzf

- `git diff --color=always | xless` preserves SGR colors while still allowing xless to render syntax and status information.
- `git`, `xcat`, and `less -R` output keep ANSI colors, including 256-color and truecolor SGR sequences.
- Search ignores the escape scaffolding around colored spans, so patterns match the text you actually see.
- Row-based motion and `-F` follow visible text instead of ANSI scaffolding, so colored output does not throw off screen-fit or page scrolling.
- `--no-highlight` is useful when you want raw source text without syntax coloring, while still keeping ANSI colors from upstream tools.
- `-p` / `--pattern` starts the pager on the first matching line before you begin interacting with it.
- `G` jumps to the last screenful of content instead of leaving the final line pinned at the top.
- `m` and `'` give you a fast return point when you are comparing code, logs, or filtered `fzf` output.
- Press `v` to jump into `vim`, `nvim`, or the editor configured in `~/.xless/config.toml`.
- The `editor` setting supports quoted arguments, so commands like `nvim -u 'NORC profile'` are valid.
- Use xless after filtering with `fzf`, for example:

```bash
fzf --multi | xargs -r xless
```

## One-Screen Exit

If you want xless to quit automatically when the content fits on one screen, use:

```bash
xless -F file.txt
```

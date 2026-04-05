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

Use less-style startup commands before the file list:

```bash
xless +42 src/main.rs
xless +/main src/main.rs
xless +?todo src/lib.rs
xless +G large.log
xless +F server.log
```

Read from standard input:

```bash
git diff --color=always | xless
xcat src/lib.rs | xless
```

## Interactive Controls

- Numeric prefixes like `5j`, `2G`, or `10d` apply to the next motion command.
- `j` / `Down` move down one line.
- `k` / `Up` move up one line.
- `f` / `Space` / `PageDown` scroll forward.
- `b` / `PageUp` scroll backward.
- `/` search forward.
- `?` search backward.
- `n` and `N` repeat the last search.
- `m` marks the first visible line, `M` marks the last visible line, and `'` jumps back to a saved mark.
- `v` open the current file in your editor.
- `r` or `R` reload a file from disk.
- `Left` and `Right` scroll horizontally when `-S` / `--chop-long-lines` is enabled, and `Home` returns to the left edge.
- `q` quit.
- `Ctrl-E` and `Ctrl-Y` move one line forward or backward; `Ctrl-F` / `Ctrl-B` page forward or backward; `Ctrl-D` / `Ctrl-U` move by half pages.

## Git, Vim, and fzf

- `git diff --color=always | xless` preserves SGR colors while still allowing xless to render syntax and status information.
- `git`, `xcat`, and `less -R` output keep ANSI colors, including 256-color and truecolor SGR sequences.
- Search ignores the escape scaffolding around colored spans, so patterns match the text you actually see.
- Row-based motion and `-F` follow visible text instead of ANSI scaffolding, so colored output does not throw off screen-fit or page scrolling.
- When `-S` / `--chop-long-lines` is active, left/right arrows let you pan across long lines without losing ANSI colors or syntax highlighting.
- `--no-highlight` is useful when you want raw source text without syntax coloring, while still keeping ANSI colors from upstream tools.
- `-s` / `--squeeze-blank-lines` collapses consecutive blank lines like less, which is useful for long logs and heavily separated output.
- `-p` / `--pattern` starts the pager on the first matching line before you begin interacting with it.
- Leading `+` startup commands follow the less convention for jumping to a line, searching forward or backward, jumping to the bottom, or starting in follow mode.
- `G` jumps to the last screenful of content instead of leaving the final line pinned at the top.
- `m`, `M`, and `'` give you fast return points when you are comparing code, logs, or filtered `fzf` output.
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

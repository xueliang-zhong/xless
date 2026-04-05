# Configuration

`xless` reads `~/.xless/config.toml` by default.

Example:

```toml
line_numbers = true
raw_control_chars = false
chop_long_lines = false
quit_if_one_screen = true
no_init = false
follow = false
ignore_case = false
wrap_search = true
highlight = true
status_bar = true
tab_width = 4
theme = "base16-ocean.dark"
editor = "vim"
```

## Fields

- `line_numbers`: show line numbers on the left.
- `raw_control_chars`: pass raw control characters through without sanitizing them.
- `chop_long_lines`: truncate long lines instead of wrapping.
- `quit_if_one_screen`: exit immediately when the file fits in the terminal.
- `no_init`: skip the alternate screen.
- `follow`: keep reloading appended file contents.
- `ignore_case`: use case-insensitive search.
- `wrap_search`: search wraps from end to start and vice versa.
- `highlight`: enable syntax highlighting.
- `status_bar`: show a status bar.
- `tab_width`: tab stop width.
- `theme`: syntect theme name.
- `editor`: editor command used by the `v` key. It is parsed with shell-style quoting, so values such as `nvim -u 'NORC profile'` work as expected.

## Color Handling

`xless` keeps ANSI SGR sequences by default when they appear in tool output. That includes standard 8-color sequences, bright variants, 256-color indexes, and truecolor foreground/background settings.

Non-SGR escape sequences are stripped rather than executed, which keeps terminal control traffic from leaking through by accident.

## CLI Overrides

Command-line flags override config file values:

```bash
xless --line-numbers --theme InspiredGitHub src/main.rs
```

To inspect the merged configuration without opening the pager:

```bash
xless --dump-config
```

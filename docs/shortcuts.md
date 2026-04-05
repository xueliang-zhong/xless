# Shortcuts

- Numeric prefixes such as `5j`, `2G`, and `10d` apply to the next motion command.
- `q` quit.
- `j`, `k`, `Up`, `Down` move one line.
- `f`, `b` page forward and backward.
- `d`, `u` move by half pages.
- `g`, `G` jump to the top or bottom.
- `/`, `?` start forward or backward search.
- `n`, `N` repeat the last search.
- `m`, `M`, `'` set and jump to a line mark.
- `v` open the current file in the configured editor.
- `r`, `R` reload the current file from disk.
- `Left` and `Right` pan across chopped long lines, and `Home` returns to the start of the line.
- `h` show the built-in help hint.
- `Ctrl-E` and `Ctrl-Y` move a single line down or up; `Ctrl-F` / `Ctrl-B` page forward or back; `Ctrl-D` / `Ctrl-U` move by half pages.

The pager keeps ANSI SGR colors from tools like `git`, `less -R`, and `xcat` while stripping unsafe terminal control sequences by default.
Search runs over visible text, so `n` and `N` keep working even when the source line contains color escapes.
`G` now lands on the last screenful of the file instead of pinning the final line at the top.
`M` marks the last visible line on screen, which is useful when comparing wrapped or multi-line output.
`-s` squeezes repeated blank lines to keep dense logs easier to scan.

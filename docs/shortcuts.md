# Shortcuts

- `q` quit.
- `j`, `k`, `Up`, `Down` move one line.
- `f`, `b` page forward and backward.
- `d`, `u` move by half pages.
- `g`, `G` jump to the top or bottom.
- `/`, `?` start forward or backward search.
- `n`, `N` repeat the last search.
- `m`, `'` set and jump to a line mark.
- `v` open the current file in the configured editor.
- `r` reload the current file from disk.
- `h` show the built-in help hint.

The pager keeps ANSI SGR colors from tools like `git`, `less -R`, and `xcat` while stripping unsafe terminal control sequences by default.
Search runs over visible text, so `n` and `N` keep working even when the source line contains color escapes.
`G` now lands on the last screenful of the file instead of pinning the final line at the top.

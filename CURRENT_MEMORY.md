# Current Memory

- `xless` now uses a safe default ANSI path: it preserves SGR color sequences from tools like `git` and `xcat`, but strips other terminal control sequences unless raw control mode is enabled.
- File-backed input is memory-mapped when possible; standard input is buffered.
- Search wraps by default, and `v` should launch the configured editor at the current file and line.
- ANSI handling is now broader and safer: 16-color, 256-color, and truecolor SGR are preserved, while OSC/DCS-style escape traffic is stripped.
- Editor commands should be parsed with shell-style quoting and protected by a raw-mode guard so terminal state is restored on failure.
- Search should operate on visible text by default, so ANSI escape scaffolding from colored tools does not block matches unless raw control mode is enabled.
- Startup `-p/--pattern` search should run before raw-mode entry and include the first line, while interactive forward search still skips the current line.
- Backward wrap search must scan from the end of the file backward; ascending wrap scans can land on the wrong match.
- Screen-fit and row-based motion need to ignore ANSI scaffolding as well, or colored `git`/`xcat` output will miscount rows even when search already works on visible text.

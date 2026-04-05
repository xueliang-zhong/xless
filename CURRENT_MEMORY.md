# Current Memory

- `xless` now uses a safe default ANSI path: it preserves SGR color sequences from tools like `git` and `xcat`, but strips other terminal control sequences unless raw control mode is enabled.
- File-backed input is memory-mapped when possible; standard input is buffered.
- Search wraps by default, and `v` should launch the configured editor at the current file and line.
- ANSI handling is now broader and safer: 16-color, 256-color, and truecolor SGR are preserved, while OSC/DCS-style escape traffic is stripped.
- Editor commands should be parsed with shell-style quoting and protected by a raw-mode guard so terminal state is restored on failure.

# Development

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

## Formatting

```bash
cargo fmt
```

## Make

The repository includes a small `Makefile` for common workflows:

```bash
make build
make test
make fmt
make fmt-check
make clippy
make smoke
```

`make smoke` runs the binary end to end with `--dump-config`, which is a quick non-interactive check that the CLI and config loader still work together.

## Notes

- The pager core is split into document loading, syntax highlighting, rendering, and input handling.
- Syntax highlighting uses `syntect` so common programming and markup languages work out of the box.
- Files are memory-mapped when possible for better large-file performance; standard input is buffered in memory.

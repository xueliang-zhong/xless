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

## Notes

- The pager core is split into document loading, syntax highlighting, rendering, and input handling.
- Syntax highlighting uses `syntect` so common programming and markup languages work out of the box.
- Files are memory-mapped when possible for better large-file performance; standard input is buffered in memory.


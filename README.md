# logz-rs

A small Rust CLI for tailing, filtering, and summarising log files.
Streams lines one at a time so memory stays flat regardless of input size,
and natively understands gzipped logs and directory trees.

## Build

```bash
cargo build --release
# binary at: target/release/logz-rs
```

Or run from source:

```bash
cargo run -- <subcommand> [args]
```

## Commands

`logz-rs` has three subcommands: `tail`, `analyze`, `stats`. They share a
common set of filter flags.

### `analyze` — filter and emit log lines

Reads from a file or directory (recursive), applies filters, and writes the
matching raw lines to stdout (or to `--out`). Plays well with stdin when
called without `--path`.

```bash
logz-rs analyze --path app.log --level ERROR
logz-rs analyze --path logs/ --regex 'timeout|refused' --out errors.txt
logz-rs analyze --path access.log.gz --match '/api/v2/'
```

Flags:

| Flag        | Description                                                     |
| ----------- | --------------------------------------------------------------- |
| `--path`    | File or directory to read. If omitted, reads path from stdin.   |
| `--out`     | File to write filtered lines to. Defaults to stdout.            |

Progress (`processed N filtered log lines`) is written to **stderr**, so
piping `--out`-less output stays clean.

### `stats` — counts and time range

Aggregates totals, level distribution, source distribution, and earliest /
latest timestamps. Output as JSON or Markdown.

```bash
logz-rs stats --path app.log --format json
logz-rs stats --path logs/ --format markdown --level ERROR --level WARN
```

Flags:

| Flag       | Description                                  |
| ---------- | -------------------------------------------- |
| `--path`   | File or directory. Stdin fallback as above.  |
| `--format` | `json` (default) or `markdown`.              |

### `tail` — follow a single file

Polls a file for new lines (no rotation/truncate detection — by design).
Applies filters to each new line and prints matches to stdout.

```bash
logz-rs tail --path app.log --interval 0.5 --level ERROR
logz-rs tail --path app.log --from-start --regex 'panic'
```

Flags:

| Flag           | Description                                                |
| -------------- | ---------------------------------------------------------- |
| `--path`       | File to tail (required).                                   |
| `--interval`   | Polling interval in seconds. Default `0.5`.                |
| `--from-start` | Read existing content before polling for new lines.        |

## Common filter flags

Available on every subcommand:

| Flag             | Description                                                                            |
| ---------------- | -------------------------------------------------------------------------------------- |
| `--date-format`  | strftime-style format for the leading timestamp. Default `%Y-%m-%d %H:%M:%S`.          |
| `--from-ts`      | Discard lines older than this (parsed with `--date-format`).                           |
| `--to-ts`        | Discard lines newer than this.                                                         |
| `--level`        | Keep only lines containing this level. Can be passed multiple times. Case-insensitive. |
| `--match`        | Substring that must appear in the line.                                                |
| `--regex`        | Regular expression that must match the line.                                           |

Levels recognised: `DEBUG`, `INFO`, `WARN`, `WARNING`, `ERROR`, `CRITICAL`.

When both `--from-ts`/`--to-ts` and a line has no parseable timestamp, the
line is dropped (it can't be placed in the window).

## Inputs

- **Files**: read line by line via a buffered reader.
- **Directories**: recursively enumerated; every file inside is streamed.
- **gzip**: files ending in `.gz` are decompressed on the fly.
- **stdin**: `analyze` and `stats` prompt for a path on stdin if `--path`
  is omitted.

## Tests

```bash
cargo test
```

Self-checks cover filter matching, timestamp extraction, and level
detection.

## Internals

Lines are streamed through `LogLineStream` (in `src/common.rs`), which
walks each file with a single reusable read buffer and applies filters
inline. Rejected lines never allocate a `LogLine`. `Arc<str>` is used
for per-file source labels so they're shared cheaply across millions of
records.

# todoscan

Scan source files for annotation tags (TODO, FIXME, HACK, XXX, BUG).

## Usage

```
todoscan [OPTIONS] [PATH]
```

Scans `PATH` recursively. Defaults to the current directory if omitted.

## Options

```
  -e, --ext <EXT>          Comma-separated file extensions to include (e.g. rs,py,js)
  -f, --file <FILE>        Scan a single specific file
  -p, --pattern <PATTERN>  Regex pattern to match (default: "TODO|FIXME|HACK|XXX|BUG")
  -i, --ignore-case        Case-insensitive matching
      --no-gitignore       Don't respect .gitignore rules
      --no-color           Disable colored output
  -c, --context <N>        Show N lines of context around each match (default: 0)
  -o, --output <FORMAT>    Output format: text (default), json, csv
  -h, --help               Print help
  -V, --version            Print version
```

## Examples

Scan the current directory:
```
todoscan
```

Scan only Rust and Python files:
```
todoscan -e rs,py ./src
```

Output as JSON:
```
todoscan -o json ./src
```

Show 2 lines of context around each match:
```
todoscan -c 2 ./src
```

Use a custom pattern:
```
todoscan -p "FIXME|HACK"
```

## Output

By default, results are printed to stdout in text format:

```
src/main.rs:42:4: TODO: handle error case
src/lib.rs:17:1: FIXME: off by one
```

Use `-o json` for machine-readable output or `-o csv` for spreadsheet-friendly output.

## Installation

```
cargo install --path .
```

## Build

```
cargo build --release
```

The binary is at `target/release/todoscan`.

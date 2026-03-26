# CLAUDE.md — todoscan

## Project Overview

`todoscan` is a cross-platform CLI tool written in Rust that recursively scans directories for code annotation tags (TODO, FIXME, HACK, XXX, BUG) and reports them with file paths, line numbers, and context. It runs on Linux, macOS, and Windows from a single compiled binary.

## Tech Stack

- **Language:** Rust (edition 2021, MSRV 1.75+)
- **Argument parsing:** `clap` with derive macros
- **Directory traversal:** `walkdir`
- **Gitignore support:** `ignore` crate
- **Pattern matching:** `regex`
- **Terminal output:** `colored`

## Project Structure

```
todoscan/
├── Cargo.toml
├── Cargo.lock
├── CLAUDE.md
├── README.md
├── LICENSE
├── src/
│   ├── main.rs          # Entry point, CLI arg parsing, orchestration
│   ├── scanner.rs       # Directory walking and file reading logic
│   ├── matcher.rs       # Pattern matching against file lines
│   ├── output.rs        # Formatting and printing results
│   └── config.rs        # CLI args struct, defaults, validation
└── tests/
    ├── integration.rs   # End-to-end CLI tests using assert_cmd
    └── fixtures/        # Sample files with known tags for testing
```

## CLI Interface

```
todoscan [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory or file to scan (default: current directory)

Options:
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

## Architecture & Design Decisions

- **Single-pass scanning:** Walk the directory tree once. For each file, read it line-by-line, test each line against the compiled regex, and collect matches. Do not load entire files into memory — use `BufReader` for line-by-line reading.
- **Filtering order:** Apply extension/filename filters _before_ opening files, not after. Skip binary files early using a byte-sniff check on the first 512 bytes.
- **Output is decoupled from scanning:** The scanner produces a `Vec<Match>` struct. The output module formats it. This keeps formatting concerns (JSON, CSV, colored text) out of the scan logic.
- **Default patterns are hardcoded, custom patterns override entirely.** When `--pattern` is provided, it replaces the defaults — it does not append to them.
- **Respect .gitignore by default.** Use the `ignore` crate's walk builder instead of raw `walkdir` when gitignore support is active. Fall back to `walkdir` when `--no-gitignore` is passed.

## Key Types

```rust
/// A single matched annotation found in a file.
struct Match {
    path: PathBuf,
    line_number: usize,
    column: usize,
    tag: String,        // The matched tag (e.g. "TODO", "FIXME")
    line_content: String,
    context_before: Vec<String>,
    context_after: Vec<String>,
}

/// Parsed and validated CLI configuration.
struct Config {
    root: PathBuf,
    extensions: Option<Vec<String>>,
    single_file: Option<PathBuf>,
    pattern: Regex,
    ignore_case: bool,
    respect_gitignore: bool,
    color: bool,
    context_lines: usize,
    output_format: OutputFormat,
}

enum OutputFormat {
    Text,
    Json,
    Csv,
}
```

## Code Conventions

- **Error handling:** Use `anyhow` for application errors. Do not `unwrap()` in library code. `unwrap()` is acceptable only in tests. Permission-denied and unreadable files should emit a stderr warning and continue scanning — never abort the whole run for a single unreadable file.
- **No `unsafe` code.** There is no need for it in this project.
- **Formatting:** Run `cargo fmt` before every commit. CI will reject unformatted code.
- **Linting:** `cargo clippy -- -D warnings` must pass with zero warnings.
- **Tests:** Every module should have unit tests in a `#[cfg(test)] mod tests` block. Integration tests go in `tests/`. Use `assert_cmd` and `predicates` for CLI-level tests.
- **Cross-platform paths:** Always use `std::path::Path` and `PathBuf`. Never hardcode `/` or `\` as separators. Use `Path::join()` for concatenation.
- **Avoid allocations in hot loops:** In the line-scanning inner loop, prefer borrowing over cloning. Compile the regex once and reuse it.

## Build & Test Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo test                     # Run all tests
cargo clippy -- -D warnings    # Lint
cargo fmt --check              # Check formatting
```

## Cross-Compilation

Build for other targets from Linux:

```bash
# Windows
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu

# macOS (requires cross or a macOS runner)
cargo install cross
cross build --release --target x86_64-apple-darwin
```

## Testing Strategy

- **Unit tests:** Test `matcher.rs` with known input lines and expected tag extraction. Test `config.rs` validation (e.g. invalid extension formats, nonexistent paths).
- **Integration tests:** Run the compiled binary against `tests/fixtures/` and assert on stdout content and exit codes.
- **Fixture files:** Include files with known annotations in multiple languages (Rust, Python, JS, C) so tests can verify extension filtering works.
- **Edge cases to cover:** Empty files, binary files, files without trailing newline, deeply nested directories, symlink loops, permission-denied files, lines with multiple tags, UTF-8 and non-UTF-8 files.

## Performance Considerations

- For large repos, the bottleneck is I/O, not regex. `BufReader` with default buffer size is fine.
- The `ignore` crate's parallel walker (`WalkParallel`) can be used as a future optimization but start with the sequential walker for simplicity and correctness.
- Compile the regex exactly once in `main` and pass a reference through to the scanner.

## What Not To Do

- Do not pull in a full async runtime (tokio, async-std). This is a synchronous CLI tool. Async adds complexity with no benefit here.
- Do not add a config file system (TOML, YAML). CLI flags are sufficient. If needed later, add it as a separate feature.
- Do not implement custom glob matching. Use the `ignore` crate's built-in glob support.
- Do not buffer all results in memory before printing. Print matches as they are found in text mode. Buffer only for JSON/CSV where the full structure is needed.
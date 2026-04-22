# rloc

`rloc` is a Rust CLI for analyzing repositories and reporting line-of-code metrics with language and file-category awareness.

Current workspace support includes:
- Shell (`sh`, `bash`, `zsh`, `.bashrc`, `.zshrc`, `.envrc`)
- SQL (`sql`, `psql`)
- Go
- HTML (`html`, `htm`, `xhtml`, `gohtml`)
- CSS
- Rust
- Python
- JavaScript
- TypeScript
- JSX
- TSX
- Markdown
- Config files (`toml`, `yaml`, `yml`, `json`, `jsonc`, `lock`, `ini`, `cfg`, `conf`)

Common lockfiles are parsed with the config backend, but categorized as `generated` in reports so they do not dominate normal source/config totals unless you opt in with `--generated`.

The tool currently provides four commands:
- `scan` — analyze a repository and print aggregated metrics
- `detect` — detect supported languages and matching project presets
- `explain` — explain how a single file is classified
- `config` — print the effective configuration or create a default config file

## Requirements

- Rust stable toolchain
- Rust 1.86 or newer for this workspace

The repository pins the toolchain to `stable` in `rust-toolchain.toml`.

## Build

Build the CLI binary from the workspace root:

```bash
cargo build -p rloc-cli
```

After that, the binary is available at:

```bash
target/debug/rloc
```

Build an optimized release binary:

```bash
cargo build --release -p rloc-cli
```

After that, the release binary is available at:

```bash
target/release/rloc
```

To install it into your local user bin directory:

```bash
install -Dm755 target/release/rloc ~/.local/bin/rloc
```

For one-off runs during development, you can also use:

```bash
cargo run -p rloc-cli -- <command>
```

## Quick start

Scan the current repository:

```bash
cargo run -p rloc-cli -- scan .
```

Detect languages and presets:

```bash
cargo run -p rloc-cli -- detect .
```

Explain classification for a single file:

```bash
cargo run -p rloc-cli -- explain crates/rloc-cli/src/main.rs
cargo run -p rloc-cli -- explain scripts/build.sh --format json
```

Print the effective configuration:

```bash
cargo run -p rloc-cli -- config
```

Create a default `.rloc.toml` in the current directory:

```bash
cargo run -p rloc-cli -- config init
```

Example scan output:

![Example `rloc scan` output](docs/images/scan-output.png)

## Commands

### `rloc [SCAN_OPTIONS] [PATH]`

Default command: `rloc` behaves like `rloc scan`.

### `rloc scan [OPTIONS] [PATH]`

Analyze a repository and print aggregated metrics.

Examples:

```bash
rloc
rloc .
rloc --format json .
rloc scan .
rloc scan . --format json
rloc scan . --group-by language --group-by category
rloc scan . --top-files 10 --top-dirs 5
rloc scan . --no-top-files --no-top-dirs
rloc scan . --list-unsupported
rloc scan . --languages rust,python,go,html,css
rloc scan . --languages rs,py,js,ts,tsx,sh,sql
rloc --exclude **/target/**,**/dist/** .
rloc scan . --no-tests --no-vendor
rloc scan . --generated
rloc scan . --config .rloc.toml
```

Available options:

- `--format <table|json>`
- `--group-by <language|category|dir|file>`
- `--top-files [N]` — override the default top-file limit; defaults to `10` when passed without a value
- `--no-top-files` — hide the top-files section entirely
- `--top-dirs [N]` — override the default top-dir limit; defaults to `10` when passed without a value
- `--no-top-dirs` — hide the top-dirs section entirely
- `--list-unsupported [N]` — list up to `N` example skipped unsupported files; defaults to `5` when passed without a value
- `--languages <LIST>` — comma-separated language list; aliases: `rs`, `py`, `js`, `ts`, `md`, `cfg`, `sh`; canonical names include `shell`, `sql`, `go`, `html`, `css`
- `--exclude <LIST>` — comma-separated exclude globs; may be passed multiple times and are appended to config excludes
- `--no-tests`
- `--generated` — include generated files
- `--no-vendor`
- `--config <PATH>`
- `--no-gitignore`
- `--hidden`

Default path: `.`

### `rloc detect [PATH]`

Run lightweight repository detection without a full scan.

This command prints:
- detected languages
- matching presets
- detected file categories
- active ignore rules
- warnings, if any

Example:

```bash
rloc detect .
rloc detect ./services/api
rloc detect . --list-unsupported
```

If `go.mod` is present or Go files are detected, `detect` also reports the `go` preset.

Available options:

- `--list-unsupported [N]` — list up to `N` example skipped unsupported files; defaults to `5` when passed without a value

Default path: `.`

### `rloc explain [OPTIONS] <FILE>`

Explain how a single file is classified.

Examples:

```bash
rloc explain src/lib.rs
rloc explain scripts/build.sh --format json
rloc explain crates/rloc-cli/src/main.rs --format json
rloc explain crates/rloc-cli/src/main.rs --format json --config .rloc.toml
```

Available options:

- `--format <table|json>`
- `--config <PATH>`

Default format: `table`

### `rloc config [COMMAND]`

Print the effective configuration discovered from the current directory upward, or initialize a default config file.
The rendered output starts with a `# source:` comment so it is obvious whether `rloc` loaded a local `.rloc.toml` or fell back to built-in defaults.

Commands:

- `init` — create a default `.rloc.toml` in the current directory

Examples:

```bash
rloc config
rloc config init
```

## Configuration

`rloc` currently uses repository-local configuration, not a separate global user config. By default, `scan` looks for the nearest `.rloc.toml` starting at the scan path and walking up parent directories. `explain` and `config` do the same starting from the current working directory. You can also provide an explicit config path with `--config`.

Configuration precedence is:
1. CLI flags
2. `.rloc.toml`
3. built-in defaults

Built-in default configuration:

```toml
[scan]
respect_gitignore = true
hidden = false

[filters]
exclude = []
include_tests = true
include_generated = false
include_vendor = false
generated_patterns = []
vendor_patterns = []

[classification]
count_doc_comments = true
count_docstrings_as_comments = true
mixed_lines_as_code = true

[report]
format = "table"
group_by = ["language"]
top_files = 10
top_dirs = 10
```

`scan` and `explain` honor classification policy from `.rloc.toml` when config is present. In the current CLI surface, `mixed_lines_as_code` affects all supported language backends, `count_doc_comments` affects Rust/JS-family doc comments, and `count_docstrings_as_comments` affects Python docstrings. Built-in report defaults are `format = "table"`, `group_by = ["language"]`, `top_files = 10`, and `top_dirs = 10`. Set `top_files = 0` or `top_dirs = 0` in config to disable those sections by default for a repository.

## Output

### Scan output

`scan` supports:
- `table`
- `json`

Table output also includes a compact `Category totals` block after `Summary`, so categories such as `source`, `test`, `docs`, and `config` are visible without adding `--group-by category`.

### Explain output

`explain` supports:
- `table`
- `json`

### Detect output

`detect` currently prints a human-readable table-style summary.

## Development

Useful workspace checks:

```bash
cargo fmt --all
cargo check
cargo test
```

## Workspace layout

```text
crates/
  rloc-cli/
  rloc-core/
  rloc-config/
  rloc-report/
  rloc-lang-shell/
  rloc-lang-sql/
  rloc-lang-go/
  rloc-lang-web/
  rloc-lang-rust/
  rloc-lang-python/
  rloc-lang-js/
```

## License

MIT

# Wave 1 Language Expansion Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add first-wave language support for `shell`, `sql`, `go`, `html`, and `css`, including scan/detect/explain integration, CLI language filters, and a new `go` preset.

**Architecture:** Keep the existing workspace boundaries intact. Extend `rloc-core` only for shared language modeling and filename-aware discovery, add dedicated scanner-based backend crates for `go`, `shell`, and `sql`, and add a shared `rloc-lang-web` crate for `html` and `css`. Do not add new config surface area, do not add tree-sitter dependencies for this wave, and do not introduce special-case report logic for the new languages.

**Tech Stack:** Rust 2024 workspace, `clap`, `serde`, existing deterministic scanners, `cargo fmt`, `cargo check`, `cargo test`.

---

## Current State

- Supported languages on current `HEAD`: `rust`, `python`, `javascript`, `typescript`, `jsx`, `tsx`, `markdown`, `config`.
- `scan`, `detect`, and `explain` already share a single `LanguageBackendRegistry`.
- Language detection is still extension-only in `crates/rloc-core/src/registry.rs`, so shell dotfiles such as `.bashrc` cannot be recognized yet.
- `detect` presets currently cover `rust`, `python`, `react`, and `monorepo`.
- The user-approved scope for this wave is:
  - add `Shell`, `Sql`, `Go`, `Html`, `Css` as normal supported languages;
  - treat `html` and `css` as `source` by default;
  - support wider filename/extension coverage now, not later;
  - add only one new preset: `go`.

## Acceptance Criteria

- `scan` no longer reports supported `sh/bash/zsh/sql/go/html/css` inputs as unsupported.
- `explain` works for one representative file of each new language.
- `detect` lists the new languages and reports `go` when `go.mod` exists or Go files are present.
- CLI language filters and help text expose the new language names and aliases.
- `cargo fmt --all`
- `cargo check`
- `cargo test`
- `cargo run -p rloc-cli -- scan .`
- `cargo run -p rloc-cli -- detect .`
- `cargo run -p rloc-cli -- explain path/to/sample.go`

## Guardrails

- Do not add new config keys for language selection or classification in this wave.
- Do not add embedded-language parsing inside HTML `<script>` or `<style>` blocks yet.
- Do not try to fully model shell heredocs or PostgreSQL dollar-quoted strings in the first pass unless a targeted regression test demands it.
- Prefer scanner-based classification over parser integration for all five languages in this wave.
- Keep `html` and `css` in `FileCategory::Source` unless existing path heuristics make them `test`, `generated`, `vendor`, and so on.

### Task 1: Extend The Shared Language Model And Registry

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/rloc-core/src/types.rs`
- Modify: `crates/rloc-core/src/registry.rs`
- Modify: `crates/rloc-cli/src/cli.rs`
- Modify: `crates/rloc-cli/src/commands/mod.rs`
- Test: `crates/rloc-core/src/registry.rs`
- Test: `crates/rloc-cli/src/cli.rs`
- Test: `crates/rloc-cli/src/commands/mod.rs`

**Step 1: Write the failing registry and CLI tests**

Add or extend tests that require:

```rust
#[test]
fn registry_detects_shell_dotfiles_and_wave1_extensions() {}

#[test]
fn supported_languages_include_wave1_languages() {}

#[test]
fn parses_wave1_language_aliases() {}

#[test]
fn scan_help_mentions_wave1_language_aliases() {}
```

The assertions should cover:
- `Language::{Shell, Sql, Go, Html, Css}` serialize to `shell`, `sql`, `go`, `html`, `css`;
- registry detection for:
  - `script.sh`
  - `script.bash`
  - `script.zsh`
  - `.bashrc`
  - `.zshrc`
  - `.envrc`
  - `query.sql`
  - `query.psql`
  - `main.go`
  - `index.html`
  - `index.htm`
  - `layout.xhtml`
  - `page.gohtml`
  - `app.css`
- CLI language parsing for `--languages sh,sql,go,html,css`.

**Step 2: Run the targeted tests to verify red**

Run: `cargo test -p rloc-core registry_detects_shell_dotfiles_and_wave1_extensions`
Expected: FAIL because the new language variants and filename matching do not exist yet.

Run: `cargo test -p rloc-cli parses_wave1_language_aliases`
Expected: FAIL because the CLI does not expose the new language values yet.

**Step 3: Implement the minimal shared model changes**

- Add `Shell`, `Sql`, `Go`, `Html`, and `Css` to `Language` in `crates/rloc-core/src/types.rs`.
- Extend `Language::as_str()` so JSON output uses the canonical spellings above.
- Extend `LanguageArg` in `crates/rloc-cli/src/cli.rs` with:
  - `Shell` plus alias `sh`
  - `Sql`
  - `Go`
  - `Html`
  - `Css`
- Update the `From<LanguageArg>` mapping.
- Update help text comments that still mention only `rs, py, js, ts, md, cfg`.
- Teach `LanguageBackendRegistry` to match both:
  - file extensions;
  - exact filenames.

Use the smallest API change that keeps the registry readable. A good shape is:

```rust
pub struct LanguageDescriptor {
    pub language: Language,
    pub display_name: &'static str,
    pub extensions: &'static [&'static str],
    pub file_names: &'static [&'static str],
}
```

Then make `detect_language()` check `path.file_name()` before `path.extension()`.

**Step 4: Update the default registry expectations**

- Do not register the new backends yet.
- Do update `default_registry_wires_all_workspace_backends()` so it becomes the reminder test to fix once the backend crates land.

**Step 5: Re-run the targeted tests**

Run: `cargo test -p rloc-core registry_detects_shell_dotfiles_and_wave1_extensions`
Expected: PASS

Run: `cargo test -p rloc-cli parses_wave1_language_aliases`
Expected: PASS

**Step 6: Commit**

```bash
git add Cargo.toml crates/rloc-core/src/types.rs crates/rloc-core/src/registry.rs crates/rloc-cli/src/cli.rs crates/rloc-cli/src/commands/mod.rs
git commit -m "feat: extend core language registry for wave1 languages"
```

### Task 2: Add The Shell Backend

**Files:**
- Create: `crates/rloc-lang-shell/Cargo.toml`
- Create: `crates/rloc-lang-shell/src/lib.rs`
- Create: `crates/rloc-lang-shell/src/classify.rs`
- Create: `crates/rloc-lang-shell/tests/classify.rs`
- Modify: `Cargo.toml`
- Modify: `crates/rloc-cli/src/commands/mod.rs`
- Test: `crates/rloc-cli/tests/scan_output.rs`
- Test: `crates/rloc-cli/tests/explain_output.rs`

**Step 1: Write the failing shell tests**

Create `crates/rloc-lang-shell/tests/classify.rs` with tests like:

```rust
#[test]
fn shebang_is_counted_as_code() {}

#[test]
fn inline_hash_comment_becomes_mixed() {}

#[test]
fn hash_inside_quotes_does_not_start_a_comment() {}
```

Add CLI integration tests that require:

```rust
#[test]
fn scan_counts_shell_files_as_supported_inputs() {}

#[test]
fn explain_supports_shell_dotfiles() {}
```

Use `.bashrc` in the explain test so filename matching is exercised end-to-end.

**Step 2: Run the targeted tests to verify red**

Run: `cargo test -p rloc-lang-shell --test classify`
Expected: FAIL because the crate does not exist yet.

Run: `cargo test -p rloc-cli scan_counts_shell_files_as_supported_inputs`
Expected: FAIL because shell is still unsupported.

**Step 3: Create the minimal shell crate**

- Add `crates/rloc-lang-shell` to workspace members.
- Depend only on `rloc-core` and existing workspace crates.
- Expose:

```rust
pub const DESCRIPTORS: [LanguageDescriptor; 1];
pub fn descriptor() -> LanguageDescriptor;
pub fn backend() -> ShellBackend;
```

- Register these extensions and filenames:
  - extensions: `sh`, `bash`, `zsh`
  - exact filenames: `.bashrc`, `.zshrc`, `.envrc`

**Step 4: Implement scanner-only shell classification**

Treat:
- shebang as `code`;
- `#` outside strings as comment start;
- `echo ok # note` as `mixed`;
- `"# not a comment"` and `'# not a comment'` as `code`.

Do not over-engineer heredoc support in this wave.

**Step 5: Register the backend**

- Add `rloc_lang_shell::backend()` to `default_registry()` in `crates/rloc-cli/src/commands/mod.rs`.
- Update the default registry test count and language list.

**Step 6: Re-run the targeted tests**

Run: `cargo test -p rloc-lang-shell --test classify`
Expected: PASS

Run: `cargo test -p rloc-cli scan_counts_shell_files_as_supported_inputs`
Expected: PASS

Run: `cargo test -p rloc-cli explain_supports_shell_dotfiles`
Expected: PASS

**Step 7: Commit**

```bash
git add Cargo.toml crates/rloc-lang-shell crates/rloc-cli/src/commands/mod.rs crates/rloc-cli/tests/scan_output.rs crates/rloc-cli/tests/explain_output.rs
git commit -m "feat: add shell language backend"
```

### Task 3: Add The SQL Backend

**Files:**
- Create: `crates/rloc-lang-sql/Cargo.toml`
- Create: `crates/rloc-lang-sql/src/lib.rs`
- Create: `crates/rloc-lang-sql/src/classify.rs`
- Create: `crates/rloc-lang-sql/tests/classify.rs`
- Modify: `Cargo.toml`
- Modify: `crates/rloc-cli/src/commands/mod.rs`
- Test: `crates/rloc-cli/tests/scan_output.rs`
- Test: `crates/rloc-cli/tests/explain_output.rs`

**Step 1: Write the failing SQL tests**

Create `crates/rloc-lang-sql/tests/classify.rs` with:

```rust
#[test]
fn double_dash_comments_are_comment_or_mixed_lines() {}

#[test]
fn block_comments_span_multiple_lines() {}

#[test]
fn comment_markers_inside_strings_do_not_start_comments() {}
```

Add CLI integration tests:

```rust
#[test]
fn scan_counts_sql_files_as_supported_inputs() {}

#[test]
fn explain_supports_psql_files() {}
```

**Step 2: Run the targeted tests to verify red**

Run: `cargo test -p rloc-lang-sql --test classify`
Expected: FAIL because the crate does not exist yet.

Run: `cargo test -p rloc-cli scan_counts_sql_files_as_supported_inputs`
Expected: FAIL because SQL is still unsupported.

**Step 3: Create the minimal SQL crate**

- Add `crates/rloc-lang-sql` to the workspace.
- Register:
  - `sql`
  - `psql`

**Step 4: Implement scanner-only SQL classification**

Support:
- `--` line comments;
- `/* ... */` block comments;
- single-quoted strings;
- double-quoted identifiers.

Keep PostgreSQL dollar-quoted strings out of scope unless a failing regression requires them later.

**Step 5: Register the backend**

- Add `rloc_lang_sql::backend()` to the default registry.
- Update the default registry test count and language list again.

**Step 6: Re-run the targeted tests**

Run: `cargo test -p rloc-lang-sql --test classify`
Expected: PASS

Run: `cargo test -p rloc-cli scan_counts_sql_files_as_supported_inputs`
Expected: PASS

Run: `cargo test -p rloc-cli explain_supports_psql_files`
Expected: PASS

**Step 7: Commit**

```bash
git add Cargo.toml crates/rloc-lang-sql crates/rloc-cli/src/commands/mod.rs crates/rloc-cli/tests/scan_output.rs crates/rloc-cli/tests/explain_output.rs
git commit -m "feat: add sql language backend"
```

### Task 4: Add The Go Backend And `go` Preset

**Files:**
- Create: `crates/rloc-lang-go/Cargo.toml`
- Create: `crates/rloc-lang-go/src/lib.rs`
- Create: `crates/rloc-lang-go/src/classify.rs`
- Create: `crates/rloc-lang-go/tests/classify.rs`
- Modify: `Cargo.toml`
- Modify: `crates/rloc-cli/src/commands/mod.rs`
- Modify: `crates/rloc-config/src/presets.rs`
- Test: `crates/rloc-cli/tests/scan_output.rs`
- Test: `crates/rloc-cli/tests/explain_output.rs`
- Test: `crates/rloc-cli/tests/detect_output.rs`
- Test: `crates/rloc-config/src/presets.rs`

**Step 1: Write the failing Go tests**

Create `crates/rloc-lang-go/tests/classify.rs` with:

```rust
#[test]
fn slash_comments_and_inline_comments_are_classified() {}

#[test]
fn raw_strings_do_not_trigger_false_comment_detection() {}

#[test]
fn rune_literals_do_not_trigger_false_comment_detection() {}
```

Add integration tests that require:

```rust
#[test]
fn scan_counts_go_files_as_supported_inputs() {}

#[test]
fn explain_supports_go_sources() {}

#[test]
fn detect_reports_go_preset_for_go_mod_and_go_files() {}
```

Add preset-unit coverage in `crates/rloc-config/src/presets.rs` for:
- `go.mod`;
- Go source without `go.mod`.

**Step 2: Run the targeted tests to verify red**

Run: `cargo test -p rloc-lang-go --test classify`
Expected: FAIL because the crate does not exist yet.

Run: `cargo test -p rloc-cli detect_reports_go_preset_for_go_mod_and_go_files`
Expected: FAIL because `go` preset does not exist.

**Step 3: Create the minimal Go crate**

- Add `crates/rloc-lang-go` to the workspace.
- Register extension `go`.
- Implement a scanner close to the existing Rust backend rules:
  - `//`
  - `/* ... */`
  - interpreted strings `"..."`;
  - raw strings `` `...` ``;
  - rune literals `'x'`.

Doc comments are regular comments in this wave, not a separate `doc` bucket.

**Step 4: Register the backend**

- Add `rloc_lang_go::backend()` to the default registry.

**Step 5: Add the `go` preset**

- Extend `Preset` in `crates/rloc-config/src/presets.rs` with `Go`.
- Detect it when:
  - `go.mod` exists; or
  - `report.languages` contains `Language::Go`.

Keep `react`, `python`, `rust`, and `monorepo` behavior unchanged.

**Step 6: Re-run the targeted tests**

Run: `cargo test -p rloc-lang-go --test classify`
Expected: PASS

Run: `cargo test -p rloc-config detects_language_driven_presets`
Expected: PASS with `go` folded into the expected list when appropriate.

Run: `cargo test -p rloc-cli detect_reports_go_preset_for_go_mod_and_go_files`
Expected: PASS

**Step 7: Commit**

```bash
git add Cargo.toml crates/rloc-lang-go crates/rloc-cli/src/commands/mod.rs crates/rloc-config/src/presets.rs crates/rloc-cli/tests/scan_output.rs crates/rloc-cli/tests/explain_output.rs crates/rloc-cli/tests/detect_output.rs
git commit -m "feat: add go language backend and preset"
```

### Task 5: Add The Shared HTML/CSS Backend Crate

**Files:**
- Create: `crates/rloc-lang-web/Cargo.toml`
- Create: `crates/rloc-lang-web/src/lib.rs`
- Create: `crates/rloc-lang-web/src/classify.rs`
- Create: `crates/rloc-lang-web/tests/classify.rs`
- Modify: `Cargo.toml`
- Modify: `crates/rloc-cli/src/commands/mod.rs`
- Test: `crates/rloc-cli/tests/scan_output.rs`
- Test: `crates/rloc-cli/tests/explain_output.rs`

**Step 1: Write the failing web tests**

Create `crates/rloc-lang-web/tests/classify.rs` with:

```rust
#[test]
fn html_comments_are_comment_lines_and_markup_is_code() {}

#[test]
fn html_text_content_is_counted_as_code() {}

#[test]
fn css_block_comments_are_comment_lines() {}

#[test]
fn css_comment_markers_inside_strings_do_not_start_comments() {}
```

Add CLI integration tests:

```rust
#[test]
fn scan_counts_html_and_css_files_as_supported_inputs() {}

#[test]
fn explain_supports_html_and_css_sources() {}
```

Use `.gohtml` and `.htm` in the scan fixture to cover the approved extended scope.

**Step 2: Run the targeted tests to verify red**

Run: `cargo test -p rloc-lang-web --test classify`
Expected: FAIL because the crate does not exist yet.

Run: `cargo test -p rloc-cli scan_counts_html_and_css_files_as_supported_inputs`
Expected: FAIL because HTML and CSS are still unsupported.

**Step 3: Create the minimal shared web crate**

- Add `crates/rloc-lang-web` to the workspace.
- Expose two backends from one crate:

```rust
pub fn html_backend() -> HtmlBackend;
pub fn css_backend() -> CssBackend;
```

- Register:
  - HTML: `html`, `htm`, `xhtml`, `gohtml`
  - CSS: `css`

**Step 4: Implement HTML classification**

Support:
- `<!-- ... -->` comments;
- everything else non-blank as `code`.

Do not attempt embedded JS or CSS parsing inside HTML in this wave.

**Step 5: Implement CSS classification**

Support:
- `/* ... */` comments;
- quoted strings;
- regular declarations/selectors as `code`.

**Step 6: Register both backends**

- Add `rloc_lang_web::html_backend()` and `rloc_lang_web::css_backend()` to the default registry.
- Update the default registry test count and language list one more time.

**Step 7: Re-run the targeted tests**

Run: `cargo test -p rloc-lang-web --test classify`
Expected: PASS

Run: `cargo test -p rloc-cli scan_counts_html_and_css_files_as_supported_inputs`
Expected: PASS

Run: `cargo test -p rloc-cli explain_supports_html_and_css_sources`
Expected: PASS

**Step 8: Commit**

```bash
git add Cargo.toml crates/rloc-lang-web crates/rloc-cli/src/commands/mod.rs crates/rloc-cli/tests/scan_output.rs crates/rloc-cli/tests/explain_output.rs
git commit -m "feat: add html and css language backends"
```

### Task 6: Finish End-To-End Coverage And Docs

**Files:**
- Modify: `README.md`
- Modify: `crates/rloc-cli/src/cli.rs`
- Modify: `crates/rloc-cli/tests/scan_output.rs`
- Modify: `crates/rloc-cli/tests/explain_output.rs`
- Modify: `crates/rloc-cli/tests/detect_output.rs`
- Modify: `crates/rloc-report/tests/scan_contract.rs`

**Step 1: Write the final failing integration assertions**

Add or extend tests to require:

```rust
#[test]
fn scan_wave1_languages_no_longer_show_as_unsupported() {}

#[test]
fn detect_lists_wave1_languages_in_human_output() {}

#[test]
fn explain_json_uses_canonical_wave1_language_names() {}
```

Also add a report contract test that verifies JSON output now emits the canonical spellings:
- `shell`
- `sql`
- `go`
- `html`
- `css`

**Step 2: Run the targeted tests to verify red**

Run: `cargo test -p rloc-cli scan_wave1_languages_no_longer_show_as_unsupported`
Expected: FAIL until all backends are wired and registered.

Run: `cargo test -p rloc-report scan_contract`
Expected: FAIL until the new fixtures are added.

**Step 3: Update docs and help**

- Update `README.md`:
  - supported language list;
  - `--languages` examples;
  - `detect` examples showing `go`;
  - explain examples for one new language.
- Update CLI help strings in `crates/rloc-cli/src/cli.rs` so the alias hint is no longer stale.

**Step 4: Verify end-to-end behavior**

Run: `cargo fmt --all`
Expected: PASS

Run: `cargo check`
Expected: PASS

Run: `cargo test`
Expected: PASS

Run: `cargo run -p rloc-cli -- detect .`
Expected: PASS, with no regressions to existing preset reporting.

Run: `cargo run -p rloc-cli -- scan .`
Expected: PASS, with supported wave1 files counted rather than bucketed under unsupported extensions.

**Step 5: Commit**

```bash
git add README.md crates/rloc-cli/src/cli.rs crates/rloc-cli/tests/scan_output.rs crates/rloc-cli/tests/explain_output.rs crates/rloc-cli/tests/detect_output.rs crates/rloc-report/tests/scan_contract.rs
git commit -m "docs: expose wave1 language support"
```

## Final Verification Checklist

Run these commands in order after Task 6:

```bash
cargo fmt --all
cargo check
cargo test
cargo run -p rloc-cli -- scan .
cargo run -p rloc-cli -- detect .
```

Expected results:
- all commands exit `0`;
- scan output includes wave1 languages when sample files are present;
- detect can emit `go`;
- explain works for `.bashrc`, `.psql`, `.go`, `.gohtml`, and `.css`.

## Suggested Commit Order

1. `feat: extend core language registry for wave1 languages`
2. `feat: add shell language backend`
3. `feat: add sql language backend`
4. `feat: add go language backend and preset`
5. `feat: add html and css language backends`
6. `docs: expose wave1 language support`

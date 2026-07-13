+++
slug = "add-wave2-languages"
type = "feat"
created = "2026-07-13"
status = "approved"
# `mode` is optional — leave it absent to let flow-implement ask at run
# time. If you know the answer up front, set:
#   mode = "worktree" — flow creates `.worktree/<id>` + a new branch.
#   mode = "inline"   — no separate dir; flow creates/checks out the plan branch.
#   mode = "current"  — track the plan on whatever branch is currently
#                       checked out; flow never creates or switches branches.
# tdd = false
review_after_group = false
# Task ids where flow-implement should checkpoint (commit). A checkpoint
# closes a MILESTONE: the run of tasks since the previous checkpoint lands in
# one commit. List the last task id of each milestone EXCEPT the final one —
# the trailing run is committed by the end-of-plan flush.
#   e.g. T1–T3 + T4–T5 = two milestones → checkpoints = ["T3"] (two commits)
# Checkpointing every task = one commit per task (a smell `flow plan check`
# warns about). An empty list just means no mid-plan commits; with
# auto_commit = true flow-implement still lands the whole result in one
# final commit at the end.
checkpoints = ["T2", "T5", "T8", "T12"]
# auto_commit is the permission switch (defaults true — keep it). true:
# flow-implement commits its own work without asking (checkpoints, if any,
# plus a final commit). false: it asks once at the end. Set false only for
# a single final review/commit, and only when the human asks.
auto_commit = true
goal = "Add comment-aware line-counting support for eleven new file types (.c, .cpp, .h, .java, .swift, .m, .zig, .xml, .ps1, .txt, .mdx) by introducing a shared C-family backend crate, a PowerShell crate, an XML backend in rloc-lang-web, and text/mdx handling in rloc-core, then wiring them into the CLI registry."
non_goals = [
    "Adding config-file presets (rloc-config) for the new languages.",
    "Handling every exotic literal form: nested block comments (Swift), C++ raw strings R\"(...)\", Swift/PowerShell here-strings and triple-quoted strings, Zig multiline string doc semantics — simple scanners are accepted; edge cases are documented as known limitations.",
    "Adding new FileCategory rules for the new extensions (e.g. classifying .txt or .xml specially); category detection stays as-is.",
    "Distinguishing doc comments (///, /**) from regular comments in the C-family scanner; all comments count as `comment`, matching the existing Go backend.",
    "Adding tree-sitter grammars for any new language; all new backends are hand-written line scanners.",
    "Treating .m as MATLAB or .h as a distinct C++ header language.",
]
max_review_iterations = 2
worktree_include = []

# Tasks: small, sequential, independently verifiable.
# `depends_on` — DAG edges; leave empty for the first task, then depend on
# the previous task or the real prerequisite task.
# `stage` — optional cognition layer for long plans (15+ tasks).
#   Set on every task or none. Common values: "design", "implement", "verify".

[[task]]
id = "T1"
title = "Add Language variants C, Cpp, Java, Swift, ObjectiveC, Zig, Xml, PowerShell, Text (before Unknown) with matching as_str arms (c, cpp, java, swift, objective-c, zig, xml, powershell, text) in rloc-core/src/types.rs, AND add the nine new variants to both exhaustive fallback match arms (parser_name and parse) in rloc-lang-js/src/parser.rs so the workspace still compiles"
status = "pending"
depends_on = []
files_modify = ["crates/rloc-core/src/types.rs", "crates/rloc-lang-js/src/parser.rs"]
verify = ["cargo check --workspace --all-targets"]

[[task]]
id = "T2"
title = "In text_backend.rs add mdx and markdown to the Markdown descriptor extensions, add a TextMode::Text plus text_backend() (descriptor Language::Text, display Text, extensions txt, non-blank lines counted as doc), re-export text_backend from lib.rs, and add unit tests asserting: a .txt file's non-blank lines count as doc, the markdown descriptor lists md/mdx/markdown, and the text descriptor lists txt"
status = "pending"
depends_on = ["T1"]
files_modify = ["crates/rloc-core/src/text_backend.rs", "crates/rloc-core/src/lib.rs"]
verify = ["cargo test -p rloc-core"]

[[task]]
id = "T3"
title = "Create the rloc-lang-cfamily crate (Cargo.toml depending on rloc-core, added to workspace members in root Cargo.toml, lib.rs with `pub mod classify;`) and write classify.rs: a C-style scanner classify_file(path, language, category, options) adapted from rloc-lang-go handling // line comments, \"...\" strings with \\ escapes, and '...' char literals; block comments /* */ are recognized for every language EXCEPT Language::Zig (gated by a per-language capability), classifying into blank/code/comment/mixed"
status = "pending"
depends_on = ["T2"]
files_create = ["crates/rloc-lang-cfamily/Cargo.toml", "crates/rloc-lang-cfamily/src/lib.rs", "crates/rloc-lang-cfamily/src/classify.rs"]
files_modify = ["Cargo.toml", "Cargo.lock"]
verify = ["cargo build -p rloc-lang-cfamily"]

[[task]]
id = "T4"
title = "In rloc-lang-cfamily/src/lib.rs add six backend structs and descriptors — C (extensions c,h), Cpp (cpp), Java (java), Swift (swift), ObjectiveC (m), Zig (zig) — each implementing LanguageBackend by delegating to classify::classify_file with its Language, plus public constructors c_backend/cpp_backend/java_backend/swift_backend/objective_c_backend/zig_backend"
status = "pending"
depends_on = ["T3"]
files_modify = ["crates/rloc-lang-cfamily/src/lib.rs"]
verify = ["cargo build -p rloc-lang-cfamily"]

[[task]]
id = "T5"
title = "Add crates/rloc-lang-cfamily/tests/classify.rs covering C (// and /* */ comments, string and char literals, mixed line), C++, Java, Swift, Objective-C, and Zig; include a Zig regression asserting a line containing /* ... */ is NOT counted as a comment (Zig has no block comments) and that a \\\\ multiline-string line is code; assert each backend's descriptor reports the expected extensions (c+h under C, m under ObjectiveC)"
status = "pending"
depends_on = ["T4"]
files_create = ["crates/rloc-lang-cfamily/tests/classify.rs"]
verify = ["cargo test -p rloc-lang-cfamily"]

[[task]]
id = "T6"
title = "Add XML support to rloc-lang-web: a Language::Xml dispatch arm in classify.rs calling classify_html_line (plus Xml arms in the error/line_reason matches), and XmlBackend + XML_DESCRIPTOR (extensions xml, display XML) + xml_backend() in lib.rs; add an XML <!-- --> case to tests/classify.rs"
status = "pending"
depends_on = ["T5"]
files_modify = ["crates/rloc-lang-web/src/classify.rs", "crates/rloc-lang-web/src/lib.rs", "crates/rloc-lang-web/tests/classify.rs"]
verify = ["cargo test -p rloc-lang-web"]

[[task]]
id = "T7"
title = "Create the rloc-lang-powershell crate (Cargo.toml depending on rloc-core, added to workspace members, lib.rs with PowerShellBackend + descriptor extensions ps1 display PowerShell + backend(); classify.rs scanner handling # line comments and <# #> block comments, tracking \"...\" strings (backtick ` escapes the next char) and '...' literal strings so that # and <# inside a string are code, not comments — into blank/code/comment/mixed)"
status = "pending"
depends_on = ["T6"]
files_create = ["crates/rloc-lang-powershell/Cargo.toml", "crates/rloc-lang-powershell/src/lib.rs", "crates/rloc-lang-powershell/src/classify.rs"]
files_modify = ["Cargo.toml", "Cargo.lock"]
verify = ["cargo build -p rloc-lang-powershell"]

[[task]]
id = "T8"
title = "Add crates/rloc-lang-powershell/tests/classify.rs covering: a # line comment, a multi-line <# #> block comment, a code line, a mixed code+comment line, a # inside a double-quoted string counted as code (\"a#b\"), a <# #> sequence inside a single-quoted string counted as code ('<# #>'), and a backtick-escaped quote keeping the double-quoted string open so a trailing # is still code (\"x `\" y # z\")"
status = "pending"
depends_on = ["T7"]
files_create = ["crates/rloc-lang-powershell/tests/classify.rs"]
verify = ["cargo test -p rloc-lang-powershell"]

[[task]]
id = "T9"
title = "Wire the new backends into the CLI: add rloc-lang-cfamily and rloc-lang-powershell to crates/rloc-cli/Cargo.toml, register all nine backends (c/cpp/java/swift/objective_c/zig, xml, powershell, text) in default_registry() in commands/mod.rs, and update its test to assert len == 22 and contains each new Language variant"
status = "pending"
depends_on = ["T8"]
files_modify = ["crates/rloc-cli/Cargo.toml", "crates/rloc-cli/src/commands/mod.rs"]
verify = ["cargo test -p rloc-cli default_registry"]

[[task]]
id = "T10"
title = "Enable CLI --languages selection for the new languages: add variants C, Cpp, Java, Swift, ObjectiveC, Zig, Xml, Powershell, Text to LanguageArg in cli.rs (with sensible #[value(alias(...))] aliases e.g. objc/m for ObjectiveC, ps1 for Powershell, txt for Text), extend the From<LanguageArg> for Language match, update the --languages doc comment, and extend parses_scan_language_aliases to cover at least two new selectors"
status = "pending"
depends_on = ["T9"]
files_modify = ["crates/rloc-cli/src/cli.rs"]
verify = ["cargo test -p rloc-cli -- language"]

[[task]]
id = "T11"
title = "Add a positive end-to-end detect integration test detect_lists_wave2_languages_in_human_output in crates/rloc-cli/tests/detect_output.rs that writes one file per new mapping (main.c, lib.cpp, api.h, App.java, View.swift, Cell.m, build.zig, config.xml, deploy.ps1, notes.txt, page.mdx) and asserts the detect output lists c, cpp, java, swift, objective-c, zig, xml, powershell, text, and markdown (proving .h→c, .m→objective-c, .mdx→markdown resolve correctly)"
status = "pending"
depends_on = ["T10"]
files_modify = ["crates/rloc-cli/tests/detect_output.rs"]
verify = ["cargo test -p rloc-cli detect_lists_wave2_languages"]

[[task]]
id = "T12"
title = "Fix CLI integration tests that relied on .txt being unsupported: in exit_codes.rs, detect_output.rs and scan_output.rs replace the .txt fixtures (notes.txt / README.txt) with a still-unsupported extension (.dat) and update the count/filename/breakdown assertions accordingly"
status = "pending"
depends_on = ["T11"]
files_modify = ["crates/rloc-cli/tests/exit_codes.rs", "crates/rloc-cli/tests/detect_output.rs", "crates/rloc-cli/tests/scan_output.rs"]
verify = ["cargo test --workspace"]

[[task]]
id = "T13"
title = "Update README.md: extend the 'Current workspace support includes' list with C, C++, Objective-C (.m), Swift, Java, Zig, XML, PowerShell (.ps1), Text (.txt), note .mdx is counted as Markdown, and add the new --languages selector aliases to the scan docs"
status = "pending"
depends_on = ["T12"]
files_modify = ["README.md"]
verify = ["grep -q 'PowerShell' README.md", "cargo clippy --workspace --all-targets -- -D warnings"]
+++

# Implementation Plan: add-wave2-languages

## Goal

Extend rloc's language coverage from 13 to 22 languages by adding comment-aware
backends for eleven requested file types. They group as:

- **C-family** (`.c`, `.cpp`, `.h`, `.java`, `.swift`, `.m`) plus **Zig** (`.zig`)
  — a new `rloc-lang-cfamily` crate with one parameterized scanner
  (`//` line + `/* */` block comments, `"..."` strings, `'...'` char literals),
  mirroring how `rloc-lang-web` shares one `classify.rs` across HTML and CSS.
  `.m` is Objective-C; `.h` is labelled C.
- **XML** (`.xml`) — an `xml_backend` in the existing `rloc-lang-web` crate that
  reuses the HTML `<!-- -->` markup scanner.
- **PowerShell** (`.ps1`) — a new `rloc-lang-powershell` crate (`#` line +
  `<# #>` block comments, `"..."`/`'...'` strings).
- **Text** (`.txt`) and **MDX** (`.mdx`) — handled in `rloc-core`'s
  `text_backend`: a new plain-text backend counting non-blank lines as `doc`,
  and extending the Markdown descriptor to also match `mdx` (and `markdown`).

Each backend follows the existing pattern (per-crate `descriptor()` +
`LanguageBackend` impl + `tests/classify.rs`), is registered in
`default_registry()`, and behaves consistently with the current Go/CSS/HTML
backends. Existing tests that used `.txt` as an "unsupported" fixture are
updated because `.txt` becomes supported.

## Non-goals

- Adding config-file presets (`rloc-config::detect_presets`) for the new languages.
- Handling exotic literal forms: nested block comments (Swift), C++ raw strings
  `R"(...)"`, Swift/PowerShell here-strings and triple-quoted strings. Simple
  scanners are accepted; edge cases are listed under Risks.
- Adding new `FileCategory` rules for the new extensions. Category detection in
  `categories.rs` is unchanged (`.mdx`/`.markdown` already map to `docs`).
- Distinguishing doc comments (`///`, `/** */`) from regular comments; all
  comments count as `comment`, matching the existing Go backend.
- Adding tree-sitter grammars; every new backend is a hand-written line scanner.
- Treating `.m` as MATLAB, or `.h` as a distinct C++-header language.
- Adding C++ aliases beyond `.cpp` (e.g. `.cc`, `.cxx`, `.hpp`) — only the
  requested extensions are registered.

## Acceptance criteria

- [ ] `Language` enum gains variants `C`, `Cpp`, `Java`, `Swift`, `ObjectiveC`,
      `Zig`, `Xml`, `PowerShell`, `Text`, each with an `as_str` arm
      (`c`, `cpp`, `java`, `swift`, `objective-c`, `zig`, `xml`, `powershell`,
      `text`).
- [ ] The workspace still compiles after the enum expansion: the two exhaustive
      `match language` blocks in `rloc-lang-js/src/parser.rs` are extended to
      cover the new variants (`cargo check --workspace --all-targets` passes).
- [ ] `default_registry()` wires all new backends; its test asserts
      `supported_languages().len() == 22` and `contains` each new language.
- [ ] `rloc detect`/`scan` classify `.c`, `.cpp`, `.h`→C, `.java`, `.swift`,
      `.m`→Objective-C, `.zig`, `.xml`, `.ps1`, `.txt`→Text, `.mdx`→Markdown
      (no longer reported as unsupported extensions) — verified by a positive
      end-to-end detect integration test that exercises every mapping,
      explicitly the non-obvious `.h`→C, `.m`→Objective-C, `.mdx`→Markdown.
- [ ] The new languages are selectable via `scan --languages` (`LanguageArg`
      extended, `From<LanguageArg> for Language` extended, aliases added);
      verified by an extended `parses_scan_language_aliases` test.
- [ ] The C-family scanner correctly distinguishes blank / code / comment /
      mixed lines for `//` and `/* */`, ignoring comment tokens inside strings
      and char literals; Zig recognizes `//` only (a `/* */` sequence in a Zig
      file is NOT a comment); verified by `rloc-lang-cfamily/tests/classify.rs`.
- [ ] The PowerShell scanner distinguishes `#` and `<# #>` comments from code;
      verified by `rloc-lang-powershell/tests/classify.rs`.
- [ ] The XML backend counts `<!-- -->` comments; verified in
      `rloc-lang-web/tests/classify.rs`.
- [ ] The Text backend counts non-blank lines as `doc`; the Markdown descriptor
      now also matches `mdx`/`markdown`; verified in `rloc-core` unit tests.
- [ ] CLI integration tests that used `.txt` as an unsupported fixture
      (`exit_codes.rs`, `detect_output.rs`, `scan_output.rs`) are updated to a
      still-unsupported extension and pass.
- [ ] `cargo test --workspace` and `cargo clippy --workspace --all-targets --
      -D warnings` are green.

## Task overview

Full task detail (files, verify commands, dependencies) lives in the `[[task]]`
frontmatter and renders with `flow plan show`; this is a milestone-level map.

- **Milestone 1 — core types & text (T1–T2, checkpoint T2)**
  - T1 — add 9 `Language` variants + `as_str`; extend the two exhaustive
    `parser.rs` match arms so the workspace compiles.
  - T2 — Markdown descriptor gains `mdx`/`markdown`; add `text_backend()`
    (non-blank → `doc`); descriptor + doc-counting unit tests.
- **Milestone 2 — C-family crate (T3–T5, checkpoint T5)**
  - T3 — new `rloc-lang-cfamily` crate + parameterized scanner (`/* */` gated
    off for Zig).
  - T4 — six backends/descriptors (C=`c,h`, Cpp, Java, Swift, ObjectiveC=`m`, Zig).
  - T5 — scanner tests incl. the Zig `/* */`-is-not-a-comment regression.
- **Milestone 3 — XML & PowerShell (T6–T8, checkpoint T8)**
  - T6 — `xml_backend` in `rloc-lang-web` reusing the HTML markup scanner + test.
  - T7 — new `rloc-lang-powershell` crate + scanner (`#`, `<# #>`, string-aware).
  - T8 — PowerShell scanner tests incl. comment tokens inside strings & escapes.
- **Milestone 4 — CLI wiring & tests (T9–T12, checkpoint T12)**
  - T9 — register 9 backends in `default_registry()`; wiring test asserts 22.
  - T10 — extend `LanguageArg` + `From` + `--languages` doc + alias test.
  - T11 — positive detect integration test covering every extension→language map.
  - T12 — repair the `.txt`-now-supported fixtures; `cargo test --workspace`.
- **Milestone 5 — docs (T13, final flush)**
  - T13 — README language list + `--languages` aliases; clippy gate.

## Affected files

Expected create:
- `crates/rloc-lang-cfamily/Cargo.toml`
- `crates/rloc-lang-cfamily/src/classify.rs` — shared C-style line scanner.
- `crates/rloc-lang-cfamily/src/lib.rs` — 6 backends + descriptors.
- `crates/rloc-lang-cfamily/tests/classify.rs`
- `crates/rloc-lang-powershell/Cargo.toml`
- `crates/rloc-lang-powershell/src/classify.rs` — PowerShell scanner.
- `crates/rloc-lang-powershell/src/lib.rs`
- `crates/rloc-lang-powershell/tests/classify.rs`

Expected modify:
- `Cargo.toml` — add the two new crates to `[workspace].members`.
- `Cargo.lock` — regenerated when the two new workspace crates are first built
  (T3, T7); listed so it is committed rather than left dirty.
- `crates/rloc-core/src/types.rs` — 9 new `Language` variants + `as_str` arms.
- `crates/rloc-lang-js/src/parser.rs` — extend the two exhaustive `match language`
  fallback arms with the 9 new variants so the crate still compiles.
- `crates/rloc-core/src/text_backend.rs` — `mdx`/`markdown` on the Markdown
  descriptor; new `text_backend()`; unit tests.
- `crates/rloc-core/src/lib.rs` — re-export `text_backend`.
- `crates/rloc-lang-web/src/lib.rs` — `XmlBackend`, `XML_DESCRIPTOR`, `xml_backend()`.
- `crates/rloc-lang-web/src/classify.rs` — `Language::Xml` dispatch + reason arms.
- `crates/rloc-lang-web/tests/classify.rs` — XML case.
- `crates/rloc-cli/Cargo.toml` — dependencies on the two new crates.
- `crates/rloc-cli/src/commands/mod.rs` — register 9 backends; update wiring test.
- `crates/rloc-cli/src/cli.rs` — extend `LanguageArg` + `From<LanguageArg>` +
  `--languages` doc + alias parser test for `scan --languages` selection.
- `crates/rloc-cli/tests/detect_output.rs` — add the positive wave2 detect test;
  swap the `README.txt` unsupported fixture.
- `crates/rloc-cli/tests/exit_codes.rs` — swap `notes.txt` fixture.
- `crates/rloc-cli/tests/scan_output.rs` — swap `README.txt` fixture.
- `README.md` — extend the supported-language list and `--languages` aliases.

Do not modify:
- `crates/rloc-core/src/categories.rs` — category rules stay as-is; `.mdx`/
  `.markdown` already resolve to `docs`, new source extensions to `source`.
- `crates/rloc-core/src/registry.rs` — the registry mechanism is unchanged.
- `crates/rloc-core/src/analyze.rs` — its unsupported-extension unit tests use an
  isolated `test_registry()` (only a fake Rust backend), so `.txt` support does
  not affect them; leave them untouched.
- `crates/rloc-config/**` — no presets for the new languages (a non-goal).

## Decisions

- Decision: One shared `rloc-lang-cfamily` crate whose `classify.rs` is
  parameterized by `Language`, producing C / C++ / Java / Swift / Objective-C /
  Zig backends.
  - Reason: These languages share `//` line comments, `"..."` strings and
    `'...'` char literals (and `/* */` block comments for all but Zig — gated by
    a capability flag, see below); one scanner avoids six near-identical copies.
    Mirrors the existing `rloc-lang-web` crate, which shares one `classify.rs`
    for HTML and CSS.
  - Alternatives considered: one crate per language (rejected — heavy
    duplication); a generic token-driven scanner in `rloc-core` (rejected —
    concentrates the change in core and diverges from the per-crate pattern).
- Decision: `.h` is labelled `Language::C` (descriptor extensions `["c", "h"]`);
  `.m` is `Language::ObjectiveC` (extension `["m"]`).
  - Reason: The requested cluster (`.c`, `.cpp`, `.swift`) is the C/Apple family;
    `.m` as Objective-C and `.h` as a C header fit it. Chosen with the human.
  - Alternatives considered: `.m` as MATLAB (`%` comments) and a distinct
    C++-header language — rejected as not matching the request.
- Decision: Zig shares the C-family scanner but with `/* */` block-comment
  recognition disabled via a per-language capability flag.
  - Reason: Zig has no block comments — `/* */` is not Zig syntax — so a scanner
    that recognized it could misclassify a token sequence as a comment. Gating
    block comments off for Zig keeps the shared scanner while staying correct;
    Zig `//` line comments (including `///`/`//!`) count as `comment`, and `\\`
    multiline-string lines scan as code. A T5 regression test asserts `/* */`
    in a Zig file is not counted as a comment.
  - Alternatives considered: reusing the scanner unchanged (rejected — codex
    critique flagged it as incorrect in principle); a fully separate Zig scanner
    (rejected — the single capability flag is far cheaper).
- Decision: The nine new variants are also added to the two exhaustive
  `match language` fallback arms in `rloc-lang-js/src/parser.rs` (rather than
  introducing a `_ =>` wildcard).
  - Reason: `parser.rs` deliberately matches every `Language` without a wildcard
    so new languages force an explicit parser decision (all non-JS languages map
    to the JS-family fallback / error). Adding the variants to the existing
    fallback arms preserves that safety and keeps the workspace compiling; T1's
    verify is `cargo check --workspace --all-targets` to catch this crate-crossing
    breakage that a `-p rloc-core` build would miss.
  - Alternatives considered: a `_ =>` wildcard (rejected — defeats the codebase's
    intentional exhaustiveness guard).
- Decision: The new languages are made selectable on the CLI by extending
  `LanguageArg` and its `From<LanguageArg> for Language` conversion in `cli.rs`.
  - Reason: "Adding support" includes filtering with `scan --languages`; without
    this the languages would be advertised as supported but unselectable.
    `LanguageArg` matches on itself (not `Language`), so this is a feature gap,
    not a compile break — but it belongs in scope.
  - Alternatives considered: leaving `--languages` unchanged (rejected — partial,
    surprising support).
- Decision: `.txt` becomes a new `Language::Text` handled by a plain-text
  backend that counts every non-blank line as `doc` (reusing the Markdown code
  path in `text_backend.rs`); `.mdx` (and `.markdown`) are added to the existing
  Markdown descriptor rather than a new language.
  - Reason: `.txt` is prose, so `doc` is the natural bucket (chosen with the
    human); MDX is Markdown-with-JSX and reporting it as `markdown` avoids a
    near-empty new language. `categories.rs` already treats `.mdx`/`.markdown`
    as docs.
  - Alternatives considered: counting `.txt` as `code` (rejected by the human);
    a separate `Mdx` language (rejected — YAGNI).
- Decision: XML reuses the HTML markup scanner (`classify_html_line`) via a new
  `Language::Xml` dispatch arm in `rloc-lang-web`.
  - Reason: XML comments are `<!-- -->`, identical to HTML; no new scanner needed.
  - Alternatives considered: a standalone XML crate — rejected as redundant.
- Decision: New `Language` variants are appended before `Unknown` in the enum;
  `supported_languages()` still sorts descriptors by `as_str`, so display/JSON
  ordering stays alphabetical regardless of enum position.
  - Reason: Keeps derived `Ord` stable-ish and avoids reshuffling existing
    variants; the sort is by string key, which no test depends on by enum order.
  - Alternatives considered: alphabetizing the whole enum — rejected as noisy.

## Risks

- The simple line scanners do not model nested block comments (Swift), C++ raw
  string literals `R"(...)"`, Swift triple-quoted strings `"""..."""`, or
  PowerShell here-strings `@"..."@` / `@'...'@` and doubled-quote escaping
  (`""`, `''`). The PowerShell scanner DOES track ordinary `"..."` (with backtick
  escaping) and `'...'` strings so `#`/`<#` inside them are code (tested in T8);
  the excluded forms above can mis-classify a small number of lines and are
  accepted as documented limitations — consistent with the existing Go/CSS
  scanners.
- Expanding the `Language` enum breaks the two exhaustive `match language` blocks
  in `rloc-lang-js/src/parser.rs` (no wildcard arm), which would fail the whole
  workspace build. Mitigation: T1 updates those arms and verifies with
  `cargo check --workspace --all-targets`; the T2 checkpoint therefore lands a
  compiling workspace.
- Making `.txt` supported changes CLI integration-test fixtures that relied on
  `.txt` being unsupported (`exit_codes.rs`, `detect_output.rs`,
  `scan_output.rs`). Mitigation: T12 swaps them to a still-unsupported extension
  (`.dat`) and updates the assertions; `cargo test --workspace` gates it.
- The `default_registry()` wiring test asserts an exact language count (13 → 22).
  Forgetting one backend fails the test loudly — this is a feature, not a risk,
  but the count must be updated in lockstep (T9).
- Extension→language typos (e.g. omitting `h` from the C descriptor) would pass
  per-crate scanner tests. Mitigation: T11's positive detect integration test
  exercises every mapping through the real registry, including `.h`→C,
  `.m`→Objective-C, `.mdx`→Markdown.

## Open questions

- None — all scoping decisions were resolved during Phase 1 (fidelity, crate
  architecture, `.m` dialect, `.txt` semantics, TDD).

## Progress log

- 2026-07-13 — plan created.
- 2026-07-13 — T1 complete: added the nine Wave 2 language variants and kept the JS parser matches exhaustive; `cargo check --workspace --all-targets` passes.
- 2026-07-13 — T2 complete: added Text document counting and expanded Markdown extensions; `cargo test -p rloc-core` passes.
- 2026-07-13 — critique round 1 (`request_changes`): added `parser.rs`
  exhaustive-match fix (compile break), CLI `--languages` selection (T10),
  a positive detect integration test (T11), Zig `/* */` gating, and a
  checkpoint after the C-family crate.
- 2026-07-13 — critique round 2 (cap): codex `request_changes` — added
  `Cargo.lock` to affected files/T3/T7, strengthened PowerShell string/escape
  tests (T8) and Risks, and added this Task overview. Its "no task detail in
  body" finding reflects the critique renderer omitting `[[task]]` titles (the
  tasks are fully specified and pass `flow plan check`). Max review iterations
  reached; no further critique round.

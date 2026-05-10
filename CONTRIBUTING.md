# Contributing to lipstyk

## Adding a new language

Use `src/java/` as the template — it's the simplest language module (text-based regex, 4 rules).

Checklist:

1. **Register the language.** Add a variant to `Lang` in `src/source_rule.rs`, map file extensions in `Lang::from_ext()`, and add extensions to `SUPPORTED_EXTENSIONS` in `src/walk.rs`.

2. **Create the module.** Add `src/<lang>/mod.rs` and one file per rule. Each rule implements the `SourceRule` trait:
   ```rust
   impl SourceRule for MyRule {
       fn name(&self) -> &'static str { "lang-rule-name" }
       fn langs(&self) -> &[Lang] { &[Lang::MyLang] }
       fn check(&self, ctx: &SourceContext) -> Vec<Diagnostic> { ... }
   }
   ```

3. **Register rules.** Add `pub mod <lang>;` in `src/lib.rs` and register each rule in `Linter::with_defaults()` in `src/lint.rs`.

4. **Add tests.** Every rule needs a positive and negative case in `tests/rules.rs`:
   ```rust
   #[test]
   fn lang_my_rule_fires() {
       assert!(has_rule(SRC_THAT_TRIGGERS, "t.ext", "lang-rule-name"));
   }

   #[test]
   fn lang_my_rule_clean() {
       assert!(no_rule(CLEAN_SRC, "t.ext", "lang-rule-name"));
   }
   ```

5. **Add a fixture.** Add a file to `tests/fixtures/` that exercises the new rules for CLI integration tests.

## Text-based vs tree-sitter

Start text-based. If a rule later needs AST depth (nesting analysis, structural repetition, scope tracking), add a tree-sitter parser at that point. Python and Go followed this progression.

## Rule conventions

**Naming:** `<lang>-<rule-name>` for source rules (e.g. `elixir-bang-overuse`). Rust rules omit the prefix (e.g. `redundant-clone`).

**Severity and weight guidelines:**

| Severity | Weight range | When to use |
|----------|-------------|-------------|
| Hint     | 0.1 - 0.75  | Suspicious in aggregate but could be human. Low confidence. |
| Warning  | 1.0 - 2.0   | Likely AI pattern. High confidence with caveats. |
| Slop     | 2.0 - 3.0   | Strong structural signal. Reserve for patterns humans almost never produce. |

**Target AI patterns, not general lint.** lipstyk is not a linter — it detects machine-generated code. A rule should answer "would a human write this?" not "is this best practice?" If a pattern is common in experienced human code, it's not a good rule regardless of whether it's also common in AI code.

## Pull requests

- One feature or fix per PR.
- `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` must pass. CI checks all three plus runs lipstyk on itself.
- Include positive and negative test cases for every new rule.
- Keep the PR description concise — what changed and why.

## Running CI locally

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo run -- --exclude-tests --threshold 60 src/   # dogfood
```

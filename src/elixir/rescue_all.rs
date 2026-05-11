use crate::diagnostic::{Diagnostic, Severity};
use crate::source_rule::{Lang, SourceContext, SourceRule};

/// Flags catch-all `rescue` clauses. This is Elixir's equivalent of bare
/// `except:` in Python or `catch (Exception e)` in Java.
///
/// `rescue _ ->`, `rescue _e ->`, and `rescue exception ->` (where
/// `exception` matches no specific module) swallow every error,
/// including programmer bugs that should crash. Idiomatic Elixir
/// rescues specific exception structs or lets-it-crash.
pub struct RescueAll;

impl SourceRule for RescueAll {
    fn name(&self) -> &'static str {
        "elixir-rescue-all"
    }

    fn langs(&self) -> &[Lang] {
        &[Lang::Elixir]
    }

    fn check(&self, ctx: &SourceContext) -> Vec<Diagnostic> {
        let lines: Vec<&str> = ctx.source.lines().collect();
        let mut diagnostics = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            let rest = match trimmed.strip_prefix("rescue") {
                Some(rest) => rest.trim(),
                None => continue,
            };
            // Skip if `rescue` is part of a longer identifier.
            if !rest.is_empty() && !rest.starts_with(|c: char| !c.is_alphanumeric() && c != '_') {
                continue;
            }

            // `rescue` on its own line — the pattern is on the next line.
            let body = if rest.is_empty() {
                lines
                    .iter()
                    .skip(i + 1)
                    .map(|l| l.trim())
                    .find(|l| !l.is_empty())
                    .unwrap_or("")
            } else {
                rest
            };

            // A specific exception rescue looks like `MyError ->` or
            // `e in [Foo, Bar] ->` — those start with an uppercase
            // module name or use the `in` keyword.
            let is_catch_all = body.is_empty()
                || body.starts_with("_ ->")
                || body.starts_with("_e ->")
                || body.starts_with("->")
                || (!body.contains(" in ") && first_word_is_lowercase(body));

            if !is_catch_all {
                continue;
            }

            let next = lines.get(i + 1).map(|l| l.trim()).unwrap_or("");
            let is_swallowed = next.is_empty()
                || next == "end"
                || next == ":ok"
                || next == "nil"
                || next.starts_with("IO.")
                || next.starts_with("Logger.");

            let (severity, weight) = if is_swallowed {
                (Severity::Warning, 1.5)
            } else {
                (Severity::Hint, 0.75)
            };

            diagnostics.push(Diagnostic {
                rule: "elixir-rescue-all",
                message: format!("`{trimmed}` — rescue specific exception structs"),
                line: i + 1,
                severity,
                weight,
            });
        }

        diagnostics
    }
}

fn first_word_is_lowercase(s: &str) -> bool {
    s.chars()
        .next()
        .map(|c| c.is_lowercase() || c == '_')
        .unwrap_or(false)
}

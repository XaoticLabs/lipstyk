use crate::common::naming;
use crate::diagnostic::{Diagnostic, Severity};
use crate::source_rule::{Lang, SourceContext, SourceRule};

/// Flags generic function names in Elixir.
pub struct GenericNaming;

impl SourceRule for GenericNaming {
    fn name(&self) -> &'static str {
        "elixir-generic-naming"
    }

    fn langs(&self) -> &[Lang] {
        &[Lang::Elixir]
    }

    fn check(&self, ctx: &SourceContext) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        for (i, line) in ctx.source.lines().enumerate() {
            let trimmed = line.trim();
            let def_line = trimmed
                .strip_prefix("def ")
                .or_else(|| trimmed.strip_prefix("defp "))
                .or_else(|| trimmed.strip_prefix("defmacro "))
                .or_else(|| trimmed.strip_prefix("defmacrop "));

            if let Some(rest) = def_line {
                // Function name is everything up to `(`, ` `, or end of line.
                let name = rest
                    .split(|c: char| c == '(' || c.is_whitespace())
                    .next()
                    .unwrap_or("")
                    .trim_end_matches('?')
                    .trim_end_matches('!');

                if !name.is_empty() && naming::is_generic_name(name) {
                    diagnostics.push(Diagnostic {
                        rule: "elixir-generic-naming",
                        message: format!("`def {name}` — name is too vague to convey intent"),
                        line: i + 1,
                        severity: Severity::Warning,
                        weight: 1.5,
                    });
                }
            }
        }

        diagnostics
    }
}

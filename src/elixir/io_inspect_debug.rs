use crate::diagnostic::{Diagnostic, Severity};
use crate::source_rule::{Lang, SourceContext, SourceRule};

/// Flags `IO.inspect` / `IO.puts` debug calls left in production code.
///
/// Elixir's equivalent of `print()` / `console.log` — AI sprinkles
/// `IO.inspect` while developing and forgets to remove them. Exempts
/// files that look like scripts (`Mix.install`, `.exs` Mix tasks).
pub struct IoInspectDebug;

impl SourceRule for IoInspectDebug {
    fn name(&self) -> &'static str {
        "elixir-io-inspect-debug"
    }

    fn langs(&self) -> &[Lang] {
        &[Lang::Elixir]
    }

    fn check(&self, ctx: &SourceContext) -> Vec<Diagnostic> {
        // Exempt scripts and Mix tasks.
        if ctx.filename.ends_with(".exs") || ctx.source.contains("Mix.install") {
            return Vec::new();
        }

        let mut hits = Vec::new();

        for (i, line) in ctx.source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if trimmed.contains("IO.inspect")
                || trimmed.contains("IO.puts")
                || trimmed.contains("dbg(")
                || trimmed.ends_with("|> dbg")
            {
                hits.push(i + 1);
            }
        }

        if hits.is_empty() {
            return Vec::new();
        }

        vec![Diagnostic {
            rule: "elixir-io-inspect-debug",
            message: format!(
                "{} debug print call(s) — use Logger or remove debug output",
                hits.len()
            ),
            line: hits[0],
            severity: if hits.len() > 5 {
                Severity::Slop
            } else {
                Severity::Warning
            },
            weight: if hits.len() > 5 { 3.0 } else { 1.5 },
        }]
    }
}

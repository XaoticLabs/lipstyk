use crate::diagnostic::{Diagnostic, Severity};
use crate::source_rule::{Lang, SourceContext, SourceRule};

/// Flags overuse of bang (`!`) functions in Elixir.
///
/// Bang variants raise on failure. They bypass the
/// `{:ok, _} | {:error, _}` pattern that idiomatic Elixir leans on. AI
/// reaches for `File.read!`, `Map.fetch!`, etc. because they're terser,
/// at the cost of error handling.
pub struct BangOveruse;

const BANG_CALLS: &[&str] = &[
    "File.read!",
    "File.write!",
    "File.open!",
    "File.mkdir!",
    "File.mkdir_p!",
    "File.cp!",
    "File.rm!",
    "File.rm_rf!",
    "File.stat!",
    "Map.fetch!",
    "Map.update!",
    "Keyword.fetch!",
    "Keyword.update!",
    "String.to_existing_atom!",
    "Integer.parse!",
    "Float.parse!",
    "Jason.decode!",
    "Jason.encode!",
    "Poison.decode!",
    "Poison.encode!",
    "Repo.get!",
    "Repo.get_by!",
    "Repo.one!",
    "Repo.insert!",
    "Repo.update!",
    "Repo.delete!",
];

impl SourceRule for BangOveruse {
    fn name(&self) -> &'static str {
        "elixir-bang-overuse"
    }

    fn langs(&self) -> &[Lang] {
        &[Lang::Elixir]
    }

    fn check(&self, ctx: &SourceContext) -> Vec<Diagnostic> {
        let mut hits = Vec::new();

        for (i, line) in ctx.source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                continue;
            }
            if BANG_CALLS.iter().any(|c| trimmed.contains(c)) {
                hits.push(i + 1);
            }
        }

        if hits.len() < 3 {
            return Vec::new();
        }

        vec![Diagnostic {
            rule: "elixir-bang-overuse",
            message: format!(
                "{} bang (`!`) calls — prefer `{{:ok, _}} | {{:error, _}}` and `with` for control flow",
                hits.len()
            ),
            line: hits[0],
            severity: if hits.len() > 8 {
                Severity::Slop
            } else {
                Severity::Warning
            },
            weight: if hits.len() > 8 { 3.0 } else { 1.5 },
        }]
    }
}

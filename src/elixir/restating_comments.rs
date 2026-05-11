use crate::common::comment_analysis;
use crate::diagnostic::Diagnostic;
use crate::source_rule::{Lang, SourceContext, SourceRule};

/// Flags restating comments in Elixir.
pub struct RestatingComments;

impl SourceRule for RestatingComments {
    fn name(&self) -> &'static str {
        "elixir-restating-comment"
    }

    fn langs(&self) -> &[Lang] {
        &[Lang::Elixir]
    }

    fn check(&self, ctx: &SourceContext) -> Vec<Diagnostic> {
        comment_analysis::find_restating_comments(
            ctx.source,
            "#",
            "elixir-restating-comment",
            |trimmed| {
                let body = trimmed.trim_start_matches('#').trim();
                body.starts_with("!")        // shebangs
                    || body.starts_with("TODO")
                    || body.starts_with("FIXME")
                    || body.starts_with("HACK")
                    || body.starts_with("credo:") // credo directives
            },
        )
    }
}

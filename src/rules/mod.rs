pub mod boxed_error;
pub mod comment_clustering;
pub mod dead_code_markers;
pub mod derive_stacking;
pub mod error_swallowing;
pub mod generic_naming;
pub mod generic_todo;
pub mod index_loop;
pub mod naming_entropy;
pub mod needless_lifetimes;
pub mod needless_type_annotation;
pub mod over_documentation;
pub mod pub_overuse;
pub mod redundant_clone;
pub mod restating_comments;
pub mod string_params;
pub mod structural_repetition;
pub mod trivial_wrapper;
pub mod unwrap_overuse;
pub mod verbose_match;
pub mod whitespace_uniformity;

use crate::diagnostic::Diagnostic;

/// Context passed to each rule during analysis.
pub struct LintContext<'a> {
    pub filename: &'a str,
    pub source: &'a str,
    pub exclude_tests: bool,
}

/// Trait that all lint rules implement.
pub trait Rule: Send + Sync {
    /// Machine-readable rule name.
    fn name(&self) -> &'static str;

    /// Run the rule against a parsed file with context.
    fn check(&self, file: &syn::File, ctx: &LintContext) -> Vec<Diagnostic>;
}

/// Check whether a filename indicates a test module file.
///
/// Covers `tests.rs`, `test.rs`, `*_test.rs`, `*_tests.rs` — files
/// conventionally referenced via `#[cfg(test)] mod tests;` from a
/// parent module. Does NOT match by directory (e.g. `tests/`) because
/// that would also catch test fixtures and integration test helpers.
pub fn is_test_file(filename: &str) -> bool {
    let path = std::path::Path::new(filename);
    if path.extension().and_then(|e| e.to_str()) != Some("rs") {
        return false;
    }
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    stem == "tests" || stem == "test" || stem.ends_with("_test") || stem.ends_with("_tests")
}

/// Check whether a syn attribute list contains `#[test]`.
pub fn has_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("test"))
}

/// Check whether a syn attribute list contains `#[cfg(test)]`.
pub fn has_cfg_test_attr(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if !a.path().is_ident("cfg") {
            return false;
        }
        // Parse the token stream inside #[cfg(...)].
        let Ok(nested) = a.parse_args::<syn::Ident>() else {
            return false;
        };
        nested == "test"
    })
}

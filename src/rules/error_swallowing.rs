use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::{LintContext, Rule};
use syn::spanned::Spanned;
use syn::visit::Visit;

/// Flags silently swallowed errors.
///
/// AI code often matches on `Err(_)` with an empty body or just a
/// print/log statement, masking real failures. Also flags broad
/// `unwrap_or_default()` on Results where the default hides errors.
///
/// `.unwrap_or_default()` after `.ok()`, `.map_err()`, or on a known
/// Result-returning chain is flagged. On Option-returning chains
/// (`.get()`, `.first()`, `.last()`, `.find()`) it's suppressed.
pub struct ErrorSwallowing;

impl Rule for ErrorSwallowing {
    fn name(&self) -> &'static str {
        "error-swallowing"
    }

    fn check(&self, file: &syn::File, _ctx: &LintContext) -> Vec<Diagnostic> {
        let mut visitor = SwallowVisitor { hits: Vec::new() };
        visitor.visit_file(file);
        visitor.hits
    }
}

struct SwallowVisitor {
    hits: Vec<Diagnostic>,
}

fn is_wildcard_err_pattern(pat: &syn::Pat) -> bool {
    if let syn::Pat::TupleStruct(ts) = pat {
        let is_err = ts.path.segments.len() == 1 && ts.path.segments[0].ident == "Err";
        return is_err && ts.elems.iter().all(|p| matches!(p, syn::Pat::Wild(_)));
    }
    false
}

fn is_trivial_body(block: &syn::Block) -> bool {
    match block.stmts.as_slice() {
        [] => true,
        [syn::Stmt::Expr(expr, _)] => matches!(expr, syn::Expr::Macro(_)),
        _ => false,
    }
}

/// Check if the method chain leading to `.unwrap_or_default()` suggests
/// the receiver is a Result (flag it) vs an Option (suppress it).
fn looks_like_result_chain(node: &syn::ExprMethodCall) -> bool {
    let option_methods = [
        "get",
        "first",
        "last",
        "find",
        "position",
        "nth",
        "peek",
        // Path/OsStr methods returning Option — fallback is intentional
        "to_str",
        "file_name",
        "parent",
        "file_stem",
        "extension",
        "strip_prefix",
        "strip_suffix",
        // Iterator/collection lookups
        "min",
        "max",
        "next",
        "next_back",
    ];
    let result_methods = ["ok", "map_err", "or_else", "and_then"];

    let mut expr = node.receiver.as_ref();
    while let syn::Expr::MethodCall(call) = expr {
        let method = call.method.to_string();
        if result_methods.iter().any(|m| *m == method) {
            return true;
        }
        if option_methods.iter().any(|m| *m == method) {
            return false;
        }
        expr = call.receiver.as_ref();
    }

    // If the chain ends in a function call, check for benign Result
    // producers where defaulting is the correct handling (env::var, etc.)
    if let syn::Expr::Call(call) = expr {
        if let syn::Expr::Path(path) = call.func.as_ref() {
            if let Some(last) = path.path.segments.last() {
                let fn_name = last.ident.to_string();
                if matches!(
                    fn_name.as_str(),
                    "var" | "var_os" | "current_dir" | "current_exe"
                ) {
                    return false;
                }
            }
        }
    }

    // Unknown chain — flag conservatively but at low weight.
    true
}

impl<'ast> Visit<'ast> for SwallowVisitor {
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        for arm in &node.arms {
            if is_wildcard_err_pattern(&arm.pat) {
                let body_is_trivial = match arm.body.as_ref() {
                    syn::Expr::Block(block) => is_trivial_body(&block.block),
                    syn::Expr::Macro(_) => true,
                    syn::Expr::Tuple(tuple) if tuple.elems.is_empty() => true,
                    _ => false,
                };

                if body_is_trivial {
                    let line = arm.pat.span().start().line;
                    self.hits.push(Diagnostic {
                        rule: "error-swallowing",
                        message: "`Err(_)` is silently swallowed or only logged".to_string(),
                        line,
                        severity: Severity::Slop,
                        weight: 2.5,
                    });
                }
            }
        }

        syn::visit::visit_expr_match(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "unwrap_or_default" && looks_like_result_chain(node) {
            let line = node.method.span().start().line;
            self.hits.push(Diagnostic {
                rule: "error-swallowing",
                message: "`.unwrap_or_default()` may silently hide errors".to_string(),
                line,
                severity: Severity::Hint,
                weight: 0.75,
            });
        }

        syn::visit::visit_expr_method_call(self, node);
    }
}

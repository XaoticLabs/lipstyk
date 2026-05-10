use std::collections::{HashMap, HashSet};

use crate::diagnostic::{Diagnostic, Severity};
use crate::rules::{LintContext, Rule};
use syn::visit::Visit;

/// Detects functions with near-identical AST shapes within a file.
///
/// AI generates functions with the same structural skeleton: same number
/// of parameters, same body statement count, same control flow pattern.
/// A file full of structurally identical functions is a strong slop signal.
///
/// We hash each function's "shape" (param count, body statement count,
/// return type presence, control flow kind) and flag files with high
/// shape duplication, but ONLY when the functions also share significant
/// body-token similarity (>50% Jaccard overlap). Shape alone is not
/// sufficient — unrelated getters or constant functions share trivial shapes.
pub struct StructuralRepetition;

impl Rule for StructuralRepetition {
    fn name(&self) -> &'static str {
        "structural-repetition"
    }

    fn check(&self, file: &syn::File, _ctx: &LintContext) -> Vec<Diagnostic> {
        let mut visitor = ShapeVisitor {
            entries: Vec::new(),
        };
        visitor.visit_file(file);

        if visitor.entries.len() < 4 {
            return Vec::new();
        }

        let mut shape_counts: HashMap<FnShape, Vec<usize>> = HashMap::new();
        for (i, entry) in visitor.entries.iter().enumerate() {
            shape_counts
                .entry(entry.shape.clone())
                .or_default()
                .push(i);
        }

        let mut diagnostics = Vec::new();

        for (shape, indices) in &shape_counts {
            if indices.len() < 3 || shape.stmt_count == 0 {
                continue;
            }

            let names: Vec<&str> = indices
                .iter()
                .map(|&i| visitor.entries[i].name.as_str())
                .collect();

            if has_common_prefix(&names) || has_common_suffix(&names) {
                continue;
            }

            if !group_has_similarity(&visitor.entries, indices, 0.5) {
                continue;
            }

            let line = visitor.entries[indices[0]].line;
            diagnostics.push(Diagnostic {
                rule: "structural-repetition",
                message: format!(
                    "{} functions share the same shape ({} params, {} stmts): {}",
                    indices.len(),
                    shape.param_count,
                    shape.stmt_count,
                    names.join(", ")
                ),
                line,
                severity: Severity::Warning,
                weight: 1.5,
            });
        }

        diagnostics
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct FnShape {
    param_count: usize,
    stmt_count: usize,
    has_return_type: bool,
    has_if: bool,
    has_match: bool,
    has_loop: bool,
}

struct FnEntry {
    name: String,
    line: usize,
    shape: FnShape,
    body_tokens: Vec<String>,
}

struct ShapeVisitor {
    entries: Vec<FnEntry>,
}

fn shape_of_sig_and_block(sig: &syn::Signature, block: &syn::Block) -> FnShape {
    let param_count = sig.inputs.len();
    let stmt_count = block.stmts.len();
    let has_return_type = !matches!(sig.output, syn::ReturnType::Default);

    let mut has_if = false;
    let mut has_match = false;
    let mut has_loop = false;

    for stmt in &block.stmts {
        check_control_flow(stmt, &mut has_if, &mut has_match, &mut has_loop);
    }

    FnShape {
        param_count,
        stmt_count,
        has_return_type,
        has_if,
        has_match,
        has_loop,
    }
}

fn check_control_flow(
    stmt: &syn::Stmt,
    has_if: &mut bool,
    has_match: &mut bool,
    has_loop: &mut bool,
) {
    match stmt {
        syn::Stmt::Expr(expr, _) => check_expr_flow(expr, has_if, has_match, has_loop),
        syn::Stmt::Local(local) => {
            if let Some(init) = &local.init {
                check_expr_flow(&init.expr, has_if, has_match, has_loop);
            }
        }
        _ => {}
    }
}

fn check_expr_flow(expr: &syn::Expr, has_if: &mut bool, has_match: &mut bool, has_loop: &mut bool) {
    match expr {
        syn::Expr::If(_) => *has_if = true,
        syn::Expr::Match(_) => *has_match = true,
        syn::Expr::ForLoop(_) | syn::Expr::While(_) | syn::Expr::Loop(_) => *has_loop = true,
        _ => {}
    }
}

fn collect_body_tokens(block: &syn::Block) -> Vec<String> {
    let mut collector = TokenCollector {
        tokens: Vec::new(),
    };
    for stmt in &block.stmts {
        syn::visit::visit_stmt(&mut collector, stmt);
    }
    collector.tokens
}

struct TokenCollector {
    tokens: Vec<String>,
}

impl<'ast> Visit<'ast> for TokenCollector {
    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.tokens.push(node.method.to_string());
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(path) = node.func.as_ref() {
            if let Some(last) = path.path.segments.last() {
                self.tokens.push(last.ident.to_string());
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        if let Some(last) = node.mac.path.segments.last() {
            self.tokens.push(last.ident.to_string());
        }
        syn::visit::visit_expr_macro(self, node);
    }
}

fn group_has_similarity(entries: &[FnEntry], indices: &[usize], threshold: f64) -> bool {
    if indices.len() < 2 {
        return false;
    }

    let mut total_sim = 0.0;
    let mut pairs = 0usize;

    for i in 0..indices.len() {
        for j in (i + 1)..indices.len() {
            let a = &entries[indices[i]].body_tokens;
            let b = &entries[indices[j]].body_tokens;
            total_sim += jaccard(a, b);
            pairs += 1;
        }
    }

    if pairs == 0 {
        return false;
    }

    (total_sim / pairs as f64) >= threshold
}

fn jaccard(a: &[String], b: &[String]) -> f64 {
    // No tokens extracted — can't assess similarity, assume different
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }

    let set_a: HashSet<&str> = a.iter().map(String::as_str).collect();
    let set_b: HashSet<&str> = b.iter().map(String::as_str).collect();
    let intersection = set_a.intersection(&set_b).count();
    let union = set_a.union(&set_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Check if all function names share a common prefix (e.g. check_*, collect_*).
/// This indicates intentional decomposition, not slop repetition.
fn has_common_prefix(names: &[&str]) -> bool {
    if names.len() < 2 {
        return false;
    }

    let first = names[0];
    for prefix_len in (3..first.len()).rev() {
        let prefix = &first[..prefix_len];
        if !prefix.ends_with('_') {
            continue;
        }
        if names.iter().all(|n| n.starts_with(prefix)) {
            return true;
        }
    }

    false
}

/// Check if all function names share a common suffix (e.g. *_handler, *_endpoint).
fn has_common_suffix(names: &[&str]) -> bool {
    if names.len() < 2 {
        return false;
    }

    let first = names[0];
    let len = first.len();
    for suffix_len in (3..len).rev() {
        let suffix = &first[len - suffix_len..];
        if !suffix.starts_with('_') {
            continue;
        }
        if names.iter().all(|n| n.ends_with(suffix)) {
            return true;
        }
    }

    false
}

impl<'ast> Visit<'ast> for ShapeVisitor {
    fn visit_item_fn(&mut self, node: &'ast syn::ItemFn) {
        let name = node.sig.ident.to_string();
        let line = node.sig.ident.span().start().line;
        let shape = shape_of_sig_and_block(&node.sig, &node.block);
        let body_tokens = collect_body_tokens(&node.block);
        self.entries.push(FnEntry {
            name,
            line,
            shape,
            body_tokens,
        });
        syn::visit::visit_item_fn(self, node);
    }

    fn visit_impl_item_fn(&mut self, node: &'ast syn::ImplItemFn) {
        let name = node.sig.ident.to_string();
        // Skip trait method impls — visitor/handler patterns inherently repeat shapes.
        if name.starts_with("visit_") || name.starts_with("handle_") {
            return;
        }
        let line = node.sig.ident.span().start().line;
        let shape = shape_of_sig_and_block(&node.sig, &node.block);
        let body_tokens = collect_body_tokens(&node.block);
        self.entries.push(FnEntry {
            name,
            line,
            shape,
            body_tokens,
        });
        syn::visit::visit_impl_item_fn(self, node);
    }
}

//! `tracing_message_interpolation`: flags `tracing` event macros whose message has format
//! arguments (`info!("created {id}")`), so variable data always lands in queryable fields.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_data_structures::fx::FxHashSet;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass, LintContext};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::{ExpnKind, MacroKind, Span};

use crate::args::{inner, is_literal, split_top_level};

declare_lint! {
    /// ### What it does
    /// Flags `tracing` macros whose message string contains format arguments.
    ///
    /// ### Why is this bad?
    /// Variable data in the message cannot be queried or aggregated; fields can.
    ///
    /// ### Example
    /// ```rust,ignore
    /// tracing::info!("created item {}", id);
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// tracing::info!(item_id = %id, "item_created");
    /// ```
    pub TRACING_MESSAGE_INTERPOLATION,
    Warn,
    "tracing macro message contains format arguments"
}

#[derive(Default)]
pub struct TracingMessageInterpolation {
    seen: FxHashSet<Span>,
}

impl_lint_pass!(TracingMessageInterpolation => [TRACING_MESSAGE_INTERPOLATION]);

const EVENT_MACROS: &[&str] = &["trace", "debug", "info", "warn", "error", "event"];

impl<'tcx> LateLintPass<'tcx> for TracingMessageInterpolation {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if !expr.span.from_expansion() {
            return;
        }
        // The outermost expansion that is a tracing event macro is the user's call site.
        let call_site = expr
            .span
            .macro_backtrace()
            .filter(|expn| matches!(expn.kind, ExpnKind::Macro(MacroKind::Bang, _)))
            .filter(|expn| {
                expn.macro_def_id.is_some_and(|def_id| {
                    cx.tcx.crate_name(def_id.krate).as_str() == "tracing"
                        && EVENT_MACROS.contains(&cx.tcx.item_name(def_id).as_str())
                })
            })
            .last()
            .map(|expn| expn.call_site);
        let Some(call_site) = call_site else { return };
        if !self.seen.insert(call_site) {
            return;
        }
        let Ok(snippet) = cx.sess().source_map().span_to_snippet(call_site) else {
            return;
        };
        if message_has_args(&snippet) {
            span_lint_and_help(
                cx,
                TRACING_MESSAGE_INTERPOLATION,
                call_site,
                "tracing message contains format arguments",
                None,
                "Put variable data in fields, not the message: info!(item_id = %id, \"item_created\"). See docs/observability/fields.md.",
            );
        }
    }
}

/// True when the message literal has `{...}` placeholders or format arguments follow it.
fn message_has_args(snippet: &str) -> bool {
    let segments = split_top_level(inner(snippet));
    let Some(index) = segments.iter().position(|segment| is_literal(segment)) else {
        return false;
    };
    let message = &segments[index];
    message.replace("{{", "").contains('{') || segments.len() > index + 1
}

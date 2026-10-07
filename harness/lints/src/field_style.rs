//! `telemetry_field_style`: field keys in `tracing` macros and `#[instrument(fields(...))]`
//! must be `snake_case` or lowercase dotted OpenTelemetry names. Runs before macro expansion,
//! where the macro arguments and the `#[instrument]` attribute are still visible.

use clippy_utils::diagnostics::span_lint_and_help;
use rustc_ast::{AttrKind, Attribute, MacCall};
use rustc_ast_pretty::pprust;
use rustc_lint::{EarlyContext, EarlyLintPass};
use rustc_session::{declare_lint, impl_lint_pass};
use rustc_span::Span;

use crate::args::{field_key, is_literal, is_valid_key, split_top_level};

declare_lint! {
    /// ### What it does
    /// Flags telemetry field keys that are not `snake_case` or lowercase dotted names.
    ///
    /// ### Why is this bad?
    /// Field names become log fields, span attributes and metric labels; mixed styles make
    /// queries miss data.
    ///
    /// ### Example
    /// ```rust,ignore
    /// tracing::info!(itemId = %id, "item_created");
    /// ```
    /// Use instead:
    /// ```rust,ignore
    /// tracing::info!(item_id = %id, "item_created");
    /// ```
    pub TELEMETRY_FIELD_STYLE,
    Warn,
    "telemetry field key is not snake_case or a lowercase dotted OpenTelemetry name"
}

pub struct TelemetryFieldStyle;

impl_lint_pass!(TelemetryFieldStyle => [TELEMETRY_FIELD_STYLE]);

const EVENT_MACROS: &[&str] = &["trace", "debug", "info", "warn", "error", "event"];
const SPAN_MACROS: &[&str] = &[
    "trace_span",
    "debug_span",
    "info_span",
    "warn_span",
    "error_span",
    "span",
];

impl EarlyLintPass for TelemetryFieldStyle {
    fn check_mac(&mut self, cx: &EarlyContext<'_>, mac: &MacCall) {
        let segments: Vec<String> = mac
            .path
            .segments
            .iter()
            .map(|segment| segment.ident.name.to_string())
            .collect();
        let Some(name) = segments.last() else { return };
        if segments.len() > 1 && segments.first().is_some_and(|first| first != "tracing") {
            return;
        }
        let args = split_top_level(&pprust::tts_to_string(&mac.args.tokens));
        let literal = args.iter().position(|arg| is_literal(arg));
        let fields: &[String] = if EVENT_MACROS.contains(&name.as_str()) {
            args.get(..literal.unwrap_or(args.len()))
                .unwrap_or_default()
        } else if SPAN_MACROS.contains(&name.as_str()) {
            literal
                .and_then(|at| args.get(at + 1..))
                .unwrap_or_default()
        } else {
            return;
        };
        report(cx, mac.span(), fields);
    }

    fn check_attribute(&mut self, cx: &EarlyContext<'_>, attr: &Attribute) {
        let AttrKind::Normal(normal) = &attr.kind else {
            return;
        };
        if normal
            .item
            .path
            .segments
            .last()
            .is_none_or(|segment| segment.ident.name.as_str() != "instrument")
        {
            return;
        }
        let text = pprust::attribute_to_string(attr);
        let Some(start) = text.find("fields(") else {
            return;
        };
        let group = text.get(start + "fields(".len()..).unwrap_or_default();
        let mut depth = 1_i32;
        let end = group
            .char_indices()
            .find(|(_, ch)| {
                depth += match ch {
                    '(' => 1,
                    ')' => -1,
                    _ => 0,
                };
                depth == 0
            })
            .map_or(group.len(), |(at, _)| at);
        report(
            cx,
            attr.span,
            &split_top_level(group.get(..end).unwrap_or_default()),
        );
    }
}

fn report(cx: &EarlyContext<'_>, span: Span, args: &[String]) {
    for key in args
        .iter()
        .filter_map(|arg| field_key(arg))
        .filter(|key| !is_valid_key(key))
    {
        span_lint_and_help(
            cx,
            TELEMETRY_FIELD_STYLE,
            span,
            format!(
                "telemetry field `{key}` is not snake_case or a lowercase dotted OpenTelemetry name"
            ),
            None,
            "Rename it, e.g. `item_id` or `http.request.method`. See docs/observability/fields.md#naming.",
        );
    }
}

//! Project lints for `tracing` usage, loaded by `cargo dylint` (see `just dylint`).
//!
//! - `tracing_message_interpolation`: messages are constant strings; variable data goes in
//!   fields.
//! - `telemetry_field_style`: field keys are `snake_case` or lowercase dotted OpenTelemetry
//!   names.
//!
//! Both are documented in docs/observability/fields.md.
#![feature(rustc_private)]
#![warn(unused_extern_crates)]

extern crate rustc_ast;
extern crate rustc_ast_pretty;
extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

mod args;
mod field_style;
mod message;

dylint_linting::dylint_library!();

#[expect(
    clippy::no_mangle_with_rust_abi,
    reason = "dylint's registration entry point"
)]
#[unsafe(no_mangle)]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[
        message::TRACING_MESSAGE_INTERPOLATION,
        field_style::TELEMETRY_FIELD_STYLE,
    ]);
    lint_store.register_late_lint_pass(Box::new(|_| {
        Box::new(message::TracingMessageInterpolation::default())
    }));
    lint_store
        .register_pre_expansion_lint_pass(Box::new(|| Box::new(field_style::TelemetryFieldStyle)));
}

#[test]
fn ui() {
    dylint_testing::ui_test_example(env!("CARGO_PKG_NAME"), "ui");
}

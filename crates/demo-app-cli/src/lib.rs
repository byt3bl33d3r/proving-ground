//! Command-line client library: typed HTTP [`client`], [`commands`], and [`output`] (the
//! only module allowed to print). `main.rs` parses arguments and maps errors to exit codes.

pub mod args;
pub mod client;
pub mod commands;
pub mod error;
pub mod harness;
pub mod output;

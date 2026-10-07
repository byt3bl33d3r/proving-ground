//! Instruction-count benchmarks (Gungraun). Deterministic, so they gate: a regression of more
//! than 2% in instructions fails `just gungraun` (exit code 3). Linux only (Valgrind).
#![expect(
    missing_docs,
    unused_qualifications,
    clippy::exit,
    clippy::disallowed_macros,
    reason = "expanded from gungraun's macros (main! exits with the benchmark status)"
)]

use std::hint::black_box;

use demo_app_core::types::{ItemId, Page};
use gungraun::{
    Callgrind, EventKind, LibraryBenchmarkConfig, library_benchmark, library_benchmark_group, main,
};

#[library_benchmark]
#[bench::valid("itm_0123456789abcdef")]
#[bench::bad_prefix("item_0123456789abcd")]
fn parse_item_id(input: &str) -> bool {
    black_box(ItemId::parse(black_box(input))).is_ok()
}

#[library_benchmark]
#[bench::middle(1_000, 50, 10_000)]
#[bench::past_end(20_000, 100, 10_000)]
fn page_bounds(offset: u32, limit: u32, len: usize) -> Option<(usize, usize)> {
    let page = Page::new(Some(offset), Some(limit)).ok()?;
    Some(black_box(page.bounds(black_box(len))))
}

library_benchmark_group!(name = hot_paths, benchmarks = [parse_item_id, page_bounds]);

main!(
    config = LibraryBenchmarkConfig::default()
        .tool(Callgrind::default().soft_limits([(EventKind::Ir, 2.0)])),
    library_benchmark_groups = hot_paths
);

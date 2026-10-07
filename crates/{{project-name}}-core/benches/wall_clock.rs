//! Wall-clock benchmarks (Criterion). Informational only: noisy, never gates. A performance
//! change is accepted when Gungraun improves, tests pass, and Criterion agrees.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use {{crate_name}}_core::types::{ItemId, Page};

fn hot_paths(criterion: &mut Criterion) {
    criterion.bench_function("parse_item_id", |bencher| {
        bencher.iter(|| ItemId::parse(black_box("itm_0123456789abcdef")));
    });
    let page = Page::new(Some(1_000), Some(50)).ok();
    criterion.bench_function("page_bounds", |bencher| {
        bencher.iter(|| page.map(|page| page.bounds(black_box(10_000))));
    });
}

criterion_group!(benches, hot_paths);
criterion_main!(benches);

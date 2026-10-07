# Performance

Hot paths and benchmarks; Gungraun gates, Criterion informs.

Profile-guided optimization (`cargo pgo`) is optional and not run by any tier: build with
`cargo pgo build`, run a representative workload (for example `just e2e`), then
`cargo pgo optimize`. Use the `profiling` Cargo profile for profilers.

* [Benchmarks](benchmarks.md) - Instruction-count (Gungraun) and wall-clock (Criterion) benchmarks: which one gates and how baselines work.
* [Hot paths](hot-paths.md) - Functions whose assembly is snapshotted and whose instruction counts are benchmarked, and how to add one.

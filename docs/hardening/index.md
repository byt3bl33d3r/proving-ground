# Hardening

Check tiers, panic audit, Kani, Miri, fuzzing and sanitizers.

* [Fuzzing](fuzzing.md) - Fuzz targets, corpora and the crash workflow: reproduce, minimize, add a regression test, fix.
* [Kani proofs](kani.md) - Kani proof harnesses in core: the quick tier, the full tier, and why string-heavy proofs run nightly.
* [Miri](miri.md) - How Miri checks core's tests for undefined behaviour in tier 1, and which tests it skips.
* [Panic audit](panic-audit.md) - How the LLVM panic audit finds new panic paths in core's release build and how the allowlist works.
* [Sanitizers](sanitizers.md) - Which AddressSanitizer and ThreadSanitizer setup runs on each platform.
* [Check tiers](tiers.md) - The five check tiers (check, ci, perf, harden, release), what each runs and where it runs.

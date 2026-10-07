---
type: Guide
title: Sanitizers
description: Which AddressSanitizer and ThreadSanitizer setup runs on each platform.
tags: [hardening, asan, tsan, sanitizers]
status: stable
code_refs: [justfile]
---

# Sanitizers

- **ASan** (`just asan`): on x86_64 Linux the stable `x86_64-unknown-linux-gnuasan` target
  (no nightly needed); elsewhere `RUSTFLAGS=-Zsanitizer=address` with `$NIGHTLY` on the host
  target.
- **TSan** (`just tsan`): `RUSTFLAGS=-Zsanitizer=thread` with `$NIGHTLY`, `-Zbuild-std` and the
  host target, run through nextest (doctests break with mixed sanitizer flags). It needs a real
  x86_64 runner on Linux (emulation breaks TSan's memory layout).

Both cover core and server and use their own target directories. They run in
[tier 3](/hardening/tiers.md). Which variants were verified is in [template notes](/template/notes.md).

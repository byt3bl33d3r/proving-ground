# Testing

Test kinds, determinism rules and deterministic simulation.

When a database is added, integration tests use testcontainers (random ports, automatic cleanup)
and the DST model gains a storage fault layer behind the `ItemRepo` trait.

* [Determinism](determinism.md) - Why time, randomness, hashing order and network go through core::platform, and how tests stay deterministic.
* [Deterministic simulation testing](dst.md) - Deterministic simulation testing with turmoil, mad-turmoil and buggify: topology, faults, oracle, seeds and reproduction.
* [Test strategy](strategy.md) - The kinds of tests (unit, property, snapshot, golden logs, transcripts, architecture, e2e, DST) and where each one lives.

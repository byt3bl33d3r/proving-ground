//! DST entry points, run by `just dst` (never in git hooks). Seeds: `DST_SEED` for one, else
//! `DST_SEEDS` seeds starting at `DST_START` (default 200 from 0). Every failure ends with the
//! line `DST failure. Reproduce: just dst SEED=<n> TEST=<test>`.
#![cfg(test)]

use std::hash::{DefaultHasher, Hash as _, Hasher as _};
use std::io::Write as _;
use std::process::Command;

use dst::{Config, run, seed_all};

fn env(name: &str) -> Option<String> {
    #[expect(
        clippy::disallowed_methods,
        reason = "DST seeds come from the `just dst` recipe"
    )]
    std::env::var(name).ok()
}

fn number(name: &str, default: u64) -> u64 {
    env(name)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn seeds() -> Vec<u64> {
    if let Some(seed) = env("DST_SEED").and_then(|value| value.parse().ok()) {
        return vec![seed];
    }
    let start = number("DST_START", 0);
    (start..start.saturating_add(number("DST_SEEDS", 200))).collect()
}

fn fail(seed: u64, test: &str, reason: &str, log: &str) -> ! {
    let path = format!(
        "{}/target/dst-failure-{seed}.jsonl",
        env!("CARGO_MANIFEST_DIR")
    );
    let saved = std::fs::write(&path, log).is_ok();
    let mut stderr = std::io::stderr();
    let detail = writeln!(
        stderr,
        "seed {seed}: {reason}{}",
        if saved {
            format!(" (log: {path})")
        } else {
            String::new()
        }
    );
    drop(detail);
    panic!("DST failure. Reproduce: just dst SEED={seed} TEST={test}");
}

#[test]
fn items_survive_faults() {
    let _clock = mad_turmoil::time::SimClocksGuard::init();
    for seed in seeds() {
        seed_all(seed);
        if let Err(failure) = run(seed, &Config::default()) {
            fail(seed, "items_survive_faults", &failure.reason, &failure.log);
        }
    }
}

/// Normalized log hash for one seed: the timestamp field is dropped, everything else is kept.
fn log_hash(log: &str) -> String {
    let mut hasher = DefaultHasher::new();
    for line in log.lines() {
        let mut event: serde_json::Value = serde_json::from_str(line).unwrap_or_default();
        if let Some(object) = event.as_object_mut() {
            object.remove("timestamp");
        }
        event.to_string().hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

/// Child mode for `dst_is_deterministic`: writes one seed's log hash to `DST_LOG_OUT`.
#[test]
fn single_seed_log() {
    let Some(out) = env("DST_LOG_OUT") else {
        return;
    };
    let _clock = mad_turmoil::time::SimClocksGuard::init();
    let seed = number("DST_SEED", 7);
    seed_all(seed);
    let log = run(seed, &Config::default()).unwrap_or_else(|failure| failure.log);
    std::fs::write(out, log_hash(&log)).expect("write hash");
}

/// The same seed in two fresh processes must produce the same normalized log. If this fails,
/// a nondeterminism source leaked past core::platform: fix that first.
#[test]
fn dst_is_deterministic() {
    if env("DST_LOG_OUT").is_some() {
        return;
    }
    let seed = number("DST_SEED", 7);
    let hashes: Vec<String> = (0..2)
        .map(|attempt| {
            let out = format!(
                "{}/target/dst-hash-{seed}-{attempt}.txt",
                env!("CARGO_MANIFEST_DIR")
            );
            let status = Command::new(std::env::current_exe().expect("test binary"))
                .args(["single_seed_log", "--exact", "--test-threads=1"])
                .env("DST_SEED", seed.to_string())
                .env("DST_LOG_OUT", &out)
                .status()
                .expect("child run");
            assert!(status.success(), "child run for seed {seed} failed");
            std::fs::read_to_string(&out).expect("hash file")
        })
        .collect();
    if hashes.first() != hashes.last() {
        fail(
            seed,
            "dst_is_deterministic",
            &format!("log hashes differ: {hashes:?}"),
            "",
        );
    }
}

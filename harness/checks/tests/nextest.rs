//! Every nextest profile a workflow selects with `NEXTEST_PROFILE` must exist in each workspace
//! nextest runs in. The variable reaches `just dst` too, and nextest rejects an unknown profile
//! before running a single test.
#![cfg(test)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// The workspaces the justfile runs `cargo nextest` in.
const WORKSPACES: &[&str] = &[".", "harness/dst"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// Profiles named by `NEXTEST_PROFILE: <name>` in .github/workflows/*.yml.
fn workflow_profiles() -> BTreeSet<String> {
    let dir = root().join(".github/workflows");
    let mut profiles = BTreeSet::new();
    for entry in fs::read_dir(&dir)
        .expect("read .github/workflows")
        .flatten()
    {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "yml") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read workflow");
        for line in text.lines() {
            if let Some(name) = line.trim().strip_prefix("NEXTEST_PROFILE:") {
                profiles.insert(name.trim().trim_matches(['"', '\'']).to_owned());
            }
        }
    }
    profiles
}

#[test]
fn workflow_profiles_exist_in_every_workspace() {
    let profiles = workflow_profiles();
    assert!(
        !profiles.is_empty(),
        "no workflow sets NEXTEST_PROFILE; update this test if CI stopped selecting a profile"
    );
    for workspace in WORKSPACES {
        let path = root().join(workspace).join(".config/nextest.toml");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        let config: toml::Table = text
            .parse()
            .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()));
        let defined = config.get("profile").and_then(toml::Value::as_table);
        for profile in profiles.iter().filter(|name| *name != "default") {
            assert!(
                defined.is_some_and(|table| table.contains_key(profile)),
                "{} has no [profile.{profile}], but a workflow sets NEXTEST_PROFILE={profile}; \
                 add the profile there",
                path.display()
            );
        }
    }
}

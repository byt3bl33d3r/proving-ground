//! Every `just <recipe>` named in the Markdown agents read (docs/, the skills, AGENTS.md,
//! CLAUDE.md and README.md) must be a recipe, so the map never points at a command that does
//! not exist. Private recipes count: each check's `repro` line names them.
#![cfg(test)]

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Recipes that docs/decisions/adding-a-ui.md plans for a future UI; the template ships none.
const PLANNED_RECIPES: &[&str] = &["record", "ui-test", "ui-snap"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

/// The Markdown agents read: the docs/ bundle, the skills, and the root maps.
fn agent_docs() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root().join("docs"), root().join(".claude/skills")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }
    let maps = ["AGENTS.md", "CLAUDE.md", "README.md"].map(|name| root().join(name));
    files.extend(maps.into_iter().filter(|path| path.exists()));
    files.sort();
    files
}

/// Code in a Markdown text: fenced blocks (one entry per line) and inline backtick spans.
fn code_spans(text: &str) -> Vec<String> {
    let mut spans = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
        } else if fenced {
            spans.push(line.to_owned());
        } else {
            spans.extend(line.split('`').skip(1).step_by(2).map(str::to_owned));
        }
    }
    spans
}

/// Recipe names that follow `just` in one code span (`just dst SEED=7` names `dst`).
/// Flags (`just --list`) and placeholders (`just <recipe>`) are not names.
fn recipes_named(span: &str) -> Vec<String> {
    let tokens: Vec<&str> = span.split_whitespace().collect();
    tokens
        .windows(2)
        .filter(|pair| pair[0].trim_start_matches(['(', '$', '"', '\'']) == "just")
        .filter(|pair| !pair[1].starts_with(['-', '<']))
        .map(|pair| pair[1].trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-'))
        .filter(|name| {
            !name.is_empty()
                && name
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
        })
        .map(str::to_owned)
        .collect()
}

/// Every recipe, private ones included, from `just --dump`.
fn recipes() -> BTreeSet<String> {
    let output = Command::new("just")
        .args(["--dump", "--dump-format", "json"])
        .current_dir(root())
        .output()
        .expect("just runs (run the tests with `just test`, which puts mise's tools on PATH)");
    assert!(
        output.status.success(),
        "just --dump failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let dump: Value = serde_json::from_slice(&output.stdout).expect("just --dump JSON");
    dump["recipes"]
        .as_object()
        .expect("recipes")
        .keys()
        .cloned()
        .collect()
}

#[test]
fn docs_name_real_recipes() {
    let recipes = recipes();
    let mut violations = Vec::new();
    for file in agent_docs() {
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        for name in code_spans(&text)
            .iter()
            .flat_map(|span| recipes_named(span))
        {
            if !recipes.contains(&name) && !PLANNED_RECIPES.contains(&name.as_str()) {
                violations.push(format!(
                    "{} mentions `just {name}`, which is not a recipe. Fix the name or add the recipe; `just --list` shows the menu.",
                    file.strip_prefix(root()).unwrap_or(&file).display()
                ));
            }
        }
    }
    violations.sort();
    violations.dedup();
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

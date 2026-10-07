//! Architecture tests (protected file: ask before editing). Structure rules that clippy cannot
//! express: crate layering, module layering inside core, file and `main.rs` size, dependency
//! version drift, the `#[expect]` ledger, generated docs and the AGENTS.md map.
//! `just docs` runs these with `CHECKS_BLESS=1`, which rewrites `docs/generated/` instead of
//! comparing against it.
#![cfg(test)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// Crate roles (by directory name) and the internal crates each may depend on.
const LAYERS: &[(&str, &[&str])] = &[
    ("core", &[]),
    ("runtime", &["core"]),
    ("server", &["core", "runtime"]),
    ("cli", &["core", "runtime"]),
    ("checks", &["cli", "core"]),
];

/// Modules of the core crate and the modules each may use (`fields` is always allowed).
const CORE_MODULES: &[(&str, &[&str])] = &[
    ("types", &[]),
    ("platform", &["types"]),
    ("repo", &["types", "platform"]),
    ("domain", &["types", "platform", "repo"]),
];

/// Crate-root macros and the module that defines them.
const CORE_MACROS: &[(&str, &str)] = &[("buggify", "platform")];

const MAX_FILE_LINES: usize = 500;
const MAX_MAIN_LINES: usize = 80;
const MAX_AGENTS_LINES: usize = 120;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn bless() -> bool {
    #[expect(
        clippy::disallowed_methods,
        reason = "test-only switch used by `just docs`"
    )]
    std::env::var_os("CHECKS_BLESS").is_some()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn relative(path: &Path) -> String {
    path.strip_prefix(root())
        .unwrap_or(path)
        .display()
        .to_string()
}

/// Every `.rs` file under `crates/` and `harness/`, skipping build output and fuzz data.
fn rust_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root().join("crates"), root().join("harness")];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() && !matches!(name.as_str(), "target" | "corpus" | "artifacts") {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn metadata() -> Value {
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .current_dir(root())
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata JSON")
}

/// Package name to role, for workspace members that have one.
fn roles(meta: &Value) -> BTreeMap<String, &'static str> {
    let packages = meta["packages"].as_array().expect("packages");
    packages
        .iter()
        .filter_map(|package| {
            let dir = Path::new(package["manifest_path"].as_str()?)
                .parent()?
                .file_name()?
                .to_str()?
                .to_owned();
            let role = LAYERS
                .iter()
                .map(|(role, _)| *role)
                .find(|role| dir == *role || dir.ends_with(&format!("-{role}")))?;
            Some((package["name"].as_str()?.to_owned(), role))
        })
        .collect()
}

/// Internal dependency edges (package name to package names), every dependency kind.
fn internal_edges(meta: &Value) -> BTreeMap<String, BTreeSet<String>> {
    let roles = roles(meta);
    let mut edges: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for package in meta["packages"].as_array().expect("packages") {
        let from = package["name"].as_str().expect("name").to_owned();
        let deps = package["dependencies"].as_array().expect("dependencies");
        let internal = deps
            .iter()
            .filter_map(|dep| dep["name"].as_str())
            .filter(|name| roles.contains_key(*name));
        edges
            .entry(from)
            .or_default()
            .extend(internal.map(str::to_owned));
    }
    edges
}

#[test]
fn crates_depend_only_downward() {
    let meta = metadata();
    let roles = roles(&meta);
    let name_of = |role: &str| {
        roles
            .iter()
            .find(|(_, owner)| **owner == role)
            .map_or_else(|| role.to_owned(), |(name, _)| name.clone())
    };
    let mut violations = Vec::new();
    for (from, deps) in internal_edges(&meta) {
        let Some(role) = roles.get(&from) else {
            continue;
        };
        let allowed: &[&str] = LAYERS
            .iter()
            .find(|(layer, _)| layer == role)
            .map_or(&[], |(_, allowed)| allowed);
        for to in deps.iter().filter(|to| !allowed.contains(&roles[*to])) {
            let names: Vec<String> = allowed
                .iter()
                .map(|allowed_role| name_of(allowed_role))
                .collect();
            violations.push(format!(
                "Forbidden dependency: {from} -> {to}. Allowed for {from}: {names:?}. See docs/architecture/layers.md. Move the shared code down a layer instead."
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Identifiers right after one `crate::` (handles `use crate::{a, b::c}` groups).
fn idents_after(rest: &str) -> Vec<String> {
    let names = rest
        .strip_prefix('{')
        .map_or(rest, |group| group.split('}').next().unwrap_or(group));
    names
        .split(',')
        .map(|item| {
            item.trim()
                .chars()
                .take_while(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect::<String>()
        })
        .filter(|ident| !ident.is_empty())
        .collect()
}

/// Modules named after `crate::` anywhere in `text`, comments excluded.
fn crate_references(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(|line| {
            line.match_indices("crate::")
                .filter_map(move |(index, _)| line.get(index + "crate::".len()..))
        })
        .flat_map(idents_after)
        .collect()
}

#[test]
fn core_modules_respect_layering() {
    let src = rust_files()
        .into_iter()
        .find(|path| path.ends_with("src/lib.rs") && relative(path).contains("-core/"));
    let src = src
        .expect("core crate")
        .parent()
        .expect("src dir")
        .to_path_buf();
    let mut violations = Vec::new();
    for file in rust_files()
        .into_iter()
        .filter(|path| path.starts_with(&src))
    {
        let rel = file.strip_prefix(&src).expect("under src");
        let first = rel
            .components()
            .next()
            .expect("component")
            .as_os_str()
            .to_string_lossy();
        let module = first.trim_end_matches(".rs");
        let Some((_, allowed)) = CORE_MODULES.iter().find(|(name, _)| *name == module) else {
            continue;
        };
        for referenced in crate_references(&read(&file)) {
            let target = CORE_MACROS
                .iter()
                .find(|(name, _)| *name == referenced)
                .map_or(referenced.as_str(), |(_, owner)| owner);
            let is_module = CORE_MODULES.iter().any(|(name, _)| *name == target);
            if is_module && target != module && !allowed.contains(&target) {
                violations.push(format!(
                    "Forbidden module dependency in {}: core::{module} -> core::{target}. Allowed for core::{module}: {allowed:?} (and fields). See docs/architecture/layers.md. Move the shared code down a layer instead.",
                    relative(&file)
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

#[test]
fn files_stay_small() {
    let mut violations = Vec::new();
    for file in rust_files() {
        let lines = read(&file).lines().count();
        if lines > MAX_FILE_LINES {
            violations.push(format!(
                "{} has {lines} lines (limit {MAX_FILE_LINES}). Split it by responsibility; see docs/conventions/modules.md.",
                relative(&file)
            ));
        }
        if file.ends_with("src/main.rs")
            && relative(&file).starts_with("crates/")
            && lines >= MAX_MAIN_LINES
        {
            violations.push(format!(
                "{} has {lines} lines (limit {MAX_MAIN_LINES}). Move logic into the library; main.rs only wires config, telemetry and I/O.",
                relative(&file)
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

fn version_of(spec: &toml::Value) -> Option<String> {
    spec.as_str()
        .or_else(|| spec.get("version")?.as_str())
        .map(|version| version.trim_start_matches('=').to_owned())
}

/// Drift between one separate workspace's manifest and `[workspace.dependencies]`.
fn drift(dir: &str, pinned: &toml::Table) -> Vec<String> {
    let manifest = root().join(dir).join("Cargo.toml");
    if !manifest.exists() {
        return Vec::new();
    }
    let parsed: toml::Value = toml::from_str(&read(&manifest)).expect("manifest");
    let sections = ["dependencies", "dev-dependencies"]
        .into_iter()
        .filter_map(|section| parsed.get(section)?.as_table());
    sections
        .flat_map(|deps| deps.iter())
        .filter_map(|(name, spec)| Some((name, version_of(spec)?, version_of(pinned.get(name)?)?)))
        .filter(|(_, theirs, ours)| theirs != ours)
        .map(|(name, theirs, ours)| {
            format!("Version drift: {dir} pins {name} {theirs} but [workspace.dependencies] pins {ours}. Use the same version in both (they upgrade together).")
        })
        .collect()
}

#[test]
fn separate_workspaces_pin_the_same_versions() {
    let workspace: toml::Value =
        toml::from_str(&read(&root().join("Cargo.toml"))).expect("Cargo.toml");
    let pinned = workspace["workspace"]["dependencies"]
        .as_table()
        .expect("[workspace.dependencies]");
    let violations: Vec<String> = ["harness/dst", "harness/fuzz"]
        .into_iter()
        .flat_map(|dir| drift(dir, pinned))
        .collect();
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// One line per `#[expect(...)]` attribute: `path: lints -- reason`.
fn lint_exceptions() -> String {
    let mut lines = Vec::new();
    for file in rust_files() {
        let text = read(&file);
        let mut offset = 0;
        for line in text.split_inclusive('\n') {
            let trimmed = line.trim_start();
            if trimmed.starts_with("#[expect(") || trimmed.starts_with("#![expect(") {
                let start = offset + line.len() - trimmed.len();
                let attribute = text
                    .get(start..)
                    .unwrap_or_default()
                    .split(")]")
                    .next()
                    .unwrap_or_default();
                let body = attribute.split_once("expect(").map_or("", |(_, body)| body);
                let flat = body.split_whitespace().collect::<Vec<_>>().join(" ");
                let (lints, reason) = flat.split_once("reason").unwrap_or((&flat, ""));
                let reason = reason
                    .trim_start_matches([' ', '='])
                    .trim_matches(['"', ',', ' ']);
                lines.push(format!(
                    "{}: {} -- {reason}",
                    relative(&file),
                    lints.trim().trim_end_matches(',')
                ));
            }
            offset += line.len();
        }
    }
    lines.join("\n") + "\n"
}

/// Internal crate graph, one line per package.
fn crate_graph() -> String {
    internal_edges(&metadata())
        .into_iter()
        .map(|(from, to)| {
            format!(
                "{from} -> {}",
                to.into_iter().collect::<Vec<_>>().join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Compares (or, under `CHECKS_BLESS=1`, rewrites) a generated file below its header comments.
fn check_generated(name: &str, fresh: &str, hint: &str) {
    let path = root().join("docs/generated").join(name);
    let current = fs::read_to_string(&path).unwrap_or_default();
    let header_lines: Vec<&str> = current
        .lines()
        .take_while(|line| line.starts_with('#'))
        .collect();
    let header = if header_lines.is_empty() {
        String::new()
    } else {
        header_lines.join("\n") + "\n"
    };
    let body = current.get(header.len()..).unwrap_or_default();
    if bless() {
        fs::write(&path, format!("{header}{fresh}")).expect("write generated file");
    } else {
        assert_eq!(body, fresh, "docs/generated/{name} is out of date. {hint}");
    }
}

#[test]
fn lint_exceptions_are_recorded() {
    check_generated(
        "lint-exceptions.txt",
        &lint_exceptions(),
        "Every #[expect(...)] must be listed: review the new exception, then run `just docs` and commit the file in the same PR.",
    );
}

#[test]
fn crate_graph_is_current() {
    check_generated(
        "crate-graph.txt",
        &crate_graph(),
        "Run `just docs` and commit the result.",
    );
}

/// Path-like tokens in AGENTS.md that must exist (runtime paths such as `.harness/` excluded).
fn agents_paths(text: &str) -> BTreeSet<String> {
    text.split(|ch: char| ch.is_whitespace() || "`()|,;\"'".contains(ch))
        .map(|token| token.trim_end_matches(['.', ':']))
        .filter(|token| token.contains('/') && !token.contains("://"))
        .filter(|token| !token.contains(['<', '*', '$', '{']))
        .filter(|token| !token.starts_with(".harness") && !token.starts_with("target/"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn agents_map_is_short_and_accurate() {
    let text = read(&root().join("AGENTS.md"));
    let lines = text.lines().count();
    assert!(
        lines <= MAX_AGENTS_LINES,
        "AGENTS.md has {lines} lines (limit {MAX_AGENTS_LINES}). Move detail into docs/ and link it."
    );
    let missing: Vec<String> = agents_paths(&text)
        .into_iter()
        .filter(|path| !root().join(path).exists())
        .collect();
    assert!(
        missing.is_empty(),
        "AGENTS.md mentions paths that do not exist: {missing:?}. Fix the map or create them."
    );
}

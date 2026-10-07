//! Assembly snapshots of the hot paths listed in docs/performance/hot-paths.md. Run by
//! `just asm-snapshots` (not `just test`: it needs a release build and cargo-show-asm).
//! A diff fails until reviewed with `cargo insta review`. Snapshots are for one canonical target
//! (`x86_64` Linux, what CI runs) on every host: `cargo asm --lib` compiles without linking, so a
//! Mac produces the same snapshot as CI.
#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn hot_paths() -> Vec<String> {
    let doc = std::fs::read_to_string(root().join("docs/performance/hot-paths.md"))
        .expect("hot-paths.md");
    doc.lines()
        .skip_while(|line| !line.starts_with("```text hot-paths"))
        .skip(1)
        .take_while(|line| !line.starts_with("```"))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn core_package() -> String {
    let crates = std::fs::read_dir(root().join("crates")).expect("crates dir");
    crates
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .find(|name| name.ends_with("-core"))
        .expect("core crate")
}

#[test]
fn hot_path_assembly_is_reviewed() {
    let package = core_package();
    let target = "x86_64-unknown-linux-gnu";
    for function in hot_paths() {
        let output = Command::new("cargo")
            .args([
                "asm",
                "--lib",
                "-p",
                &package,
                "--target",
                target,
                "--simplify",
                &function,
            ])
            .current_dir(root())
            .output()
            .expect("cargo asm runs (mise install provides cargo-show-asm)");
        let asm = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success() && !asm.is_empty(),
            "no assembly for {function}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let name: String = function
            .chars()
            .map(|ch| if ch.is_alphanumeric() { ch } else { '_' })
            .collect();
        insta::with_settings!({
            snapshot_suffix => target,
            filters => vec![
                (r"\.?Lfunc_begin\d+", "[func_begin]"),
                (r"\.?LBB\d+_(\d+)", "[bb_$1]"),
                (r"\.?Ltmp\d+", "[tmp]"),
                (r"\.?Lloh\d+", "[loh]"),
                (r"l_anon\.[0-9a-f]+\.\d+", "[anon]"),
                (r"h[0-9a-f]{16}", "[hash]"),
            ],
        }, {
            insta::assert_snapshot!(name.trim_matches('_'), asm);
        });
    }
}

//! CLI transcripts in `tests/cmd/*.toml` (trycmd). Regenerate with `TRYCMD=overwrite`.
//! Cases that need a server live in `harness/checks/tests/e2e.rs`.
#![cfg(test)]

#[test]
fn transcripts() {
    trycmd::TestCases::new().case("tests/cmd/*.toml");
}

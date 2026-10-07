//! End-to-end tests against the server started by `just up` (run with `just e2e`).
#![cfg(test)]

#[test]
fn placeholder() {
    let answer = 2_u8.checked_add(2);
    assert_eq!(answer, Some(4), "replaced in milestone 4");
}

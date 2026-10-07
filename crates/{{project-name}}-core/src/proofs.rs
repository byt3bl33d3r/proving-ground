//! Kani proof harnesses (compiled only by `cargo kani`). `quick_*` run in tier 1 (`just kani`),
//! everything runs in tier 3 (`just kani-full`). See docs/hardening/kani.md.

use crate::types::{ItemId, MAX_PAGE_LIMIT, Page, hex_value};

/// Parsing any short input never panics.
#[kani::proof]
#[kani::unwind(8)]
fn quick_parse_item_id_never_panics() {
    let bytes: [u8; 6] = kani::any();
    if let Ok(text) = core::str::from_utf8(&bytes) {
        let parsed = ItemId::parse(text);
        kani::cover!(parsed.is_err(), "short inputs are rejected");
    }
}

/// A validated page always has a limit in `1..=MAX_PAGE_LIMIT`.
#[kani::proof]
#[kani::unwind(8)]
fn quick_page_upholds_invariant() {
    let (offset, limit): (Option<u32>, Option<u32>) = (kani::any(), kani::any());
    if let Ok(page) = Page::new(offset, limit) {
        assert!(
            (1..=MAX_PAGE_LIMIT).contains(&page.limit()),
            "limit in range"
        );
        assert!(
            offset.is_none_or(|value| value == page.offset()),
            "offset kept"
        );
    }
}

/// Page bounds never overflow and stay inside the collection.
#[kani::proof]
#[kani::unwind(8)]
fn quick_pagination_never_overflows() {
    let (offset, limit, len): (u32, u32, usize) = (kani::any(), kani::any(), kani::any());
    if let Ok(page) = Page::new(Some(offset), Some(limit)) {
        let (start, end) = page.bounds(len);
        assert!(start <= end && end <= len, "bounds inside the collection");
        let _next = Page::next_offset(end, len);
    }
}

/// The hex-digit decoder behind `ItemId::parse` accepts exactly `0-9a-f`, with the right value,
/// for every possible byte (tier 3). Kept byte-level on purpose: harnesses that build symbolic
/// strings make CBMC run out of memory, so full-id round trips are left to proptest and the
/// `parse_item_id` fuzz target.
#[kani::proof]
fn full_hex_digit_decoding_is_exact() {
    let byte: u8 = kani::any();
    let expected = char::from(byte)
        .to_digit(16)
        .filter(|_| !byte.is_ascii_uppercase());
    assert_eq!(
        hex_value(byte).map(u32::from),
        expected,
        "decoder matches lowercase hex"
    );
}

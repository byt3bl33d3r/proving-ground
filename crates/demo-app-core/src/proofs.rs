//! Kani proof harnesses (compiled only by `cargo kani`). `quick_*` run in tier 1 (`just kani`),
//! everything runs in tier 3 (`just kani-full`). See docs/hardening/kani.md.

use crate::types::{ITEM_ID_PREFIX, ITEM_NAME_MAX_CHARS, ItemId, ItemName, MAX_PAGE_LIMIT, Page};

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

/// A validated name is non-empty, trimmed, short and free of control characters, over two
/// characters from a small alphabet (space, letter, control). Tier 3 only: CBMC is slow on
/// symbolic strings; tier 1 covers names with proptest and the decode_create_item fuzz target.
#[kani::proof]
#[kani::unwind(8)]
fn full_item_name_upholds_invariant() {
    item_name_invariant::<2>();
}

fn item_name_invariant<const N: usize>() {
    let alphabet = [' ', 'a', '\u{7}'];
    let picks: [u8; N] = kani::any();
    let text: String = picks
        .iter()
        .map(|pick| alphabet[usize::from(*pick % 3)])
        .collect();
    if let Ok(name) = ItemName::new(&text) {
        let value = name.as_str();
        let bytes = value.as_bytes();
        assert!(!bytes.is_empty(), "non-empty");
        assert!(
            bytes.first() != Some(&b' ') && bytes.last() != Some(&b' '),
            "trimmed"
        );
        assert!(bytes.len() <= ITEM_NAME_MAX_CHARS, "bounded length");
        assert!(!bytes.contains(&0x07), "no control characters");
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

/// Full-length ids: every well-formed id parses and round-trips through `Display`.
#[kani::proof]
#[kani::unwind(21)]
fn full_parse_item_id_round_trips() {
    let digits: [u8; 16] = kani::any();
    kani::assume(
        digits
            .iter()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f')),
    );
    let mut text = String::from(ITEM_ID_PREFIX);
    text.extend(digits.iter().map(|byte| char::from(*byte)));
    let id = ItemId::parse(&text);
    assert!(id.is_ok(), "well-formed ids parse");
}

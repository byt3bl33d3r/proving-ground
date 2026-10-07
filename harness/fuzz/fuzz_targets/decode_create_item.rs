//! Decoding a `POST /items` JSON body never panics, and any name the domain accepts upholds the
//! `ItemName` invariant and survives a JSON round trip.
#![no_main]

use demo_app_core::types::{CreateItem, ITEM_NAME_MAX_CHARS, ItemName};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(request) = serde_json::from_slice::<CreateItem>(data) else {
        return;
    };
    let Ok(name) = ItemName::new(&request.name) else {
        return;
    };
    let value = name.as_str();
    assert!(
        !value.is_empty() && value.trim_ascii() == value,
        "trimmed and non-empty"
    );
    assert!(
        value.chars().count() <= ITEM_NAME_MAX_CHARS,
        "bounded length"
    );
    assert!(
        !value.chars().any(char::is_control),
        "no control characters"
    );
    let json = serde_json::to_string(&name).expect("names serialize");
    assert_eq!(
        serde_json::from_str::<ItemName>(&json).ok(),
        Some(name),
        "JSON round trip"
    );
});

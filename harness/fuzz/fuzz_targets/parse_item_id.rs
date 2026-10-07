//! `ItemId::parse` never panics, and every id it accepts round-trips through `Display`.
#![no_main]

use demo_app_core::types::ItemId;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let Ok(text) = std::str::from_utf8(data) else {
        return;
    };
    if let Ok(id) = ItemId::parse(text) {
        assert_eq!(
            ItemId::parse(&id.to_string()),
            Ok(id),
            "accepted ids round-trip"
        );
    }
});

//! Domain types: validated newtypes, items, pagination and errors. Uses no other module.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Prefix of the textual form of an [`ItemId`].
pub const ITEM_ID_PREFIX: &str = "itm_";
/// Number of lowercase hex digits after [`ITEM_ID_PREFIX`].
pub const ITEM_ID_HEX_DIGITS: usize = 16;
/// Longest accepted item name, in characters.
pub const ITEM_NAME_MAX_CHARS: usize = 64;
/// Page size used when a request gives none.
pub const DEFAULT_PAGE_LIMIT: u32 = 50;
/// Largest accepted page size.
pub const MAX_PAGE_LIMIT: u32 = 100;

/// Item identifier: 64 random bits shown as `itm_` followed by 16 lowercase hex digits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ItemId(u64);

impl ItemId {
    /// Wraps raw bits (from `platform::Rng`).
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Parses the textual form. Never panics, whatever the input. Hot path: kept out of line so
    /// its assembly is snapshotted (docs/performance/hot-paths.md).
    #[inline(never)]
    pub fn parse(input: &str) -> Result<Self, ParseIdError> {
        let hex = input
            .strip_prefix(ITEM_ID_PREFIX)
            .ok_or(ParseIdError::MissingPrefix)?;
        if hex.len() != ITEM_ID_HEX_DIGITS {
            return Err(ParseIdError::Length { actual: hex.len() });
        }
        hex.bytes()
            .try_fold(0_u64, |acc, byte| {
                acc.checked_mul(16)?
                    .checked_add(u64::from(hex_value(byte)?))
            })
            .map(Self)
            .ok_or(ParseIdError::InvalidDigit)
    }
}

/// Value of one lowercase hex digit, `None` for any other byte.
pub(crate) const fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => match byte.checked_sub(b'a') {
            Some(low) => low.checked_add(10),
            None => None,
        },
        _ => None,
    }
}

impl fmt::Display for ItemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{ITEM_ID_PREFIX}{:016x}", self.0)
    }
}

impl TryFrom<String> for ItemId {
    type Error = ParseIdError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

impl From<ItemId> for String {
    fn from(id: ItemId) -> Self {
        id.to_string()
    }
}

/// Why an [`ItemId`] failed to parse.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ParseIdError {
    /// The input does not start with `itm_`.
    #[error("item id must start with \"{ITEM_ID_PREFIX}\"")]
    MissingPrefix,
    /// Wrong number of hex digits.
    #[error("item id must have {ITEM_ID_HEX_DIGITS} hex digits after the prefix, got {actual}")]
    Length {
        /// Digits found after the prefix.
        actual: usize,
    },
    /// A character outside `0-9a-f`.
    #[error("item id may only contain lowercase hex digits after the prefix")]
    InvalidDigit,
}

/// Item name: trimmed of ASCII whitespace, 1 to 64 characters, no control characters.
/// (ASCII trimming keeps the Kani proof of this invariant fast; see docs/hardening/kani.md.)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ItemName(String);

impl ItemName {
    /// Validates and trims `raw`.
    pub fn new(raw: &str) -> Result<Self, ValidationError> {
        let trimmed = raw.trim_ascii();
        if trimmed.is_empty() {
            return Err(ValidationError::EmptyName);
        }
        if trimmed.chars().take(ITEM_NAME_MAX_CHARS + 1).count() > ITEM_NAME_MAX_CHARS {
            return Err(ValidationError::NameTooLong {
                max: ITEM_NAME_MAX_CHARS,
            });
        }
        if trimmed.chars().any(char::is_control) {
            return Err(ValidationError::NameControlChars);
        }
        Ok(Self(trimmed.to_owned()))
    }

    /// The validated name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for ItemName {
    type Error = ValidationError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(&value)
    }
}

impl From<ItemName> for String {
    fn from(name: ItemName) -> Self {
        name.0
    }
}

/// Milliseconds since the Unix epoch, as reported by `platform::Clock`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Timestamp(u64);

impl Timestamp {
    /// Wraps milliseconds since the Unix epoch.
    pub const fn from_millis(millis: u64) -> Self {
        Self(millis)
    }

    /// Milliseconds since the Unix epoch.
    pub const fn as_millis(self) -> u64 {
        self.0
    }
}

/// A stored item.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Item {
    /// Identifier.
    pub id: ItemId,
    /// Name.
    pub name: ItemName,
    /// Creation time.
    pub created_at_ms: Timestamp,
}

/// Request body for creating an item. The name is validated by the domain, not by serde,
/// so a bad name is a validation error rather than a decode error.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateItem {
    /// Requested name.
    pub name: String,
}

/// A validated page request: `limit` items starting at `offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Page {
    offset: u32,
    limit: u32,
}

impl Page {
    /// Validates the query parameters; `limit` defaults to [`DEFAULT_PAGE_LIMIT`].
    pub fn new(offset: Option<u32>, limit: Option<u32>) -> Result<Self, ValidationError> {
        let limit = limit.unwrap_or(DEFAULT_PAGE_LIMIT);
        if limit == 0 || limit > MAX_PAGE_LIMIT {
            return Err(ValidationError::Limit {
                max: MAX_PAGE_LIMIT,
            });
        }
        Ok(Self {
            offset: offset.unwrap_or(0),
            limit,
        })
    }

    /// Index of the first item.
    pub const fn offset(self) -> u32 {
        self.offset
    }

    /// Maximum number of items.
    pub const fn limit(self) -> u32 {
        self.limit
    }

    /// The `start..end` slice bounds for a collection of `len` items. Never overflows and
    /// always satisfies `start <= end <= len`. Hot path (docs/performance/hot-paths.md).
    #[inline(never)]
    pub fn bounds(self, len: usize) -> (usize, usize) {
        let start = usize::try_from(self.offset).unwrap_or(usize::MAX).min(len);
        let limit = usize::try_from(self.limit).unwrap_or(usize::MAX);
        (start, start.saturating_add(limit).min(len))
    }

    /// Offset of the following page, if items remain after `end`.
    pub fn next_offset(end: usize, len: usize) -> Option<u32> {
        if end < len {
            u32::try_from(end).ok()
        } else {
            None
        }
    }
}

/// One page of a listing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemPage {
    /// Items in id order.
    pub items: Vec<Item>,
    /// Offset of the next page, absent on the last page.
    pub next_offset: Option<u32>,
}

/// Input that breaks a domain rule.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ValidationError {
    /// Empty or whitespace-only name.
    #[error("name must not be empty")]
    EmptyName,
    /// Name longer than the limit.
    #[error("name must be at most {max} characters")]
    NameTooLong {
        /// The limit.
        max: usize,
    },
    /// Name with control characters.
    #[error("name must not contain control characters")]
    NameControlChars,
    /// Page size out of range.
    #[error("limit must be between 1 and {max}")]
    Limit {
        /// The largest accepted limit.
        max: u32,
    },
    /// Malformed item id.
    #[error(transparent)]
    Id(#[from] ParseIdError),
}

/// Failure of a domain operation.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ItemError {
    /// The request broke a domain rule.
    #[error("invalid input: {0}")]
    Validation(#[from] ValidationError),
    /// No item has this id.
    #[error("item {0} not found")]
    NotFound(ItemId),
    /// A transient failure; the same request may succeed when retried.
    #[error("temporarily unavailable, retry the request")]
    Unavailable,
}

impl ItemError {
    /// Stable machine-readable code, used as `error.type` and in API error bodies.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation_error",
            Self::NotFound(_) => "not_found",
            Self::Unavailable => "unavailable",
        }
    }
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    #[test]
    fn item_id_round_trips_through_text() {
        let id = ItemId::from_bits(0x0123_4567_89ab_cdef);
        assert_eq!(id.to_string(), "itm_0123456789abcdef", "display form");
        assert_eq!(ItemId::parse("itm_0123456789abcdef"), Ok(id), "parse form");
    }

    #[test]
    fn item_id_rejects_bad_input() {
        assert_eq!(
            ItemId::parse("0123456789abcdef"),
            Err(ParseIdError::MissingPrefix),
            "prefix"
        );
        assert_eq!(
            ItemId::parse("itm_12"),
            Err(ParseIdError::Length { actual: 2 }),
            "length"
        );
        assert_eq!(
            ItemId::parse("itm_0123456789ABCDEF"),
            Err(ParseIdError::InvalidDigit),
            "case"
        );
    }

    #[test]
    fn item_name_is_trimmed_and_validated() {
        assert_eq!(
            ItemName::new("  pen ").map(|n| n.0),
            Ok("pen".to_owned()),
            "trimmed"
        );
        assert_eq!(ItemName::new(" "), Err(ValidationError::EmptyName), "empty");
        assert_eq!(
            ItemName::new("a\u{7}b"),
            Err(ValidationError::NameControlChars),
            "control"
        );
        let long = "x".repeat(ITEM_NAME_MAX_CHARS + 1);
        assert_eq!(
            ItemName::new(&long),
            Err(ValidationError::NameTooLong {
                max: ITEM_NAME_MAX_CHARS
            }),
            "too long"
        );
    }

    #[test]
    fn values_serialize_as_strings_and_numbers() {
        let item = Item {
            id: ItemId::from_bits(1),
            name: ItemName::new("pen").expect("valid"),
            created_at_ms: Timestamp::from_millis(5),
        };
        let json = serde_json::to_value(&item).expect("serialize");
        assert_eq!(
            json,
            serde_json::json!({ "id": "itm_0000000000000001", "name": "pen", "created_at_ms": 5 }),
            "wire shape"
        );
        assert_eq!(
            serde_json::from_value::<Item>(json).ok(),
            Some(item),
            "round trip"
        );
    }

    #[test]
    fn error_codes_are_stable() {
        assert_eq!(
            ItemError::Validation(ValidationError::EmptyName).code(),
            "validation_error",
            "validation"
        );
        assert_eq!(
            ItemError::NotFound(ItemId::from_bits(0)).code(),
            "not_found",
            "not found"
        );
        assert_eq!(ItemError::Unavailable.code(), "unavailable", "unavailable");
    }

    #[test]
    fn page_keeps_offset_and_limit() {
        let page = Page::new(Some(7), Some(3)).expect("valid");
        assert_eq!((page.offset(), page.limit()), (7, 3), "accessors");
        assert_eq!(
            Page::new(None, None).map(Page::offset),
            Ok(0),
            "default offset"
        );
    }

    #[test]
    fn page_rejects_out_of_range_limits() {
        assert!(Page::new(None, Some(0)).is_err(), "zero limit");
        assert!(
            Page::new(None, Some(MAX_PAGE_LIMIT + 1)).is_err(),
            "limit above max"
        );
        assert_eq!(
            Page::new(None, None).map(Page::limit),
            Ok(DEFAULT_PAGE_LIMIT),
            "default"
        );
    }

    proptest! {
        #[cfg_attr(miri, ignore = "proptest persists failures to files")]
        #[test]
        fn parse_never_panics_and_round_trips(input in ".*", bits in any::<u64>()) {
            let _parsed: Result<ItemId, ParseIdError> = ItemId::parse(&input);
            let id = ItemId::from_bits(bits);
            prop_assert_eq!(ItemId::parse(&id.to_string()), Ok(id), "round trip");
        }

        #[cfg_attr(miri, ignore = "proptest persists failures to files")]
        #[test]
        fn page_bounds_stay_in_range(offset in any::<u32>(), limit in 1..=MAX_PAGE_LIMIT, len in 0_usize..10_000) {
            let page = Page::new(Some(offset), Some(limit)).map_err(|e| TestCaseError::fail(e.to_string()))?;
            let (start, end) = page.bounds(len);
            prop_assert!(start <= end && end <= len, "start {} end {} len {}", start, end, len);
        }
    }
}

//! Differential test: random operation sequences go to the real `ItemService` and to a
//! `BTreeMap` model, which must agree on every result.
#![no_main]

use std::collections::BTreeMap;
use std::sync::Arc;

use arbitrary::Arbitrary;
use {{crate_name}}_core::domain::ItemService;
use {{crate_name}}_core::platform::{SeededRng, TokioClock};
use {{crate_name}}_core::repo::InMemoryItemRepo;
use {{crate_name}}_core::types::{Item, ItemError, ItemId, Page, Timestamp};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
enum Op {
    Create(String),
    Get(u8),
    Delete(u8),
    List { offset: u8, limit: u8 },
}

#[derive(Arbitrary, Debug)]
struct Input {
    seed: u64,
    ops: Vec<Op>,
}

/// An id that exists in the model when there is one, otherwise a fixed unknown id.
fn pick(model: &BTreeMap<ItemId, Item>, index: u8) -> ItemId {
    let len = model.len().max(1);
    model
        .keys()
        .nth(usize::from(index) % len)
        .copied()
        .unwrap_or(ItemId::from_bits(u64::from(index)))
}

async fn run(input: Input) {
    let service = ItemService::new(
        Arc::new(InMemoryItemRepo::default()),
        Arc::new(TokioClock::new(Timestamp::from_millis(0))),
        Arc::new(SeededRng::new(input.seed)),
    );
    let mut model: BTreeMap<ItemId, Item> = BTreeMap::new();
    for op in input.ops.into_iter().take(64) {
        match op {
            Op::Create(name) => match service.create(&name).await {
                Ok(item) => assert!(model.insert(item.id, item).is_none(), "fresh id"),
                Err(err) => assert!(
                    matches!(err, ItemError::Validation(_)),
                    "only validation fails: {err}"
                ),
            },
            Op::Get(index) => {
                let id = pick(&model, index);
                assert_eq!(
                    service.get(&id.to_string()).await.ok(),
                    model.get(&id).cloned(),
                    "get agrees"
                );
            }
            Op::Delete(index) => {
                let id = pick(&model, index);
                assert_eq!(
                    service.delete(&id.to_string()).await.is_ok(),
                    model.remove(&id).is_some(),
                    "delete agrees"
                );
            }
            Op::List { offset, limit } => {
                let listed = service.list(Some(offset.into()), Some(limit.into())).await;
                match Page::new(Some(offset.into()), Some(limit.into())) {
                    Ok(page) => {
                        let (start, end) = page.bounds(model.len());
                        let expected: Vec<Item> = model
                            .values()
                            .skip(start)
                            .take(end - start)
                            .cloned()
                            .collect();
                        assert_eq!(
                            listed.map(|page| page.items).ok(),
                            Some(expected),
                            "list agrees"
                        );
                    }
                    Err(_) => assert!(listed.is_err(), "invalid pages are rejected"),
                }
            }
        }
    }
}

fuzz_target!(|input: Input| {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime");
    runtime.block_on(run(input));
});

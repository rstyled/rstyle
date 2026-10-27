//! Exactly one observation per owning unit; errors and absence stay distinct.
use crate::HirFnFacts;
use std::{cell::OnceCell, sync::Arc};

pub(crate) fn single<T>(
    functions: &[HirFnFacts],
    read: impl Fn(&HirFnFacts) -> Result<Option<Arc<T>>, String>,
    duplicate: &str,
) -> Result<Option<Arc<T>>, String> {
    let mut selected = None;
    for facts in functions {
        let Some(record) = read(facts)? else {
            continue;
        };
        if selected.is_some() {
            return Err(duplicate.into());
        }
        selected = Some(record);
    }
    Ok(selected)
}

pub(crate) fn retained<T: Clone>(
    slot: &OnceCell<Result<T, String>>,
    read: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    slot.get_or_init(read).clone()
}

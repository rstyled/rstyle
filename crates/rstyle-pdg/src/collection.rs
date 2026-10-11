//! Shared collection of optional graph records.

/// Preserve iteration order and multiplicity while omitting absent records.
pub fn collect_present<T, U>(
    items: impl IntoIterator<Item = T>,
    select: impl FnMut(T) -> Option<U>,
) -> Vec<U> {
    items.into_iter().filter_map(select).collect()
}

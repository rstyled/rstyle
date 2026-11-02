//! First-match indexes for document inventories.

use std::collections::BTreeMap;

/// Index `entries` by `key`, keeping the first entry for each key, so a lookup returns
/// what a linear `find` over the same inventory would. Malformed inventories with repeated
/// keys keep that behavior; complete document validation still rejects them.
pub(crate) fn first_by_key<'a, T>(
    entries: &'a [T],
    key: impl Fn(&'a T) -> &'a str,
) -> BTreeMap<&'a str, &'a T> {
    let mut index = BTreeMap::new();
    for entry in entries {
        index.entry(key(entry)).or_insert(entry);
    }
    index
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_the_first_entry_for_each_key() {
        let entries = [("a", 1), ("b", 2), ("a", 3)];
        let index = first_by_key(&entries, |entry| entry.0);
        assert_eq!(index.len(), 2);
        assert_eq!(index["a"].1, 1);
        assert_eq!(index["b"].1, 2);
    }
}

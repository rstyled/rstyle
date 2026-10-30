//! Shared staleness compares full ordered identities; no hashes or source copies.
use super::{Locator, StaleReason, Versions};

pub(super) fn stale_reason<'a, C: PartialEq + 'a, I, J>(
    old_versions: &Versions,
    new_versions: &Versions,
    previous: I,
    current: J,
) -> Option<StaleReason>
where
    I: Iterator<Item = (&'a Locator, &'a C)> + Clone,
    J: Iterator<Item = (&'a Locator, &'a C)> + Clone,
{
    let overlap = previous
        .clone()
        .any(|(old, _)| current.clone().any(|(new, _)| old == new));
    if !overlap {
        return None;
    }
    if old_versions != new_versions {
        return Some(StaleReason::Versions);
    }

    if previous
        .clone()
        .map(|(locator, _)| locator)
        .ne(current.clone().map(|(locator, _)| locator))
    {
        return Some(StaleReason::Membership);
    }
    if previous.zip(current).any(|((_, old), (_, new))| old != new) {
        return Some(StaleReason::Context);
    }
    Some(StaleReason::Body)
}

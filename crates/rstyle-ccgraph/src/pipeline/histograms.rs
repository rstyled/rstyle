//! Readable labels exist only at the stage-detail presentation boundary.
use std::{collections::BTreeMap, fmt};

pub(super) struct Readable<'a>(pub &'a BTreeMap<u64, u64>);

impl fmt::Debug for Readable<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut entries: Vec<_> = self
            .0
            .iter()
            .map(|(label, count)| (format!("L{label}"), count))
            .collect();
        entries.sort_unstable_by(|first, second| first.0.cmp(&second.0));

        formatter.debug_map().entries(entries).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detail_rendering_retains_lexical_label_order_and_bytes() {
        let histogram = BTreeMap::from([(2, 5), (10, 3), (1, 2), (u64::MAX, 4)]);
        let original: BTreeMap<_, _> = histogram
            .iter()
            .map(|(label, count)| (format!("L{label}"), *count))
            .collect();
        assert_eq!(
            format!("{:?}", Readable(&histogram)),
            format!("{original:?}")
        );
        assert_eq!(format!("{:?}", Readable(&BTreeMap::new())), "{}");
    }
}

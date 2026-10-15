//! Complete refinement keys; lookup hashes never replace equality.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct WlTuple {
    pub(crate) label: u64,
    pub(crate) incoming: Vec<(u8, u64)>,
    pub(crate) outgoing: Vec<(u8, u64)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        hash::{BuildHasherDefault, Hasher},
    };

    #[derive(Default)]
    struct Colliding;

    impl Hasher for Colliding {
        fn finish(&self) -> u64 {
            0
        }
        fn write(&mut self, _: &[u8]) {}
    }

    #[test]
    fn lookup_collisions_preserve_complete_refinement_tuple_equality() {
        let base = WlTuple {
            label: 1,
            incoming: vec![(0, 7)],
            outgoing: Vec::new(),
        };
        let cases = [
            base.clone(),
            WlTuple {
                label: 2,
                ..base.clone()
            },
            WlTuple {
                incoming: vec![(1, 7)],
                ..base.clone()
            },
            WlTuple {
                incoming: vec![(0, 8)],
                ..base.clone()
            },
            WlTuple {
                incoming: vec![(0, 7), (0, 7)],
                ..base.clone()
            },
            WlTuple {
                incoming: Vec::new(),
                outgoing: vec![(0, 7)],
                ..base.clone()
            },
        ];
        let mut table = HashMap::<_, _, BuildHasherDefault<Colliding>>::default();
        for (index, tuple) in cases.iter().enumerate() {
            let next = table.len();
            assert_eq!(*table.entry(tuple.clone()).or_insert(next), index);
        }
        for (index, tuple) in cases.iter().enumerate() {
            let next = table.len();
            assert_eq!(*table.entry(tuple.clone()).or_insert(next), index);
        }
        assert_eq!(table.len(), cases.len());
    }
}

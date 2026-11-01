#[derive(Clone, PartialEq, Eq)]
pub(super) struct Bits {
    words: Vec<u64>,
}

impl Bits {
    pub fn empty(size: usize) -> Self {
        Self {
            words: vec![0; size.div_ceil(64)],
        }
    }

    pub fn full(size: usize) -> Self {
        let mut result = Self {
            words: vec![u64::MAX; size.div_ceil(64)],
        };
        if !size.is_multiple_of(64) {
            *result.words.last_mut().expect("partial word") = (1 << (size % 64)) - 1;
        }
        result
    }

    pub fn insert(&mut self, bit: usize) {
        self.words[bit / 64] |= 1 << (bit % 64);
    }

    pub fn remove(&mut self, bit: usize) {
        self.words[bit / 64] &= !(1 << (bit % 64));
    }

    pub fn contains(&self, bit: usize) -> bool {
        self.words[bit / 64] & (1 << (bit % 64)) != 0
    }

    pub fn intersect(&mut self, other: &Self) {
        for (word, other) in self.words.iter_mut().zip(&other.words) {
            *word &= other;
        }
    }

    pub fn union(&mut self, other: &Self) {
        for (word, other) in self.words.iter_mut().zip(&other.words) {
            *word |= other;
        }
    }

    pub fn count(&self) -> u32 {
        self.words.iter().copied().map(u64::count_ones).sum()
    }

    pub fn members(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(index, &word)| {
            let mut remaining = word;
            std::iter::from_fn(move || {
                if remaining == 0 {
                    return None;
                }
                let bit = remaining.trailing_zeros() as usize;
                remaining &= remaining - 1;
                Some(index * 64 + bit)
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Bits;

    #[test]
    fn word_boundaries_preserve_exact_membership() {
        for size in [0, 1, 63, 64, 65, 127, 128, 129] {
            let full = Bits::full(size);
            assert_eq!(
                full.members().collect::<Vec<_>>(),
                (0..size).collect::<Vec<_>>()
            );
            assert_eq!(full.count() as usize, size);
            let mut evens = Bits::empty(size);
            for bit in (0..size).step_by(2) {
                evens.insert(bit);
            }
            let mut intersection = full.clone();
            intersection.intersect(&evens);
            assert_eq!(
                intersection.members().collect::<Vec<_>>(),
                (0..size).step_by(2).collect::<Vec<_>>()
            );
            let mut complement = full.clone();
            for bit in (0..size).step_by(2) {
                complement.remove(bit);
            }
            complement.union(&evens);
            assert_eq!(
                complement.members().collect::<Vec<_>>(),
                full.members().collect::<Vec<_>>()
            );
        }
    }
}

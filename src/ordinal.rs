use std::ops::{Bound, RangeBounds};

pub type Ordinal = u32;

const WORDS: usize = 4;
const BITS_PER_WORD: u32 = u64::BITS;
const MAX_ORDINAL_COUNT: u32 = WORDS as u32 * BITS_PER_WORD;

#[derive(Clone, Debug, Eq)]
pub enum OrdinalSet {
    All {
        min: Ordinal,
        max: Ordinal,
    },
    Explicit {
        min: Ordinal,
        max: Ordinal,
        bits: [u64; WORDS],
        count: u32,
    },
}

impl OrdinalSet {
    pub fn empty(min: Ordinal, max: Ordinal) -> Self {
        Self::assert_valid_domain(min, max);
        Self::Explicit {
            min,
            max,
            bits: [0; WORDS],
            count: 0,
        }
    }

    pub fn all(min: Ordinal, max: Ordinal) -> Self {
        Self::assert_valid_domain(min, max);
        Self::All { min, max }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_iter<I>(min: Ordinal, max: Ordinal, ordinals: I) -> Self
    where
        I: IntoIterator<Item = Ordinal>,
    {
        let mut set = Self::empty(min, max);
        for ordinal in ordinals {
            set.insert(ordinal);
        }
        set
    }

    pub fn insert(&mut self, ordinal: Ordinal) -> bool {
        let (min, max) = self.domain();
        assert!(
            Self::ordinal_is_in_domain(min, max, ordinal),
            "ordinal {ordinal} outside supported domain {min}-{max}"
        );

        match self {
            Self::All { .. } => false,
            Self::Explicit {
                bits,
                count,
                min,
                max,
            } => {
                let (word, mask) = Self::word_and_mask(*min, ordinal);
                if bits[word] & mask != 0 {
                    return false;
                }

                bits[word] |= mask;
                *count += 1;
                if *count == Self::domain_len(*min, *max) {
                    *self = Self::All {
                        min: *min,
                        max: *max,
                    };
                }
                true
            }
        }
    }

    pub fn union_assign(&mut self, other: &Self) {
        let (min, max) = self.domain();
        assert_eq!(
            (min, max),
            other.domain(),
            "cannot union ordinal sets with different domains"
        );

        match (&mut *self, other) {
            (Self::All { .. }, _) => {}
            (_, Self::All { .. }) => *self = Self::All { min, max },
            (
                Self::Explicit { bits, count, .. },
                Self::Explicit {
                    bits: other_bits, ..
                },
            ) => {
                for (word, other_word) in bits.iter_mut().zip(other_bits.iter()) {
                    *word |= *other_word;
                }
                *count = bits.iter().map(|word| word.count_ones()).sum();
                self.normalize();
            }
        }
    }

    pub fn contains(&self, ordinal: Ordinal) -> bool {
        let (min, max) = self.domain();
        if !Self::ordinal_is_in_domain(min, max, ordinal) {
            return false;
        }

        match self {
            Self::All { .. } => true,
            Self::Explicit { bits, min, .. } => {
                let (word, mask) = Self::word_and_mask(*min, ordinal);
                bits[word] & mask != 0
            }
        }
    }

    pub fn count(&self) -> u32 {
        match self {
            Self::All { min, max } => Self::domain_len(*min, *max),
            Self::Explicit { count, .. } => *count,
        }
    }

    pub fn len(&self) -> usize {
        self.count() as usize
    }

    pub fn is_all(&self) -> bool {
        matches!(self, Self::All { .. })
    }

    pub fn iter(&self) -> OrdinalSetIter<'_> {
        self.range(..)
    }

    pub fn range<R>(&self, range: R) -> OrdinalSetIter<'_>
    where
        R: RangeBounds<Ordinal>,
    {
        let Some((front, back)) = self.range_bounds(range) else {
            return OrdinalSetIter::empty();
        };

        match self {
            Self::All { .. } => OrdinalSetIter::all(front, back),
            Self::Explicit { min, max, bits, .. } => {
                OrdinalSetIter::explicit(*min, *max, bits, front, back)
            }
        }
    }

    pub fn next_ge(&self, ordinal: Ordinal) -> Option<Ordinal> {
        let (min, max) = self.domain();
        match self {
            Self::All { .. } => {
                if ordinal <= min {
                    Some(min)
                } else if ordinal <= max {
                    Some(ordinal)
                } else {
                    None
                }
            }
            Self::Explicit { bits, .. } => Self::next_ge_in_bits(min, max, bits, ordinal),
        }
    }

    pub fn prev_le(&self, ordinal: Ordinal) -> Option<Ordinal> {
        let (min, max) = self.domain();
        match self {
            Self::All { .. } => {
                if ordinal < min {
                    None
                } else if ordinal >= max {
                    Some(max)
                } else {
                    Some(ordinal)
                }
            }
            Self::Explicit { bits, .. } => Self::prev_le_in_bits(min, max, bits, ordinal),
        }
    }

    fn normalize(&mut self) {
        let convert_to_all = match self {
            Self::All { .. } => false,
            Self::Explicit {
                min, max, count, ..
            } => *count == Self::domain_len(*min, *max),
        };

        if convert_to_all {
            let (min, max) = self.domain();
            *self = Self::All { min, max };
        }
    }

    fn domain(&self) -> (Ordinal, Ordinal) {
        match self {
            Self::All { min, max } | Self::Explicit { min, max, .. } => (*min, *max),
        }
    }

    fn range_bounds<R>(&self, range: R) -> Option<(Ordinal, Ordinal)>
    where
        R: RangeBounds<Ordinal>,
    {
        let (min, max) = self.domain();
        let front = match range.start_bound() {
            Bound::Included(&ordinal) => {
                if ordinal > max {
                    return None;
                }
                ordinal.max(min)
            }
            Bound::Excluded(&ordinal) => {
                if ordinal >= max {
                    return None;
                }
                ordinal.saturating_add(1).max(min)
            }
            Bound::Unbounded => min,
        };

        let back = match range.end_bound() {
            Bound::Included(&ordinal) => {
                if ordinal < min {
                    return None;
                }
                ordinal.min(max)
            }
            Bound::Excluded(&ordinal) => {
                if ordinal <= min {
                    return None;
                }
                ordinal.saturating_sub(1).min(max)
            }
            Bound::Unbounded => max,
        };

        (front <= back).then_some((front, back))
    }

    fn next_ge_in_bits(
        min: Ordinal,
        max: Ordinal,
        bits: &[u64; WORDS],
        ordinal: Ordinal,
    ) -> Option<Ordinal> {
        if ordinal > max {
            return None;
        }

        let start = ordinal.max(min);
        let offset = start - min;
        let mut word_index = (offset / BITS_PER_WORD) as usize;
        let bit_index = offset % BITS_PER_WORD;
        let mut word = bits[word_index] & (!0u64 << bit_index);

        loop {
            if word != 0 {
                let ordinal = min + word_index as u32 * BITS_PER_WORD + word.trailing_zeros();
                return (ordinal <= max).then_some(ordinal);
            }

            word_index += 1;
            if word_index == WORDS {
                return None;
            }
            word = bits[word_index];
        }
    }

    fn prev_le_in_bits(
        min: Ordinal,
        max: Ordinal,
        bits: &[u64; WORDS],
        ordinal: Ordinal,
    ) -> Option<Ordinal> {
        if ordinal < min {
            return None;
        }

        let start = ordinal.min(max);
        let offset = start - min;
        let mut word_index = (offset / BITS_PER_WORD) as usize;
        let bit_index = offset % BITS_PER_WORD;
        let mask = if bit_index == BITS_PER_WORD - 1 {
            !0u64
        } else {
            (1u64 << (bit_index + 1)) - 1
        };
        let mut word = bits[word_index] & mask;

        loop {
            if word != 0 {
                let ordinal = min
                    + word_index as u32 * BITS_PER_WORD
                    + (BITS_PER_WORD - 1 - word.leading_zeros());
                return (ordinal <= max).then_some(ordinal);
            }

            if word_index == 0 {
                return None;
            }
            word_index -= 1;
            word = bits[word_index];
        }
    }

    fn word_and_mask(min: Ordinal, ordinal: Ordinal) -> (usize, u64) {
        let offset = ordinal - min;
        let word = (offset / BITS_PER_WORD) as usize;
        let bit = offset % BITS_PER_WORD;
        (word, 1u64 << bit)
    }

    fn assert_valid_domain(min: Ordinal, max: Ordinal) {
        assert!(
            min <= max,
            "ordinal domain minimum {min} must be less than or equal to maximum {max}"
        );
        assert!(
            Self::domain_len(min, max) <= MAX_ORDINAL_COUNT,
            "ordinal domain {min}-{max} is too large for the fixed bitset"
        );
    }

    fn domain_len(min: Ordinal, max: Ordinal) -> u32 {
        max - min + 1
    }

    fn ordinal_is_in_domain(min: Ordinal, max: Ordinal, ordinal: Ordinal) -> bool {
        (min..=max).contains(&ordinal)
    }
}

impl PartialEq for OrdinalSet {
    fn eq(&self, other: &Self) -> bool {
        if self.domain() != other.domain() {
            return false;
        }

        match (self, other) {
            (Self::All { .. }, Self::All { .. }) => true,
            (
                Self::Explicit { bits, count, .. },
                Self::Explicit {
                    bits: other_bits,
                    count: other_count,
                    ..
                },
            ) => count == other_count && bits == other_bits,
            _ => self.is_all() && other.is_all(),
        }
    }
}

impl<'a> IntoIterator for &'a OrdinalSet {
    type Item = Ordinal;
    type IntoIter = OrdinalSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct OrdinalSetIter<'a> {
    state: OrdinalSetIterState<'a>,
}

enum OrdinalSetIterState<'a> {
    Empty,
    All {
        front: Ordinal,
        back: Ordinal,
    },
    Explicit {
        min: Ordinal,
        max: Ordinal,
        bits: &'a [u64; WORDS],
        front: Ordinal,
        back: Ordinal,
    },
}

impl OrdinalSetIter<'_> {
    fn empty() -> Self {
        Self {
            state: OrdinalSetIterState::Empty,
        }
    }

    fn all(front: Ordinal, back: Ordinal) -> Self {
        Self {
            state: OrdinalSetIterState::All { front, back },
        }
    }

    fn explicit<'a>(
        min: Ordinal,
        max: Ordinal,
        bits: &'a [u64; WORDS],
        front: Ordinal,
        back: Ordinal,
    ) -> OrdinalSetIter<'a> {
        OrdinalSetIter {
            state: OrdinalSetIterState::Explicit {
                min,
                max,
                bits,
                front,
                back,
            },
        }
    }
}

impl Iterator for OrdinalSetIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            OrdinalSetIterState::Empty => None,
            OrdinalSetIterState::All { front, back } => {
                if front > back {
                    self.state = OrdinalSetIterState::Empty;
                    return None;
                }

                let ordinal = *front;
                if ordinal == *back {
                    self.state = OrdinalSetIterState::Empty;
                } else {
                    *front += 1;
                }
                Some(ordinal)
            }
            OrdinalSetIterState::Explicit {
                min,
                max,
                bits,
                front,
                back,
            } => {
                let ordinal = OrdinalSet::next_ge_in_bits(*min, *max, bits, *front)?;
                if ordinal > *back {
                    self.state = OrdinalSetIterState::Empty;
                    return None;
                }

                if ordinal == *back {
                    self.state = OrdinalSetIterState::Empty;
                } else {
                    *front = ordinal + 1;
                }
                Some(ordinal)
            }
        }
    }
}

impl DoubleEndedIterator for OrdinalSetIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        match &mut self.state {
            OrdinalSetIterState::Empty => None,
            OrdinalSetIterState::All { front, back } => {
                if front > back {
                    self.state = OrdinalSetIterState::Empty;
                    return None;
                }

                let ordinal = *back;
                if ordinal == *front {
                    self.state = OrdinalSetIterState::Empty;
                } else {
                    *back -= 1;
                }
                Some(ordinal)
            }
            OrdinalSetIterState::Explicit {
                min,
                max,
                bits,
                front,
                back,
            } => {
                let ordinal = OrdinalSet::prev_le_in_bits(*min, *max, bits, *back)?;
                if ordinal < *front {
                    self.state = OrdinalSetIterState::Empty;
                    return None;
                }

                if ordinal == *front {
                    self.state = OrdinalSetIterState::Empty;
                } else {
                    *back = ordinal - 1;
                }
                Some(ordinal)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Bound::{Excluded, Included, Unbounded};

    #[test]
    fn all_membership_count_and_iteration() {
        let set = OrdinalSet::all(3, 7);

        assert!(set.contains(3));
        assert!(set.contains(5));
        assert!(set.contains(7));
        assert!(!set.contains(2));
        assert!(!set.contains(8));
        assert_eq!(set.count(), 5);
        assert!(set.is_all());
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![3, 4, 5, 6, 7]);
        assert_eq!(set.iter().rev().collect::<Vec<_>>(), vec![7, 6, 5, 4, 3]);
        assert_eq!(
            set.range((Included(4), Excluded(7))).collect::<Vec<_>>(),
            vec![4, 5, 6]
        );
    }

    #[test]
    fn explicit_insert_duplicate_and_canonicalize_to_all() {
        let mut set = OrdinalSet::empty(0, 2);

        assert!(set.insert(0));
        assert!(!set.insert(0));
        assert_eq!(set.count(), 1);
        assert!(!set.is_all());
        assert!(set.insert(2));
        assert!(set.insert(1));

        assert!(set.is_all());
        assert_eq!(set.count(), 3);
        assert_eq!(set.iter().collect::<Vec<_>>(), vec![0, 1, 2]);
    }

    #[test]
    fn union_assign_canonicalizes_full_domain() {
        let mut first = OrdinalSet::from_iter(1, 4, [1, 3]);
        let second = OrdinalSet::from_iter(1, 4, [2, 4]);

        first.union_assign(&second);

        assert!(first.is_all());
        assert_eq!(first.iter().collect::<Vec<_>>(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn explicit_range_bounds_and_empty_ranges() {
        let set = OrdinalSet::from_iter(10, 20, [10, 12, 15, 20]);

        assert_eq!(
            set.range((Included(10), Included(15))).collect::<Vec<_>>(),
            vec![10, 12, 15]
        );
        assert_eq!(
            set.range((Excluded(10), Excluded(20))).collect::<Vec<_>>(),
            vec![12, 15]
        );
        assert_eq!(
            set.range((Unbounded, Included(12))).collect::<Vec<_>>(),
            vec![10, 12]
        );
        assert_eq!(
            set.range((Excluded(15), Unbounded)).collect::<Vec<_>>(),
            vec![20]
        );
        assert_eq!(
            set.range((Included(0), Included(99))).collect::<Vec<_>>(),
            vec![10, 12, 15, 20]
        );
        assert!(set.range((Included(21), Unbounded)).next().is_none());
        assert!(set.range((Unbounded, Excluded(10))).next().is_none());
        assert!(set.range((Included(15), Excluded(15))).next().is_none());
        assert_eq!(
            set.range((Included(12), Included(20)))
                .rev()
                .collect::<Vec<_>>(),
            vec![20, 15, 12]
        );
    }

    #[test]
    fn next_ge_and_prev_le_cover_edges_and_gaps() {
        let set = OrdinalSet::from_iter(5, 15, [5, 8, 12, 15]);

        assert_eq!(set.next_ge(0), Some(5));
        assert_eq!(set.next_ge(5), Some(5));
        assert_eq!(set.next_ge(6), Some(8));
        assert_eq!(set.next_ge(13), Some(15));
        assert_eq!(set.next_ge(15), Some(15));
        assert_eq!(set.next_ge(16), None);

        assert_eq!(set.prev_le(20), Some(15));
        assert_eq!(set.prev_le(15), Some(15));
        assert_eq!(set.prev_le(14), Some(12));
        assert_eq!(set.prev_le(6), Some(5));
        assert_eq!(set.prev_le(5), Some(5));
        assert_eq!(set.prev_le(4), None);
    }
}

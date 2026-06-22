use std::fmt;
use std::ops::{Bound, RangeBounds};

pub type Ordinal = u32;

const WORDS: usize = 4;
const WORD_BITS: Ordinal = u64::BITS;
const MAX_SPAN: Ordinal = WORD_BITS * WORDS as Ordinal;

#[derive(Clone, Eq)]
pub(crate) struct OrdinalSet {
    min: Ordinal,
    max: Ordinal,
    len: usize,
    kind: OrdinalSetKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OrdinalSetKind {
    All,
    Bits([u64; WORDS]),
}

impl OrdinalSet {
    pub(crate) fn empty(min: Ordinal, max: Ordinal) -> Self {
        assert_supported_span(min, max);
        Self {
            min,
            max,
            len: 0,
            kind: OrdinalSetKind::Bits([0; WORDS]),
        }
    }

    pub(crate) fn all(min: Ordinal, max: Ordinal) -> Self {
        assert_supported_span(min, max);
        Self {
            min,
            max,
            len: span_len(min, max),
            kind: OrdinalSetKind::All,
        }
    }

    pub(crate) fn from_values<I>(min: Ordinal, max: Ordinal, values: I) -> Self
    where
        I: IntoIterator<Item = Ordinal>,
    {
        let mut set = Self::empty(min, max);
        for value in values {
            set.insert(value);
        }
        set
    }

    pub(crate) fn try_from_values<I>(min: Ordinal, max: Ordinal, values: I) -> Option<Self>
    where
        I: IntoIterator<Item = Ordinal>,
    {
        supports_span(min, max).then(|| Self::from_values(min, max, values))
    }

    pub(crate) fn insert(&mut self, ordinal: Ordinal) -> bool {
        debug_assert!(
            self.in_bounds(ordinal),
            "ordinal {ordinal} outside supported range {}..={}",
            self.min,
            self.max
        );

        if !self.in_bounds(ordinal) {
            return false;
        }

        let OrdinalSetKind::Bits(words) = &mut self.kind else {
            return false;
        };
        let offset = ordinal - self.min;
        let word = (offset / WORD_BITS) as usize;
        let bit = 1_u64 << (offset % WORD_BITS);
        let was_present = words[word] & bit != 0;
        if !was_present {
            words[word] |= bit;
            self.len += 1;
        }
        !was_present
    }

    pub(crate) fn contains(&self, ordinal: &Ordinal) -> bool {
        if !self.in_bounds(*ordinal) {
            return false;
        }

        match self.kind {
            OrdinalSetKind::All => true,
            OrdinalSetKind::Bits(words) => {
                let offset = ordinal - self.min;
                let word = (offset / WORD_BITS) as usize;
                let bit = 1_u64 << (offset % WORD_BITS);
                words[word] & bit != 0
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub(crate) fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn is_all(&self) -> bool {
        self.len == span_len(self.min, self.max)
    }

    #[cfg(test)]
    pub(crate) fn is_unrestricted(&self) -> bool {
        matches!(self.kind, OrdinalSetKind::All)
    }

    pub(crate) fn iter(&self) -> OrdinalSetIter<'_> {
        self.iter_between(self.min, self.max)
    }

    pub(crate) fn range<R>(&self, range: R) -> OrdinalSetIter<'_>
    where
        R: RangeBounds<Ordinal>,
    {
        let Some((start, end)) = bounds_to_inclusive(range, self.min, self.max) else {
            return OrdinalSetIter::empty(self);
        };
        self.iter_between(start, end)
    }

    pub(crate) fn ordered_range<R>(&self, range: R, reverse: bool) -> OrderedOrdinalSetIter<'_>
    where
        R: RangeBounds<Ordinal>,
    {
        OrderedOrdinalSetIter {
            iter: self.range(range),
            reverse,
        }
    }

    fn iter_between(&self, start: Ordinal, end: Ordinal) -> OrdinalSetIter<'_> {
        if start > end {
            return OrdinalSetIter::empty(self);
        }

        OrdinalSetIter {
            set: self,
            front: start,
            back: end,
            exhausted: false,
        }
    }

    fn in_bounds(&self, ordinal: Ordinal) -> bool {
        ordinal >= self.min && ordinal <= self.max
    }

    fn bit_is_set(&self, ordinal: Ordinal) -> bool {
        bit_is_set(self.kind, self.min, ordinal)
    }
}

impl fmt::Debug for OrdinalSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            OrdinalSetKind::All => f
                .debug_struct("OrdinalSet::All")
                .field("min", &self.min)
                .field("max", &self.max)
                .finish(),
            OrdinalSetKind::Bits(_) => f.debug_set().entries(self.iter()).finish(),
        }
    }
}

impl PartialEq for OrdinalSet {
    fn eq(&self, other: &Self) -> bool {
        self.len == other.len && self.iter().eq(other.iter())
    }
}

impl<'a> IntoIterator for &'a OrdinalSet {
    type Item = Ordinal;
    type IntoIter = OrdinalSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for OrdinalSet {
    type Item = Ordinal;
    type IntoIter = OwnedOrdinalSetIter;

    fn into_iter(self) -> Self::IntoIter {
        OwnedOrdinalSetIter {
            min: self.min,
            kind: self.kind,
            front: self.min,
            back: self.max,
            exhausted: self.is_empty(),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OrdinalSetIter<'a> {
    set: &'a OrdinalSet,
    front: Ordinal,
    back: Ordinal,
    exhausted: bool,
}

impl<'a> OrdinalSetIter<'a> {
    fn empty(set: &'a OrdinalSet) -> Self {
        Self {
            set,
            front: set.min,
            back: set.min,
            exhausted: true,
        }
    }
}

impl Iterator for OrdinalSetIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        while self.front <= self.back {
            let candidate = self.front;
            let last_candidate = self.front == self.back;
            if last_candidate {
                self.exhausted = true;
            } else {
                self.front += 1;
            }

            if self.set.bit_is_set(candidate) {
                return Some(candidate);
            }
            if last_candidate {
                break;
            }
        }

        self.exhausted = true;
        None
    }
}

impl DoubleEndedIterator for OrdinalSetIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        while self.front <= self.back {
            let candidate = self.back;
            let last_candidate = self.front == self.back;
            if last_candidate {
                self.exhausted = true;
            } else {
                self.back -= 1;
            }

            if self.set.bit_is_set(candidate) {
                return Some(candidate);
            }
            if last_candidate {
                break;
            }
        }

        self.exhausted = true;
        None
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OrderedOrdinalSetIter<'a> {
    iter: OrdinalSetIter<'a>,
    reverse: bool,
}

impl Iterator for OrderedOrdinalSetIter<'_> {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        if self.reverse {
            self.iter.next_back()
        } else {
            self.iter.next()
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OwnedOrdinalSetIter {
    min: Ordinal,
    kind: OrdinalSetKind,
    front: Ordinal,
    back: Ordinal,
    exhausted: bool,
}

impl Iterator for OwnedOrdinalSetIter {
    type Item = Ordinal;

    fn next(&mut self) -> Option<Self::Item> {
        if self.exhausted {
            return None;
        }

        while self.front <= self.back {
            let candidate = self.front;
            let last_candidate = self.front == self.back;
            if last_candidate {
                self.exhausted = true;
            } else {
                self.front += 1;
            }

            if bit_is_set(self.kind, self.min, candidate) {
                return Some(candidate);
            }
            if last_candidate {
                break;
            }
        }

        self.exhausted = true;
        None
    }
}

fn bounds_to_inclusive<R>(range: R, min: Ordinal, max: Ordinal) -> Option<(Ordinal, Ordinal)>
where
    R: RangeBounds<Ordinal>,
{
    let start = match range.start_bound() {
        Bound::Included(start) => *start,
        Bound::Excluded(start) => start.checked_add(1)?,
        Bound::Unbounded => min,
    }
    .max(min);

    let end = match range.end_bound() {
        Bound::Included(end) => *end,
        Bound::Excluded(end) => end.checked_sub(1)?,
        Bound::Unbounded => max,
    }
    .min(max);

    (start <= end).then_some((start, end))
}

fn bit_is_set(kind: OrdinalSetKind, min: Ordinal, ordinal: Ordinal) -> bool {
    match kind {
        OrdinalSetKind::All => true,
        OrdinalSetKind::Bits(words) => {
            let offset = ordinal - min;
            let word = (offset / WORD_BITS) as usize;
            let bit = 1_u64 << (offset % WORD_BITS);
            words[word] & bit != 0
        }
    }
}

fn assert_supported_span(min: Ordinal, max: Ordinal) {
    assert!(
        supports_span(min, max),
        "ordinal range {min}..={max} exceeds {MAX_SPAN} supported ordinals"
    );
}

fn supports_span(min: Ordinal, max: Ordinal) -> bool {
    min <= max && max - min < MAX_SPAN
}

fn span_len(min: Ordinal, max: Ordinal) -> usize {
    (max - min + 1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ops::Bound::{Excluded, Included};

    #[test]
    fn all_set_contains_and_iterates_supported_range() {
        let set = OrdinalSet::all(0, 5);

        assert!(set.is_all());
        assert!(set.is_unrestricted());
        assert_eq!(6, set.len());
        assert!(set.contains(&0));
        assert!(set.contains(&5));
        assert!(!set.contains(&6));
        assert_eq!(vec![0, 1, 2, 3, 4, 5], set.iter().collect::<Vec<_>>());
    }

    #[test]
    fn explicit_bits_track_len_membership_and_content_equality() {
        let mut set = OrdinalSet::empty(0, 5);

        assert!(set.is_empty());
        assert!(set.insert(1));
        assert!(!set.insert(1));
        assert!(set.insert(5));

        assert_eq!(2, set.len());
        assert!(set.contains(&1));
        assert!(!set.contains(&2));
        assert_eq!(vec![1, 5], set.iter().collect::<Vec<_>>());

        let explicit_all = OrdinalSet::from_values(0, 5, 0..=5);
        assert_eq!(OrdinalSet::all(0, 5), explicit_all);
        assert!(!explicit_all.is_unrestricted());
    }

    #[test]
    fn ranges_are_clipped_and_double_ended() {
        let set = OrdinalSet::from_values(0, 10, [1, 3, 7, 10]);

        assert_eq!(vec![3, 7], set.range(2..=8).collect::<Vec<_>>());
        assert_eq!(
            vec![1, 3, 7],
            set.range((Included(1), Excluded(10))).collect::<Vec<_>>()
        );
        assert_eq!(vec![7, 10], set.range(7..=99).collect::<Vec<_>>());

        let mut iter = set.range(0..=10);
        assert_eq!(Some(1), iter.next());
        assert_eq!(Some(10), iter.next_back());
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(7), iter.next_back());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn ordered_ranges_can_iterate_in_reverse() {
        let set = OrdinalSet::from_values(0, 10, [1, 3, 7, 10]);

        assert_eq!(
            vec![10, 7, 3],
            set.ordered_range(2..=10, true).collect::<Vec<_>>()
        );
        assert_eq!(
            vec![3, 7, 10],
            set.ordered_range(2..=10, false).collect::<Vec<_>>()
        );
    }

    #[test]
    fn year_span_fits_in_fixed_words() {
        let years = OrdinalSet::from_values(1970, 2099, [1970, 2099]);
        let all_years = OrdinalSet::all(1970, 2099);

        assert_eq!(130, all_years.len());
        assert_eq!(vec![1970, 2099], years.iter().collect::<Vec<_>>());
        assert_eq!(
            vec![2098, 2099],
            all_years.range(2098..=2100).collect::<Vec<_>>()
        );
    }
}

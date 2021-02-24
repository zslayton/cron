use std::collections::BTreeSet;
use std::collections::btree_set;
use std::ops::RangeBounds;

pub type Ordinal = u32;
pub type OrdinalSet = BTreeSet<Ordinal>;

pub struct OrdinalIter<'a> {
    set_iter: btree_set::Iter<'a, Ordinal>,
}

impl <'a> OrdinalIter<'a> {
    pub fn new(set: &'a OrdinalSet) -> Self{
        OrdinalIter { set_iter: set.iter() }
    }
}

impl<'a> Iterator for OrdinalIter<'a> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
        self.set_iter.next().copied()
    }
}

impl<'a> DoubleEndedIterator for OrdinalIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.set_iter.next_back().copied()
    }
}

pub struct OrdinalRangeIter<'a> {
    range_iter: btree_set::Range<'a, Ordinal>,
}

impl <'a> OrdinalRangeIter<'a> {
    pub fn new<R>(set: &'a OrdinalSet, range: R) -> Self
    where
        R: RangeBounds<Ordinal>, {
        OrdinalRangeIter {
            range_iter: set.range(range)
        }
    }
}

impl<'a> Iterator for OrdinalRangeIter<'a> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
        self.range_iter.next().copied()
    }
}

impl<'a> DoubleEndedIterator for OrdinalRangeIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.range_iter.next_back().copied()
    }
}
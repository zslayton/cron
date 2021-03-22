use std::collections::BTreeSet;
use std::collections::btree_set;
// use std::ops::RangeBounds;
use core::iter::FusedIterator;
// use std::iter::Cloned;

pub type Ordinal = u32;
pub type OrdinalSet = BTreeSet<Ordinal>;
// pub type OrdinalIter<'a> = Cloned<btree_set::Iter<'a, Ordinal>>;
// pub type OrdinalRange<'a> = Cloned<btree_set::Range<'a, Ordinal>>;

pub struct OrdinalIter<'a> {
    pub(crate) set_iter: btree_set::Iter<'a, Ordinal>,
}

// impl <'a> OrdinalIter<'a> {
//      pub(crate) fn new(set: &'a OrdinalSet) -> Self{
//         OrdinalIter { set_iter: set.iter() }
//     }
// }

pub struct OrdinalRange<'a> {
    pub(crate) set_range: btree_set::Range<'a, Ordinal>,
}

// impl <'a> OrdinalRange<'a> {
//     pub(crate) fn new<R>(set: &'a OrdinalSet, range: R) -> Self
//     where
//         R: RangeBounds<Ordinal>, {
//         OrdinalRange { set_range: set.range(range) }
//     }
// }

impl<'a> DoubleEndedIterator for OrdinalIter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.set_iter.next_back().copied()
    }
}

impl<'a> DoubleEndedIterator for OrdinalRange<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.set_range.next_back().copied()
    }
}

impl <'a> ExactSizeIterator for OrdinalIter<'a> {
    fn len(&self) -> usize {
        self.set_iter.len()
    }
}

impl <'a> FusedIterator for OrdinalIter<'a> {}

impl <'a> FusedIterator for OrdinalRange<'a> {}

impl<'a> Iterator for OrdinalIter<'a> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
        self.set_iter.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.set_iter.size_hint()
    }
}

impl<'a> Iterator for OrdinalRange<'a> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
        self.set_range.next().copied()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.set_range.size_hint()
    }
}
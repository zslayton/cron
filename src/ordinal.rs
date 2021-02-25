use std::collections::BTreeSet;
use std::collections::btree_set;
use std::iter::Cloned;

pub type Ordinal = u32;
pub type OrdinalSet = BTreeSet<Ordinal>;
pub type OrdinalIter<'a> = Cloned<btree_set::Iter<'a, Ordinal>>;
pub type OrdinalRangeIter<'a> = Cloned<btree_set::Range<'a, Ordinal>>;
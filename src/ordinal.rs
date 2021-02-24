use std::collections::BTreeSet;
use std::collections::btree_set;

pub type Ordinal = u32;
pub type OrdinalSet = BTreeSet<Ordinal>;
pub type OrdinalIter<'a> = btree_set::Iter<'a, Ordinal>;
pub type OrdinalRangeIter<'a> = btree_set::Range<'a, Ordinal>;
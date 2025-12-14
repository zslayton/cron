use std::collections::BTreeSet;

pub type Ordinal = u32;
// TODO: Make OrdinalSet an enum.
// It should either be a BTreeSet of ordinals or an `All` option to save space.
// `All` can iterate from inclusive_min to inclusive_max and answer membership
// queries
pub type OrdinalSet = BTreeSet<Ordinal>;

// Bit flags to store special constraints on ordinals in high bits

pub const IS_WEEKDAY: u32 = 0b1000_0000_0000_0000_0000_0000_0000_0000;

pub const IS_1ST_OCCURRENCE: u32 = 0b0100_0000_0000_0000_0000_0000_0000_0000;
pub const IS_2ND_OCCURRENCE: u32 = 0b0010_0000_0000_0000_0000_0000_0000_0000;
pub const IS_3RD_OCCURRENCE: u32 = 0b0001_0000_0000_0000_0000_0000_0000_0000;
pub const IS_4TH_OCCURRENCE: u32 = 0b0000_1000_0000_0000_0000_0000_0000_0000;
pub const IS_5TH_OCCURRENCE: u32 = 0b0000_0100_0000_0000_0000_0000_0000_0000;
pub const IS_LAST_OCCURRENCE: u32 = 0b0000_0010_0000_0000_0000_0000_0000_0000;

pub const IS_NTH_OCCURRENCE: u32 = 0b0111_1100_0000_0000_0000_0000_0000_0000;

use schedule::{Ordinal, OrdinalSet};
use time_unit::TimeUnitField;
use std::borrow::Cow;

pub struct Minutes(OrdinalSet);

impl TimeUnitField for Minutes {
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        Minutes(ordinal_set)
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Minutes")
    }
    fn inclusive_min() -> Ordinal {
        0
    }
    fn inclusive_max() -> Ordinal {
        59
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.0
    }
}

use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct Hours(OrdinalSet);

impl TimeUnitField for Hours {
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        Hours(ordinal_set)
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Hours")
    }
    fn inclusive_min() -> Ordinal {
        0
    }
    fn inclusive_max() -> Ordinal {
        23
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.0
    }
}

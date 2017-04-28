use schedule::{Ordinal, OrdinalSet};
use time_unit::TimeUnitField;
use std::borrow::Cow;

#[derive(PartialEq, Eq)]
pub struct DaysOfMonth(OrdinalSet);

impl TimeUnitField for DaysOfMonth {
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        DaysOfMonth(ordinal_set)
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Days of Month")
    }
    fn inclusive_min() -> Ordinal {
        1
    }
    fn inclusive_max() -> Ordinal {
        31
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.0
    }
}

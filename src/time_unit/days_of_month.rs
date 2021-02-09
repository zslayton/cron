use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;
use lazy_static::lazy_static;

lazy_static!{
    static ref ALL: OrdinalSet = DaysOfMonth::all().ordinals.unwrap();
}

#[derive(Clone, Debug)]
pub struct DaysOfMonth{
    ordinals: Option<OrdinalSet>
}

impl TimeUnitField for DaysOfMonth {
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self {
        DaysOfMonth {
            ordinals: Some(ordinal_set)
        }
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
        match &self.ordinals {
            Some(ordinal_set) => &ordinal_set,
            None => &ALL
        }
    }
    fn is_specified(&self) -> bool {
        self.ordinals.is_some()
    }
}

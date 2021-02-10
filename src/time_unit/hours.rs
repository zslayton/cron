use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;
use lazy_static::lazy_static;

lazy_static!{
    static ref ALL: OrdinalSet = Hours::supported_ordinals();
}

#[derive(Clone, Debug)]
pub struct Hours{
    ordinals: Option<OrdinalSet>
}

impl TimeUnitField for Hours {
    fn from_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        Hours{
            ordinals: ordinal_set
        }
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
        match &self.ordinals {
            Some(ordinal_set) => &ordinal_set,
            None => &ALL
        }
    }
    fn is_specified(&self) -> bool {
        self.ordinals.is_some()
    }
}

use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;

#[derive(Clone, Debug, Eq)]
pub struct Seconds {
    ordinals: OrdinalSet,
}

impl TimeUnitField for Seconds {
    fn from_optional_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        Seconds {
            ordinals: ordinal_set.unwrap_or_else(Self::supported_ordinals),
        }
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Seconds")
    }
    fn inclusive_min() -> Ordinal {
        0
    }
    fn inclusive_max() -> Ordinal {
        59
    }
    fn ordinals(&self) -> &OrdinalSet {
        &self.ordinals
    }
}

impl PartialEq for Seconds {
    fn eq(&self, other: &Seconds) -> bool {
        self.ordinals() == other.ordinals()
    }
}

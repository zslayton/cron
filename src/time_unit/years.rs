use crate::ordinal::{Ordinal, OrdinalSet};
use crate::time_unit::TimeUnitField;
use std::borrow::Cow;
use lazy_static::lazy_static;

lazy_static!{
    static ref ALL: OrdinalSet = Years::supported_ordinals();
}

#[derive(Clone, Debug)]
pub struct Years{
    ordinals: Option<OrdinalSet>
}

impl TimeUnitField for Years {
    fn from_ordinal_set(ordinal_set: Option<OrdinalSet>) -> Self {
        Years{
            ordinals: ordinal_set
        }
    }
    fn name() -> Cow<'static, str> {
        Cow::from("Years")
    }

    // TODO: Using the default impl, this will make a set w/100+ items each time "*" is used.
    // This is obviously suboptimal.
    fn inclusive_min() -> Ordinal {
        1970
    }
    fn inclusive_max() -> Ordinal {
        2100
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

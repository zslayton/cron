use crate::ordinal::*;
use std::fmt::{Display, Formatter, Result as FmtResult};

#[derive(Debug, PartialEq)]
pub enum RangeEndpoint {
    Ordinal(Ordinal),
    Name(String),
}

impl Display for RangeEndpoint {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        match self {
            Self::Ordinal(ordinal) => write!(f, "{ordinal}"),
            Self::Name(name) => write!(f, "{name}"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum Specifier {
    All,
    Point(Ordinal),
    Range(RangeEndpoint, RangeEndpoint),
}

// Separating out a root specifier allows for a higher tiered specifier, allowing us to achieve
// periods with base values that are more advanced than an ordinal:
// - all: '*/2'
// - range: '10-2/2'
// - named range: 'Mon-Thurs/2'
//
// Without this separation we would end up with invalid combinations such as 'Mon/2'
#[derive(Debug, PartialEq)]
pub enum RootSpecifier {
    Specifier(Specifier),
    Period(Specifier, u32),
    NamedPoint(String),
}

impl From<Specifier> for RootSpecifier {
    fn from(specifier: Specifier) -> Self {
        Self::Specifier(specifier)
    }
}

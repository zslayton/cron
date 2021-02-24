use crate::error::Error;
use crate::ordinal::OrdinalSet;
use crate::specifier::{RootSpecifier, Specifier};
use crate::time_unit::TimeUnitField;

#[derive(Debug, PartialEq)]
pub struct Field {
    pub specifiers: Vec<RootSpecifier>, // TODO: expose iterator?
}

pub trait FromField
where
    Self: Sized,
{
    //TODO: Replace with std::convert::TryFrom when stable
    fn from_field(field: Field) -> Result<Self, Error>;
}

impl<T> FromField for T
where
    T: TimeUnitField,
{
    fn from_field(field: Field) -> Result<T, Error> {
        if field.specifiers.len() == 1 && 
            field.specifiers.get(0).unwrap() == &RootSpecifier::from(Specifier::All) 
            { return Ok(T::all()); }
        let mut ordinals = OrdinalSet::new(); 
        for specifier in field.specifiers {
            let specifier_ordinals: OrdinalSet = T::ordinals_from_root_specifier(&specifier)?;
            for ordinal in specifier_ordinals {
                ordinals.insert(T::validate_ordinal(ordinal)?);
            }
        }
        Ok(T::from_ordinal_set(ordinals))
    }
}
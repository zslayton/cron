mod seconds;
mod minutes;
mod hours;
mod days_of_month;
mod months;
mod days_of_week;
mod years;

pub use self::seconds::Seconds;
pub use self::minutes::Minutes;
pub use self::hours::Hours;
pub use self::days_of_month::DaysOfMonth;
pub use self::months::Months;
pub use self::days_of_week::DaysOfWeek;
pub use self::years::Years;

use std::collections::btree_set;
use std::collections::range::{RangeArgument};
use schedule::{Specifier, Ordinal, OrdinalSet};
use error::*;
use std::borrow::Cow;
use std::iter;

pub struct OrdinalIter<'a> {
    set_iter: btree_set::Iter<'a, Ordinal>
}

impl <'a> Iterator for OrdinalIter<'a> {
    type Item = Ordinal;
    fn next(&mut self) -> Option<Ordinal> {
      self.set_iter.next().map(|ordinal| ordinal.clone()) // No real expense; Ordinal is u32: Copy
    }
}

pub struct OrdinalRangeIter<'a> {
  range_iter: btree_set::Range<'a, Ordinal>
}

impl <'a> Iterator for OrdinalRangeIter<'a> {
  type Item = Ordinal;
  fn next(&mut self) -> Option<Ordinal> {
    self.range_iter.next().map(|ordinal| ordinal.clone()) // No real expense; Ordinal is u32: Copy
  }
}

pub trait TimeUnitSpec {
  fn includes(&self, ordinal: Ordinal) -> bool;
  fn iter<'a>(&'a self) -> OrdinalIter<'a>;
  fn range<'a, R>(&'a self, range: R) -> OrdinalRangeIter<'a> where R: RangeArgument<Ordinal>;
  fn count(&self) -> u32;
}

impl <T> TimeUnitSpec for T where T: TimeUnitField {
  fn includes(&self, ordinal: Ordinal) -> bool {
    self.ordinals().contains(&ordinal)
  }
  fn iter<'a>(&'a self) -> OrdinalIter<'a> {
    OrdinalIter {
      set_iter: TimeUnitField::ordinals(self).iter()
    }
  }
  fn range<'a, R>(&'a self, range: R) -> OrdinalRangeIter<'a> where R: RangeArgument<Ordinal> {
    OrdinalRangeIter {
      range_iter: TimeUnitField::ordinals(self).range(range)
    }
  }
  fn count(&self) -> u32 {
    self.ordinals().len() as u32
  }
}

pub trait TimeUnitField
    where Self: Sized
{
    fn from_ordinal_set(ordinal_set: OrdinalSet) -> Self;
    fn name() -> Cow<'static, str>;
    fn inclusive_min() -> Ordinal;
    fn inclusive_max() -> Ordinal;
    fn ordinals(&self) -> &OrdinalSet;
    fn from_ordinal(ordinal: Ordinal) -> Self {
        Self::from_ordinal_set(iter::once(ordinal).collect())
    }
    fn supported_ordinals() -> OrdinalSet {
        (Self::inclusive_min()..Self::inclusive_max() + 1).collect()
    }
    fn all() -> Self {
        Self::from_ordinal_set(Self::supported_ordinals())
    }
    fn ordinal_from_name(name: &str) -> Result<Ordinal> {
        bail!(ErrorKind::Expression(format!("The '{}' field does not support using names. '{}' \
                                     specified.",
                                    Self::name(),
                                    name)))
    }
    fn validate_ordinal(ordinal: Ordinal) -> Result<Ordinal> {
        //println!("validate_ordinal for {} => {}", Self::name(), ordinal);
        match ordinal {
            i if i < Self::inclusive_min() => {
                bail!(ErrorKind::Expression(format!("{} must be greater than or equal to {}. ('{}' \
                                             specified.)",
                                            Self::name(),
                                            Self::inclusive_min(),
                                            i)))
            }
            i if i > Self::inclusive_max() => {
                bail!(ErrorKind::Expression(format!("{} must be less than {}. ('{}' specified.)",
                                            Self::name(),
                                            Self::inclusive_max(),
                                            i)))
            }
            i => Ok(i),
        }
    }

    fn ordinals_from_specifier(specifier: &Specifier) -> Result<OrdinalSet> {
        use self::Specifier::*;
        //println!("ordinals_from_specifier for {} => {:?}", Self::name(), specifier);
        match *specifier {
            All => Ok(Self::supported_ordinals()),
            Point(ordinal) => Ok((&[ordinal]).iter().cloned().collect()),
            NamedPoint(ref name) => {
                Ok((&[Self::ordinal_from_name(name)?]).iter().cloned().collect())
            }
            Period(start, step) => {
                let start = Self::validate_ordinal(start)?;
                Ok((start..Self::inclusive_max() + 1).step_by(step).collect())
            }
            Range(start, end) => {
                match (Self::validate_ordinal(start), Self::validate_ordinal(end)) {
                    (Ok(start), Ok(end)) if start <= end => Ok((start..end + 1).collect()),
                    _ => {
                        bail!(ErrorKind::Expression(format!("Invalid range for {}: {}-{}",
                                                    Self::name(),
                                                    start,
                                                    end)))
                    }
                }
            }
            NamedRange(ref start_name, ref end_name) => {
                let start = Self::ordinal_from_name(start_name)?;
                let end = Self::ordinal_from_name(end_name)?;
                match (Self::validate_ordinal(start), Self::validate_ordinal(end)) {
                    (Ok(start), Ok(end)) if start <= end => Ok((start..end + 1).collect()),
                    _ => {
                        bail!(ErrorKind::Expression(format!("Invalid named range for {}: {}-{}",
                                                    Self::name(),
                                                    start_name,
                                                    end_name)))
                    }
                }
            }
        }
    }
}

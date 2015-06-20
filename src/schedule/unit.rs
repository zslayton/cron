//use std::collections::BTreeSet;
use std::collections::Bound::{Included, Unbounded};
use std::u32;

pub struct UnitSchedule {
  values: Vec<u32>,
}

pub struct UnitScheduleIterator<'a> {
  schedule: &'a UnitSchedule,
  next_index: u32, 
  min: u32,
  max: u32
}

impl <'a, 'b> Iterator for &'b mut UnitScheduleIterator <'a> {
    type Item = u32;
    /*
  */

  fn next(&mut self) -> Option<u32> {
    while self.next_index < self.schedule.values.len() as u32 {
      let current = self.schedule.values[self.next_index as usize];
      self.next_index = self.next_index + 1;
      if current >= self.min && current <= self.max {
        return Some(current);
      }
    }
    None
  }

}

impl <'a> UnitScheduleIterator <'a> {
  pub fn current(&self) -> Option<u32> {
    match self.next_index {
      0 => None,
      index => self.schedule.values.get((self.next_index-1) as usize).cloned()
    }
  }
  pub fn reset(&'a mut self) -> &'a mut UnitScheduleIterator {
    self.min = u32::MIN;
    self.max = u32::MAX;
    self.next_index = 0;
    self
  }
}

impl UnitSchedule {
  pub fn first(&self) -> Option<u32> {
    self.values.get(0).cloned()
  }

/*  pub fn nearest(&self, unit: u32) -> Option<u32> {
    match self.values.iter().skip_while(|&num| *num < u32).next() {
      Some(number) => Some(number),
      None => self.values.get(0)
    }
  }*/

  // TODO: Use a Result to propagate error explanations
  pub fn from(unit: u32) -> UnitSchedule { 
    let mut values: Vec<u32> = Vec::new(); 
    values.push(unit);
    UnitSchedule {
      values: values,
    }
  }
  
  pub fn from_values(mut units: Vec<u32>) -> UnitSchedule {
    units.sort();
    units.dedup();
    UnitSchedule {
      values: units,
    }
  }

  pub fn range_iter(&self, min: u32, max: u32) -> UnitScheduleIterator {
    UnitScheduleIterator {
      schedule: self,
      next_index: 0,
      min: min,
      max: max
    }
  }

  pub fn matches(&self, unit: u32) -> bool {
    return self.values.contains(&unit);
  }
}

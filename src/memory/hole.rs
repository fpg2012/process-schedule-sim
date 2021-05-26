use std::cmp::Ordering;

#[derive(Copy, Clone, Debug)]
pub struct Hole {
    beg: u32,
    end: u32,
}

impl Hole {
    pub fn new(beg: u32, end: u32) -> Result<Hole, &'static str> {
        if beg >= end {
            Err("invalid range")
        } else {
            Ok(Hole {
                beg,
                end,
            })
        }
    }
    pub fn from_range(range: (u32, u32)) -> Result<Hole, &'static str> {
        let (beg, end) = range;
        Self::new(beg, end)
    }
    pub fn get_size(&self) -> u32 {
        self.end - self.beg
    }
    pub fn test_adjacency(&self, other: &Self) -> bool {
        self.beg == other.end || self.end == other.beg
    }
    pub fn merge_into_self(&mut self, other: &Self) -> Result<(), &'static str> {
        if !self.test_adjacency(other) {
            Err("Cannot Merge")
        } else {
            if self.beg == other.end {
                self.beg = other.beg;
            } else {
                self.end = other.end;
            }
            Ok(())
        }
    }
    pub fn shrink_head(&mut self, size: u32) -> Result<(), &'static str> {
        if size >= self.get_size() {
            Err("hole is not large enough")
        } else {
            self.beg += size;
            Ok(())
        }
    }
    pub fn split_head(&mut self, size: u32) -> Result<Self, &'static str> {
        if size >= self.get_size() {
            Err("hole is not large enough")
        } else {
            let hole = Self::new(self.beg, self.beg + size);
            self.beg += size;
            hole
        }
    }
    pub fn to_tuple(&self) -> (u32, u32) {
        (self.beg, self.end)
    }
}

impl PartialOrd for Hole {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.end <= other.beg {
            Some(Ordering::Less)
        } else if self.beg >= other.end {
            Some(Ordering::Greater)
        } else if self.eq(other) {
            Some(Ordering::Equal)
        } else {
            None
        }
    }
}

impl PartialEq for Hole {
    fn eq(&self, other: &Self) -> bool {
        (self.beg, self.end) == (other.beg, other.end)
    }
}
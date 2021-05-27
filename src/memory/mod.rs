pub mod hole;

use std::collections::HashMap;
use crate::memory::hole::*;

#[derive(Debug)]
pub enum MemoryError {
    OutOfMemory,
    PIDInvalid,
}

pub struct MemoryManager {
    size: u64,
    page_size: u32,
    pid_to_mem: HashMap<u32, Hole>,
    holes: Vec<Hole>,
}

impl MemoryManager {
    pub fn new(size: u64, page_size: u32) -> Self {
        MemoryManager {
            size,
            page_size,
            pid_to_mem: HashMap::new(),
            holes: vec![Hole::new(0, (size / page_size as u64) as u32).unwrap()],
        }
    }
    pub fn allocate(&mut self, req_size: u32, pid: u32) -> Result<Hole, MemoryError> {
        // first fit
        let mut fit: Option<usize> = None;
        let mut entire_hole = false;
        for i in 0..self.holes.len() {
            if self.holes[i].get_size() >= req_size {
                fit = Some(i);
                entire_hole = self.holes[i].get_size() == req_size;
                break;
            }
        }
        match fit {
            None => Err(MemoryError::OutOfMemory),
            Some(i) => {
                let hole = if entire_hole {
                    let temp = self.holes[i];
                    self.holes.remove(i);
                    temp
                } else {
                    self.holes[i].split_head(req_size).unwrap()
                };
                self.pid_to_mem.insert(pid, hole);
                Ok(hole)
            }
        }
    }
    pub fn free(&mut self, pid: u32) -> Result<(), MemoryError> {
        if !self.pid_to_mem.contains_key(&pid) {
            Err(MemoryError::PIDInvalid)
        } else {
            let hole = self.pid_to_mem[&pid];
            self.pid_to_mem.remove(&pid);
            let mut pos: usize = 0;
            for i in 0..self.holes.len() {
                if self.holes[i] > hole {
                    pos = i;
                    break;
                }
            }
            if self.holes[pos].test_adjacency(&hole) {
                self.holes[pos].merge_into_self(&hole)
                    .expect("hole merge failed");
            } else {
                self.holes.insert(pos, hole);
            }
            if pos > 0 {
                if self.holes[pos - 1].test_adjacency(&self.holes[pos]) {
                    let temp = self.holes[pos];
                    self.holes[pos - 1].merge_into_self(&temp).unwrap();
                    self.holes.remove(pos);
                }
            }
            Ok(())
        }
    }
    pub fn size(&self) -> u64 {
        self.size
    }
    pub fn page_size(&self) -> u32 {
        self.page_size
    }
    pub fn print(&self) {
        println!("mem: ");
        for h in &self.holes {
            println!("{:?}", h);
        }
    }
    pub fn get_mem_usage(&self) -> &HashMap<u32, Hole> {
        &self.pid_to_mem
    }
}
use std::cmp::{Ordering, Reverse};
use crate::memory::hole::*;
use crate::condition::*;
use std::fmt::Formatter;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ProcessState {
    New,
    Ready,
    Running,
    Terminated,
    Blocked,
}

#[derive(Debug)]
pub struct Task {
    pid: u32,
    request_time: i32,
    sch_time: i32,
    priority: u32,
    state: ProcessState,
    in_queue_time: i32,
    memory_size: u32,
    memory_range: Option<Hole>,
    cond: Option<ConditionRef>,
    is_suspended: bool
}

impl Task {
    pub fn new(pid: u32, request_time: i32, priority: u32, memory_size: u32) -> Self {
        Task {
            pid,
            request_time,
            priority,
            state: ProcessState::New,
            in_queue_time: 0,
            sch_time: 0x3f3f3f3f,
            memory_size,
            memory_range: None,
            cond: None,
            is_suspended: false,
        }
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn request_time(&self) -> i32 {
        self.request_time
    }
    pub fn priority(&self) -> u32 {
        self.priority
    }
    pub fn state(&self) -> &ProcessState {
        &self.state
    }
    pub fn set_request_time(&mut self, request_time: i32) {
        self.request_time = request_time;
    }
    pub fn set_priority(&mut self, priority: u32) {
        self.priority = priority;
    }
    pub fn set_state(&mut self, state: ProcessState) {
        self.state = state;
    }
    pub fn in_queue_time(&self) -> i32 {
        self.in_queue_time
    }
    pub fn set_in_queue_time(&mut self, in_queue_time: i32) {
        self.in_queue_time = in_queue_time;
    }
    pub fn decrement_time(&mut self, t: i32) {
        self.request_time -= t;
        self.sch_time -= t;
    }
    pub fn memory_range(&self) -> Option<Hole> {
        self.memory_range
    }
    pub fn set_memory_range(&mut self, h: Hole) {
        self.memory_range = Some(h);
    }
    pub fn memory_size(&self) -> u32 {
        self.memory_size
    }
    pub fn sch_time(&self) -> i32 {
        self.sch_time
    }
    pub fn set_sch_time(&mut self, sch_time: i32) {
        self.sch_time = sch_time;
    }
    pub fn cond(&self) -> Option<ConditionRef> {
        self.cond.clone()
    }
    pub fn set_cond(&mut self, cond: Option<ConditionRef>) {
        self.cond = cond;
    }
    pub fn is_cond_satisfied(&self) -> bool {
        match &self.cond {
            None => true,
            Some(cond) => {
                cond.borrow().is_done()
            }
        }
    }
    pub fn is_suspended(&self) -> bool {
        self.is_suspended
    }
    pub fn suspend(&mut self) {
        self.is_suspended = true;
    }
    pub fn unsuspend(&mut self) {
        self.is_suspended = false;
    }
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        (self.priority, self.in_queue_time, self.state).eq(&(other.priority, other.in_queue_time, other.state))
    }
}

impl PartialOrd for Task {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        (self.priority, Reverse(self.in_queue_time)).partial_cmp(&(other.priority, Reverse(other.in_queue_time)))
    }
}

impl Ord for Task {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.priority, Reverse(self.in_queue_time)).cmp(&(other.priority, Reverse(other.in_queue_time)))
    }
}

impl Eq for Task {}
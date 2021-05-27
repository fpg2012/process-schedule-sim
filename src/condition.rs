use std::rc::Rc;
use std::cell::RefCell;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum CondState {
    NotDone,
    Done,
}

#[derive(Debug)]
pub struct Condition {
    cond: CondState
}

impl Condition {
    pub fn new() -> Self {
        Self {
            cond: CondState::NotDone,
        }
    }
    pub fn set_ok(&mut self) {
        self.cond = CondState::Done;
    }
    pub fn is_done(&self) -> bool {
        self.cond == CondState::Done
    }
}

pub type ConditionRef = Rc<RefCell<Condition>>;
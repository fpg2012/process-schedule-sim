use std::rc::Rc;
use std::cell::RefCell;
use crate::task::*;

pub type TaskRef = Rc<RefCell<Task>>;

pub struct Processor {
    proc_id: u32,
    executing_task: Option<TaskRef>,
}

impl Processor {
    pub fn new(proc_id: u32) -> Self {
        Self {
            proc_id,
            executing_task: None,
        }
    }
    pub fn run_task(&mut self) {
        if let Some(task) = &mut self.executing_task {
            let mut task = task.borrow_mut();
            println!("Proc {}: Running {}, req_time {}, sch_time {}", self.proc_id, task.pid(), task.request_time(), task.sch_time());
            task.decrement_time(1);
        } else {
            println!("Proc {}: idle", self.proc_id);
        }
    }
    pub fn is_task_finished(&self) -> bool {
        // return true if task finished or run out of time slice
        match &self.executing_task {
            None => true,
            Some(task) => task.borrow().request_time() <= 0 || task.borrow().sch_time() <= 0
        }
    }
    pub fn turn_to_task(&mut self, task: Option<TaskRef>) -> Option<TaskRef> {
        // preempt if `task` is `Some`
        // do nothing but return `None` if `task` is None and `self.executing_task` is not `None`
        let mut cur_task = self.executing_task.clone();
        if let Some(task) = task {
            self.executing_task = Some(task);
        } else {
            if let Some(task) = &mut self.executing_task {
                // continue to run
                if task.borrow_mut().request_time() > 0 {
                    cur_task = None;
                } else {
                    self.executing_task = None;
                }
            }
        }
        cur_task
    }
    pub fn proc_id(&self) -> u32 {
        self.proc_id
    }
    pub fn executing_task(&self) -> Option<TaskRef> {
        self.executing_task.clone()
    }
}
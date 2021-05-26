use std::collections::{BinaryHeap, HashMap, HashSet};
use crate::task::*;
use crate::task::ProcessState::*;
use crate::memory::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::processor::*;
use crate::scheduler::SchedulerError::{InvalidCondition, InvalidPid};
use crate::condition::{Condition, ConditionRef};
use std::cmp::min;
use std::borrow::Borrow;
use min_max_heap::MinMaxHeap;

#[derive(Debug)]
pub enum SchedulerError {
    UnknownError,
    InvalidCondition,
    InvalidPid,
}

pub struct Scheduler {
    task_queue: BinaryHeap<TaskRef>,
    suspend_queue: BinaryHeap<TaskRef>,
    time: i32,
    time_slice: u32,
    slots: u32,
    processors: [Processor; 2],
    memory_manager: MemoryManager,
    pid_to_trigger: HashMap<u32, ConditionRef>,
    valid_pid: HashSet<u32>,
}

impl Scheduler {
    pub fn new(time_slice: u32, slots: u32) -> Self {
        Scheduler {
            task_queue: BinaryHeap::new(),
            suspend_queue: BinaryHeap::new(),
            time: 0,
            time_slice,
            slots,
            processors: [Processor::new(0), Processor::new(1)],
            memory_manager: MemoryManager::new(16777216, 4096),
            pid_to_trigger: HashMap::new(),
            valid_pid: HashSet::new(),
        }
    }
    pub fn add_task(&mut self, mut task: Task, cond: Option<u32>) -> Result<(), SchedulerError> {
        // validate pid
        if self.valid_pid.contains(&task.pid()) {
            return Err(InvalidPid);
        }
        // check cond
        if let Some(pid) = cond {
            if !self.valid_pid.contains(&pid) {
                return Err(InvalidCondition);
            } else {
                if let Some(tri) = self.pid_to_trigger.get(&pid) {
                    let new_tri = tri.clone();
                    task.set_cond(Some(new_tri));
                }
            }
        }
        let my_tri = Rc::new(RefCell::new(Condition::new()));
        self.pid_to_trigger.insert(task.pid(), my_tri);
        self.valid_pid.insert(task.pid());
        // check cond
        if !task.is_cond_satisfied() {
            println!("Condition not satisfied");
            self.suspend_task(Rc::new(RefCell::new(task)));
            return Ok(());
        }

        // check slots
        if !self.has_available_slots() {
            println!("No enough slots, suspend task {}", task.pid());
            self.suspend_task(Rc::new(RefCell::new(task)));
            return Ok(())
        }

        // allocate mem
        match self.memory_manager.allocate(task.memory_size(), task.pid()) {
            Ok(hole) => {
                task.set_memory_range(hole);
                self.ready_task(Rc::new(RefCell::new(task)));
            }
            Err(_) => {
                println!("No enough memory, suspend task {}", task.pid());
                self.suspend_task(Rc::new(RefCell::new(task)));
            }
        }
        Ok(())
    }
    pub fn has_available_slots(&self) -> bool {
        self.slots > self.running_tasks()
    }
    pub fn running_tasks(&self) -> u32 {
        let mut cnt: u32 = 0;
        for proc in self.processors.iter() {
            if proc.executing_task().is_some() {
                cnt += 1;
            }
        }
        cnt += self.task_queue.len() as u32;
        cnt
    }
    pub fn suspend_task(&mut self, task: TaskRef) {
        task.borrow_mut().set_state(Suspended);
        task.borrow_mut().set_in_queue_time(self.time);
        println!("Task Suspended: {}", task.borrow().pid());
        self.memory_manager.free(task.borrow().pid());
        self.suspend_queue.push(task);
    }
    pub fn unsuspend_task(&mut self, task: TaskRef) -> Result<(), TaskRef> {
        let (size, pid) = (task.borrow().memory_size(), task.borrow().pid());
        if !task.borrow().is_cond_satisfied() {
            println!("failed tp unsuspend task {}, condition not satisfied", &pid);
            return Err(task);
        }
        match self.memory_manager.allocate(size, pid) {
            Ok(h) => {
                task.borrow_mut().set_memory_range(h);
                println!("Task Unsuspended: {} with memory {:?}", task.borrow().pid(), task.borrow().memory_range());
                self.ready_task(task);
                Ok(())
            }
            Err(_) => {
                println!("failed to unsuspend task {}, out of memory", pid);
                Err(task)
            }
        }
    }
    pub fn ready_task(&mut self, task: TaskRef) {
        task.borrow_mut().set_state(Ready);
        task.borrow_mut().set_in_queue_time(self.time);
        println!("Task Ready: {}", task.borrow().pid());
        self.task_queue.push(task);
    }
    pub fn terminate_task(&mut self, task: TaskRef) {
        println!("--> {} finished. ", task.borrow().pid());
        task.borrow_mut().set_state(Terminated);
        self.memory_manager.free(task.borrow().pid());
        let cond = self.pid_to_trigger.get(&task.borrow().pid()).unwrap().clone();
        cond.borrow_mut().set_ok();
    }
    pub fn advance_time(&mut self) {
        self.time += 1;
        self.schedule();
        // run task
        for proc in self.processors.iter_mut() {
            proc.run_task();
        }
        self.memory_manager.print();
    }
    pub fn schedule_proc(&mut self, proc: usize) {
        let proc = &mut self.processors[proc];
        let new_task = self.task_queue.pop();
        if let Some(temp) = &new_task {
            temp.borrow_mut().set_sch_time(self.time_slice as i32);
            temp.borrow_mut().set_state(Running);
        }
        let old_task = proc.turn_to_task(new_task);
        if let Some(task) = old_task {
            if task.borrow_mut().request_time() <= 0 {
                self.terminate_task(task);
            } else {
                self.ready_task(task);
            }
        }
    }
    pub fn schedule(&mut self) {
        println!("time {}: ", self.time);
        // ready <=> running
        for proc in 0..self.processors.len() {
            if self.processors[proc].is_task_finished() {
                self.schedule_proc(proc);
            }
        }
        // suspended <=> ready
        // check priority

        for t in self.task_queue {
            min_pri = min(t.borrow().priority(), min_pri);
        }
        // unsuspend
        let mut temp_vec: Vec<TaskRef> = vec![];
        while self.has_available_slots() && !self.suspend_queue.is_empty() {
            let task = self.suspend_queue.pop();
            if let Some(task) = task {
                match self.unsuspend_task(task) {
                    Err(task) => {
                        temp_vec.push(task);
                    }
                    _ => (),
                }
            }
        }
        for t in temp_vec {
            self.suspend_task(t);
        }
    }
    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }
    pub fn executing_task(&self) -> [Option<u32>; 2] {
        let mut temp = [None, None];
        for i in 0..self.processors.len() {
            let proc = &self.processors[i];
            let t = proc.executing_task().clone();
            let t = if let Some(t) = t {
                Some(t.borrow().pid())
            } else {
                None
            };
            temp[i] = t;
        }
        temp
    }
    pub fn time(&self) -> i32 {
        self.time
    }
}
use std::collections::{BinaryHeap, HashMap, HashSet};
use crate::task::*;
use crate::task::ProcessState::*;
use crate::memory::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::processor::*;
use crate::scheduler::SchedulerError::{InvalidCondition, InvalidPid};
use crate::condition::{Condition, ConditionRef};
use min_max_heap::MinMaxHeap;

#[derive(Debug)]
pub enum SchedulerError {
    UnknownError,
    InvalidCondition,
    InvalidPid,
}

pub struct Scheduler {
    new_queue: BinaryHeap<TaskRef>,
    task_queue: MinMaxHeap<TaskRef>,
    blocked_queue: MinMaxHeap<TaskRef>,
    blocked_suspend_queue: BinaryHeap<TaskRef>,
    ready_suspend_queue: BinaryHeap<TaskRef>,
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
            new_queue: BinaryHeap::new(),
            task_queue: MinMaxHeap::new(),
            blocked_queue: MinMaxHeap::new(),
            ready_suspend_queue: BinaryHeap::new(),
            blocked_suspend_queue: BinaryHeap::new(),
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

        task.set_state(New);
        println!("New task: {:?}", task);
        self.new_queue.push(Rc::new(RefCell::new(task)));
        Ok(())
    }
    pub fn has_available_slots(&self) -> bool {
        self.slots > self.running_tasks_count()
    }
    pub fn running_tasks_count(&self) -> u32 {
        let mut cnt: u32 = 0;
        for proc in self.processors.iter() {
            if proc.executing_task().is_some() {
                cnt += 1;
            }
        }
        cnt += self.task_queue.len() as u32;
        cnt
    }
    pub fn block_task(&mut self, task: TaskRef) {
        assert_ne!(*task.borrow().state(), Blocked);
        task.borrow_mut().set_state(Blocked);
        task.borrow_mut().set_in_queue_time(self.time);
        println!("Task {} blocked", task.borrow().pid());
        if task.borrow().is_suspended() {
            self.blocked_suspend_queue.push(task);
        } else {
            self.blocked_queue.push(task);
        }
    }
    pub fn new_to_ready_task(&mut self, task: TaskRef) {
        assert_eq!(*task.borrow().state(), New);
        task.borrow_mut().set_state(Ready);
        println!("Task {} New -> Ready", task.borrow().pid());
        self.task_queue.push(task);
    }
    // pub fn new_to_blocked_task(&mut self, task: TaskRef) {
    //     assert_eq!(*task.borrow().state(), New);
    //     task.borrow_mut().set_state(Blocked);
    //     println!("Task {} New -> Blocked", task.borrow().pid());
    //     self.blocked_queue.push(task);
    // }
    pub fn ready_task(&mut self, task: TaskRef) {
        assert_ne!(*task.borrow().state(), Ready);
        task.borrow_mut().set_state(Ready);
        task.borrow_mut().set_in_queue_time(self.time);
        println!("Task Ready: {}", task.borrow().pid());
        if task.borrow().is_suspended() {
            self.ready_suspend_queue.push(task);
        } else {
            self.task_queue.push(task);
        }
    }
    pub fn suspend_task(&mut self, task: TaskRef) {
        if task.borrow().is_suspended() {
            panic!("A task cannot suspend more than once!");
        }
        task.borrow_mut().suspend();
        task.borrow_mut().set_in_queue_time(self.time);
        match task.borrow().state() {
            Ready => self.ready_suspend_queue.push(task.clone()),
            Blocked => self.blocked_suspend_queue.push(task.clone()),
            Terminated | New | Running => panic!("Error, cannot suspend {:?} task", task.borrow().state()),
        }
        self.memory_manager.free(task.borrow().pid());
        println!("Task Suspended: {}", task.borrow().pid());
    }
    pub fn unsuspend_task(&mut self, task: TaskRef) -> Result<(), TaskRef> {
        if !task.borrow().is_suspended() {
            panic!("A task cannot unsuspend more than once!");
        }
        let (size, pid) = (task.borrow().memory_size(), task.borrow().pid());
        // try to allocate memory
        match self.memory_manager.allocate(size, pid) {
            Ok(h) => {
                task.borrow_mut().unsuspend();
                task.borrow_mut().set_memory_range(h);
                println!("Task Unsuspended: {} with memory {:?}", task.borrow().pid(), task.borrow().memory_range());
                match task.borrow().state() {
                    Ready => self.task_queue.push(task.clone()),
                    Blocked => self.blocked_queue.push(task.clone()),
                    Terminated | New | Running => panic!("Error, cannot suspend {:?} task", task.borrow().state()),
                }
                Ok(())
            }
            Err(_) => {
                println!("failed to unsuspend task {}, out of memory", pid);
                Err(task)
            }
        }
    }
    pub fn terminate_task(&mut self, task: TaskRef) {
        println!("==> {} finished. ", task.borrow().pid());
        task.borrow_mut().set_state(Terminated);
        self.memory_manager.free(task.borrow().pid());
        let cond = self.pid_to_trigger.get(&task.borrow().pid()).unwrap().clone();
        cond.borrow_mut().set_ok();
    }
    pub fn high_level_schedule(&mut self) {
        let mut out_of_mem = vec![];
        // ready, suspend <=> unsuspend
        // occupy all available slots
        while self.has_available_slots() && !self.ready_suspend_queue.is_empty(){
            let to_unsuspend_task = self.ready_suspend_queue.pop().unwrap();
            match self.unsuspend_task(to_unsuspend_task) {
                Err(task) => out_of_mem.push(task),
                _ => (),
            }
        }
        self.ready_suspend_queue.extend(out_of_mem);
        let mut out_of_mem = vec![];
        // higher priority and less memory usage
        while !self.has_available_slots() && !self.task_queue.is_empty() && !self.ready_suspend_queue.is_empty() {
            let (active_task, suspended_task)
                = (self.task_queue.peek_min().unwrap(), self.ready_suspend_queue.peek().unwrap());
            // higher priority
            if active_task.borrow().priority() < suspended_task.borrow().priority() {
                // less mem
                if active_task.borrow().memory_size() >= suspended_task.borrow().memory_size() {
                    let active_task = self.task_queue.pop_min().unwrap();
                    let suspended_task = self.ready_suspend_queue.pop().unwrap();
                    self.suspend_task(active_task);
                    self.unsuspend_task(suspended_task);
                } else {
                    out_of_mem.push(self.ready_suspend_queue.pop().unwrap());
                }
            } else {
                break;
            }
        }
        self.ready_suspend_queue.extend(out_of_mem);
        let mut out_of_mem = vec![];
        while !self.has_available_slots() && !self.task_queue.is_empty() && !self.new_queue.is_empty() {
            let (active_task, new_task)
                = (self.task_queue.peek_min().unwrap(), self.new_queue.peek().unwrap());
            // higher priority
            if active_task.borrow().priority() < new_task.borrow().priority() {
                // less mem
                if active_task.borrow().memory_size() >= new_task.borrow().memory_size() {
                    let active_task = self.task_queue.pop_min().unwrap();
                    self.suspend_task(active_task);
                } else {
                    out_of_mem.push(self.new_queue.pop().unwrap());
                }
            } else {
                break;
            }
        }
        self.new_queue.extend(out_of_mem);
        // block, unsuspend => suspend
        // suspend blocked tasks if there are new tasks
        while !self.blocked_queue.is_empty() && !self.new_queue.is_empty() {
            let blocked_task = self.blocked_queue.pop_min().unwrap();
            self.suspend_task(blocked_task);
        }
    }
    pub fn mid_level_schedule(&mut self) {
        let mut out_of_mem = vec![];
        while !self.new_queue.is_empty() && self.has_available_slots() {
            let task = self.new_queue.pop().unwrap();
            // allocate mem
            let (size, pid) = (task.borrow().memory_size(), task.borrow().pid());
            match self.memory_manager.allocate(size, pid) {
                Ok(hole) => {
                    task.borrow_mut().set_memory_range(hole);
                    self.new_to_ready_task(task);
                }
                Err(_) => {
                    println!("No enough memory, {}", task.borrow().pid());
                    out_of_mem.push(task);
                }
            }
        }
        for i in out_of_mem.iter() {
            self.new_queue.push(i.clone());
        }
    }
    pub fn check_and_unblock(&mut self) {
        let mut not_ready = vec![];
        let mut ready = vec![];
        for t in self.blocked_queue.drain() {
            if t.borrow().is_cond_satisfied() {
                ready.push(t);
            } else {
                not_ready.push(t);
            }
        }
        for t in self.blocked_suspend_queue.drain() {
            if t.borrow().is_cond_satisfied() {
                ready.push(t);
            } else {
                not_ready.push(t);
            }
        }
        for i in ready.iter() {
            if self.has_available_slots() {
                self.ready_task(i.clone());
            } else {
                not_ready.push(i.clone());
            }
        }
        for i in not_ready.iter() {
            if i.borrow().is_suspended() {
                self.blocked_suspend_queue.push(i.clone());
            } else {
                self.blocked_queue.push(i.clone());
            }
        }
    }
    pub fn low_level_schedule(&mut self, proc: usize) {
        let mut new_task = self.task_queue.peek_max();
        while new_task.is_some() && !new_task.unwrap().borrow().is_cond_satisfied() {
            let temp = self.task_queue.pop_max().unwrap();
            self.block_task(temp);
            new_task = self.task_queue.peek_max();
        }
        let proc = &mut self.processors[proc];
        let mut preempt_flag = false;
        if let (Some(nt), Some(ot)) = (new_task, proc.executing_task()) {
            let (nt, ot) = (nt.clone(), ot.clone());
            if nt.borrow().priority() > ot.borrow().priority() {
                preempt_flag = true;
                println!("ready to preempt {} -> {}", nt.borrow().pid(), ot.borrow().pid());
            }
        }
        if preempt_flag || proc.is_task_finished() {
            let new_task = self.task_queue.pop_max();
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
    }
    pub fn schedule(&mut self) {
        println!("time {}: ", self.time);
        self.check_and_unblock();
        // suspend <=> unsuspend
        self.high_level_schedule();
        // new <=> ready
        self.mid_level_schedule();
        // ready <=> running
        for proc in 0..self.processors.len() {
            self.low_level_schedule(proc);
        }
    }
    pub fn advance_time(&mut self) {
        self.time += 1;
        self.schedule();
        // run task
        for proc in self.processors.iter_mut() {
            proc.run_task();
        }
        // self.memory_manager.print();
    }
    pub fn memory_manager(&self) -> &MemoryManager {
        &self.memory_manager
    }
    pub fn get_executing_tasks(&self) -> [Option<u32>; 2] {
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
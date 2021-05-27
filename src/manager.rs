use crate::scheduler::Scheduler;
use crate::task::Task;
use std::collections::HashMap;
use crate::memory::hole::Hole;

pub struct Manager {
    scheduler: Scheduler,
    pid_counter: u32,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            scheduler: Scheduler::new(4, 5),
            pid_counter: 1,
        }
    }
    pub fn create_task(&mut self, req_time: i32, priority: i32, memory_size: u32, pre: Option<u32>) {
        match self.scheduler.add_task(Task::new(self.pid_counter, req_time, priority, memory_size), pre) {
            Err(e) => eprintln!("Error occurred, unable to create new task: {:?}", e),
            _ => (),
        }
        self.pid_counter += 1;
    }
    pub fn advance(&mut self) {
        self.scheduler.advance_time();
    }
    pub fn get_mem_usage(&self) -> &HashMap<u32, Hole> {
        self.scheduler.memory_manager().get_mem_usage()
    }
    pub fn get_running_task(&self) -> [Option<u32>; 2] {
        self.scheduler.get_executing_tasks()
    }
    pub fn time(&self) -> i32 {
        self.scheduler.time()
    }
}
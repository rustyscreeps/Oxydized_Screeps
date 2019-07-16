//! Some doc
//!
//!

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use log::{error};
use serde::{Serialize, Deserialize};

use crate::process::{Message, Process, ProcessResult, ReturnValue, MaybeSerializedProcess};

#[derive(Serialize, Deserialize, Debug)]
pub struct Kernel {
    // Try and refactor those to using Vec<Option<u32>> instead
    process_table: BTreeMap<u32, ProcessInfo>,
    next_pid_number: u32,
    scheduler: Scheduler,
    current_tick: u32,
    wake_list: BTreeMap<u32, Vec<u32>>,
}

impl Kernel {
    pub fn new(init_proc: Box<dyn Process>, current_tick: u32) -> Self {
        let mut k = Kernel{
            process_table: BTreeMap::default(),
            next_pid_number: 0,
            scheduler: Scheduler::new(),
            current_tick,
            wake_list: BTreeMap::default(),
        };

        k.launch_process(init_proc, 0);

        k
    }

    pub fn run_next(&mut self, deserializer: &impl Fn(&Vec<u8>) -> Box<dyn Process>) -> bool {  // todo: have better return value?
        match self.scheduler.next() {
            Some(Task {ty, pid}) => {
                if let Some(mut pinfo) = self.process_table.remove(&pid) {  // todo: gather mutations
                    let process = pinfo.process.deserialized_process(deserializer);
                    if let Some(presult) = match ty {
                        TaskType::Start => process.start(),
                        TaskType::Run => process.run(),
                        TaskType::Join(result) => process.join(result),
                        TaskType::ReceiveMessage(msg) => process.receive(msg),
                    } {
                        self.process_result(presult, &mut pinfo, deserializer);
                    }
                    self.process_table.insert(pid, pinfo);
                }
                true
            }
            None => false
        }
    }

    fn process_result(&mut self, proc_res: ProcessResult, pinfo: &mut ProcessInfo, deserializer: &impl Fn(&Vec<u8>) -> Box<dyn Process>) {
        match proc_res {
            ProcessResult::Done(rv) => self.done(pinfo, rv, deserializer),
            ProcessResult::Yield => self.scheduler.schedule(pinfo.pid, TaskType::Run),
            ProcessResult::Sleep(duration) =>{
                let proc_to_wake = self.wake_list.entry(self.current_tick + duration).or_default();
                proc_to_wake.push(pinfo.pid);
            },
            ProcessResult::Fork(procs, proc_result) => {
                self.fork(procs, pinfo);
                self.process_result(*proc_result, pinfo, deserializer);
            },
            ProcessResult::Error(s) => error!{"Proc {}: {} -- {}", pinfo.pid, pinfo.process_type, s}
        }
    }

    fn fork(&mut self, new_procs: Vec<Box<dyn Process>>, pinfo: &mut ProcessInfo) {
        for p in new_procs {
            pinfo.children_processes.insert(self.launch_process(p, pinfo.pid));
        }
    }

    fn done(&mut self, pinfo: &ProcessInfo, rv: ReturnValue, deserializer: &impl Fn(&Vec<u8>) -> Box<dyn Process>) {
        self.scheduler.join_process(pinfo.parent_pid, rv);
        if let Some(parent) = self.process_table.get_mut(&pinfo.parent_pid) {
            parent.children_processes.remove(&pinfo.pid);
        }

        for cpid in pinfo.children_processes.iter() {
            if let Some(mut pinfo) = self.process_table.remove(cpid) {
                let process = pinfo.process.deserialized_process(deserializer);
                process.kill();
            }
        }

    }

    pub fn launch_process(&mut self, proc: Box<dyn Process>, parent_pid: u32) -> u32 {
        let pinfo = ProcessInfo::new(
            self.next_pid_number,
            parent_pid,
            proc.type_string(),
            MaybeSerializedProcess::De(proc));
        self.process_table.insert(self.next_pid_number, pinfo);     // todo:  Make this more robust.
        self.scheduler.schedule(self.next_pid_number, TaskType::Start);

        self.next_pid_number += 1;
        self.next_pid_number - 1
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessInfo {
    pid: u32,
    parent_pid: u32,
    children_processes: BTreeSet<u32>,
    process_type: String,
    process: MaybeSerializedProcess,  // replace this with an enum MaybeSerializedProcess
}

impl ProcessInfo {
    fn new(pid: u32, parent_pid:u32, type_str: String, process: MaybeSerializedProcess) -> Self {
        ProcessInfo {
            pid,
            parent_pid,
            children_processes: BTreeSet::new(),
            process_type: type_str.to_owned(),
            process,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Scheduler {
    task_queue: VecDeque<Task>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TaskType {
    Start,
    Run,
    Join(ReturnValue),
    ReceiveMessage(Message),
}

impl Default for TaskType {
    fn default() -> Self {
        TaskType::Start
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Task {
    ty: TaskType,
    pid: u32,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule(&mut self, pid: u32, ty: TaskType) {
        self.task_queue.push_back(Task { ty, pid });
    }

    #[allow(clippy::should_implement_trait)]  // Implement iter?
    pub fn next(&mut self) -> Option<Task> {
        self.task_queue.pop_front()
    }

    pub fn launch_process(&mut self, pid: u32) {
        self.schedule(pid, TaskType::Start);
    }

    pub fn join_process(&mut self, parent_pid: u32, result: ReturnValue){
        self.schedule(parent_pid, TaskType::Join(result));
    }

    pub fn send_message(&mut self, receiver_pid: u32, msg: Message) {
        self.schedule(receiver_pid, TaskType::ReceiveMessage(msg))
    }
}

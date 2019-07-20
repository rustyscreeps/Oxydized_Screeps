//! Some doc
//!
//!


// /////////////
// todo:
// /////////////
// - Remove most of the `pub`
// - Better killing of processes.





use std::collections::{BTreeMap, BTreeSet, VecDeque};

use log::{error};
use serde::{Serialize, Deserialize};

use crate::process::{BoxedProcess, Message, PResult, PSignalResult, ReturnValue, MaybeSerializedProcess};

#[derive(Serialize, Deserialize, Debug)]
pub struct Kernel {
    process_table: BTreeMap<u32, ProcessInfo>,
    next_pid_number: u32,
    scheduler: Scheduler,
    current_tick: u32,
    wake_list: BTreeMap<u32, Vec<u32>>,
}

impl Kernel {
    pub fn new(current_tick: u32) -> Self {
        Kernel{
            process_table: BTreeMap::default(),
            next_pid_number: 0,
            scheduler: Scheduler::new(),
            current_tick,
            wake_list: BTreeMap::default(),
        }
    }

    pub fn run_next(&mut self, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess) -> bool {
        if let Some(Task {ty, pid}) = self.scheduler.next() {
            // Grab ownership of the process and it's metadata
            if let Some(mut pinfo) = self.process_table.remove(&pid) {  // todo: gather mutations
                let process = pinfo.process.deserialized_process(deserializer);
                match ty {
                    TaskType::Start => {
                        let pr = process.start();
                        self.process_result(pr, pinfo, deserializer);
                    },
                    TaskType::Run => {
                        let pr = process.run();
                        self.process_result(pr, pinfo, deserializer);
                    },
                    TaskType::Join(result) => {
                        let pr = process.join(result);
                        self.process_signal_result(pr, pinfo, deserializer);
                    },
                    TaskType::ReceiveMessage(msg) => {
                        let pr = process.receive(msg);
                        self.process_signal_result(pr, pinfo, deserializer);
                    },
                };
            }
            true
        } else {
            false
        }
    }

    pub fn launch_process(&mut self, proc: BoxedProcess, parent_pid: Option<u32>) -> u32 {
        let pinfo = ProcessInfo::new(
            self.next_pid_number,
            parent_pid,
            proc.type_id(),
            MaybeSerializedProcess::De(proc));
        self.process_table.insert(self.next_pid_number, pinfo);     // todo:  Make this more robust.
        self.scheduler.schedule(self.next_pid_number, TaskType::Start);

        self.next_pid_number += 1;
        self.next_pid_number - 1
    }

    pub fn next_tick(&mut self) {
        self.current_tick += 1;

        // Wake processes
        if let Some(procs_to_wake) = self.wake_list.remove(&self.current_tick) {
            for pid in procs_to_wake.iter() {
                self.scheduler.reschedule(pid.clone());
            }
        }
    }

    pub fn tick(&self) -> u32 {
        self.current_tick
    }

    fn process_result(&mut self, proc_res: PResult, mut pinfo: ProcessInfo, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess) {
        match proc_res {
            PResult::Done(rv) => {
                self.join_parent(&pinfo, rv);
                self.terminate(pinfo, deserializer);
            },
            PResult::Yield => self.scheduler.reschedule(pinfo.pid),
            PResult::Sleep(duration) => {
                let procs_to_wake = self.wake_list.entry(self.current_tick + duration).or_default();
                procs_to_wake.push(pinfo.pid);
                self.process_table.insert(pinfo.pid, pinfo);
            },
            PResult::Wait => {
                self.process_table.insert(pinfo.pid, pinfo);
            },
            PResult::Fork(procs, proc_result) => {
                self.fork(procs, &mut pinfo);
                self.process_result(*proc_result, pinfo, deserializer);
            },
            PResult::Error(s) => {
                error!{"Proc {}: {} -- {}\n     Killing process...", pinfo.pid, pinfo.process_id, s}
                self.terminate(pinfo, deserializer);
            },
        };
    }

    fn process_signal_result(&mut self, proc_res: PSignalResult, mut pinfo: ProcessInfo, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess) {
        match proc_res {
            PSignalResult::None => {
                self.process_table.insert(pinfo.pid, pinfo);
            },
            PSignalResult::Done(rv) => {
                self.join_parent(&pinfo, rv);
                self.terminate(pinfo, deserializer);
            },
            PSignalResult::Fork(procs, proc_result) => {
                self.fork(procs, &mut pinfo);
                self.process_signal_result(*proc_result, pinfo, deserializer);
            },
            PSignalResult::Error(s) => {
                error!{"Proc {}: {} -- {}\n     Killing process...", pinfo.pid, pinfo.process_id, s}
                self.terminate(pinfo, deserializer);
            },
        };
    }

    fn fork(&mut self, new_procs: Vec<BoxedProcess>, pinfo: &mut ProcessInfo) {
        for p in new_procs {
            pinfo.children_processes.insert(self.launch_process(p, Some(pinfo.pid)));
        }
    }

    fn join_parent(&mut self, pinfo: &ProcessInfo, rv: ReturnValue) {
        if let Some(parent_pid) = pinfo.parent_pid {
            if let Some(parent) = self.process_table.get_mut(&parent_pid) {
                self.scheduler.join_process(parent_pid, rv);
                parent.children_processes.remove(&pinfo.pid);
            }
        }
    }

    fn terminate(&mut self, mut pinfo: ProcessInfo, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess) {
        for cpid in pinfo.children_processes.iter() {
            if let Some(cpinfo) = self.process_table.remove(cpid) {
                let process = pinfo.process.deserialized_process(deserializer);
                process.kill();
                self.terminate(cpinfo, deserializer);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessInfo {
    pid: u32,
    parent_pid: Option<u32>,
    children_processes: BTreeSet<u32>,
    process_id: u32,
    process: MaybeSerializedProcess,
}

impl ProcessInfo {
    fn new(pid: u32, parent_pid: Option<u32>, type_id: u32, process: MaybeSerializedProcess) -> Self {
        ProcessInfo {
            pid,
            parent_pid,
            children_processes: BTreeSet::new(),
            process_id: type_id,
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

    #[allow(clippy::should_implement_trait)]  // Implement iter?
    pub fn next(&mut self) -> Option<Task> {
        self.task_queue.pop_front()
    }

    pub fn launch_process(&mut self, pid: u32) {
        self.schedule(pid, TaskType::Start);
    }

    pub fn reschedule(&mut self, pid: u32) {
        self.schedule(pid, TaskType::Run);
    }

    pub fn join_process(&mut self, parent_pid: u32, result: ReturnValue){
        self.schedule(parent_pid, TaskType::Join(result));
    }

    pub fn send_message(&mut self, receiver_pid: u32, msg: Message) {
        self.schedule(receiver_pid, TaskType::ReceiveMessage(msg))
    }

    fn schedule(&mut self, pid: u32, ty: TaskType) {
        // Todo: Might want to check whether the process has already been scheduled?
        self.task_queue.push_back(Task { ty, pid });
    }
}

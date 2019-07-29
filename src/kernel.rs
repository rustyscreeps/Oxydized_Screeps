//! Some doc
//!
//!

// /////////////
// todo:
// /////////////
// - Remove most of the `pub`
// - Better killing of processes.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use log::error;
use serde::{Deserialize, Serialize};

use crate::process::{
    BoxedProcess, MaybeSerializedProcess, Message, PResult, PSignalResult, ReturnValue,
};

#[derive(Serialize, Deserialize, Debug)]
pub struct Kernel {
    process_table: BTreeMap<u32, MaybeSerializedProcess>,
    info_table: BTreeMap<u32, ProcessInfo>,
    next_pid_number: u32,
    scheduler: Scheduler,
    current_tick: u32,
    wake_list: BTreeMap<u32, Vec<u32>>,
}

impl Kernel {
    pub fn new(current_tick: u32) -> Self {
        Kernel {
            process_table: BTreeMap::default(),
            info_table: BTreeMap::default(),
            next_pid_number: 0,
            scheduler: Scheduler::new(),
            current_tick,
            wake_list: BTreeMap::default(),
        }
    }

    pub fn run_next(&mut self, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess) -> bool {
        if let Some(Task { ty, pid }) = self.scheduler.next() {
            if let Some(ser_proc) = self.process_table.get_mut(&pid) {
                let process = ser_proc.deserialized_process(deserializer);

                match ty {
                    TaskType::Start => {
                        let pr = process.start();
                        self.process_result(pr, pid, deserializer);
                    }
                    TaskType::Run => {
                        let pr = process.run();
                        self.process_result(pr, pid, deserializer);
                    }
                    TaskType::Join(result) => {
                        let pr = process.join(result);
                        self.process_signal_result(pr, pid, deserializer);
                    }
                    TaskType::ReceiveMessage(msg) => {
                        let pr = process.receive(msg);
                        self.process_signal_result(pr, pid, deserializer);
                    }
                };
            };
            true
        } else {
            false
        }
    }

    pub fn launch_process(&mut self, proc: BoxedProcess, parent_pid: Option<u32>) -> u32 {
        let pinfo = ProcessInfo::new(self.next_pid_number, parent_pid, proc.type_id());
        self.info_table.insert(self.next_pid_number, pinfo); // todo:  Make this more robust.

        self.process_table
            .insert(self.next_pid_number, MaybeSerializedProcess::De(proc));
        self.scheduler
            .schedule(self.next_pid_number, TaskType::Start);

        self.next_pid_number += 1;
        self.next_pid_number - 1
    }

    pub fn next_tick(&mut self) {
        self.current_tick += 1;

        // Wake processes
        if let Some(procs_to_wake) = self.wake_list.remove(&self.current_tick) {
            for pid in procs_to_wake.into_iter() {
                self.scheduler.reschedule(pid);
            }
        }

        // Then add recurrent tasks
        self.scheduler.next_tick();
    }

    pub fn tick(&self) -> u32 {
        self.current_tick
    }

    fn process_result(
        &mut self,
        proc_res: PResult,
        pid: u32,
        deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess,
    ) {
        match proc_res {
            PResult::Done(rv) => {
                self.join_parent(pid, rv);
                self.terminate(pid, deserializer);
            }
            PResult::Yield => {
                self.scheduler.reschedule(pid);
            }
            PResult::YieldTick => {
                self.scheduler.schedule_next_tick(pid);
            }
            PResult::Sleep(duration) => {
                self.wake_list
                    .entry(self.current_tick + duration)
                    .or_default()
                    .push(pid);
            }
            PResult::Wait => {}
            PResult::Fork(procs, proc_result) => {
                self.fork(procs, pid);
                self.process_result(*proc_result, pid, deserializer);
            }
            PResult::Error(s) => {
                let pinfo = self.info_table.get(&pid).unwrap();
                error!(
                    "Proc {}: {} -- {}\n     Killing process...",
                    pid, pinfo.process_type_id, s
                );
                self.terminate(pid, deserializer);
            }
        };
    }

    fn process_signal_result(
        &mut self,
        proc_res: PSignalResult,
        pid: u32,
        deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess,
    ) {
        match proc_res {
            PSignalResult::None => {}
            PSignalResult::Done(rv) => {
                self.join_parent(pid, rv);
                self.terminate(pid, deserializer);
            }
            PSignalResult::Fork(procs, proc_result) => {
                self.fork(procs, pid);
                self.process_signal_result(*proc_result, pid, deserializer);
            }
            PSignalResult::Error(s) => {
                let pinfo = self.info_table.get(&pid).unwrap();
                error! {"Proc {}: {} -- {}\n     Killing process...", pid, pinfo.process_type_id, s}
                self.terminate(pid, deserializer);
            }
        };
    }

    fn fork(&mut self, new_procs: Vec<BoxedProcess>, pid: u32) -> Vec<u32> {
        let mut cpids = Vec::new();
        for p in new_procs {
            let cpid = self.launch_process(p, Some(pid));
            cpids.push(cpid);
        }

        let pinfo = self.info_table.get_mut(&pid).unwrap();
        pinfo.children_processes.extend(cpids.iter());

        cpids
    }

    fn join_parent(&mut self, pid: u32, rv: Option<ReturnValue>) {
        if let Some(pinfo) = self.info_table.get(&pid) {
            if let Some(parent_pid) = pinfo.parent_pid {
                if let Some(parent) = self.info_table.get_mut(&parent_pid) {
                    self.scheduler.join_process(parent_pid, rv);
                    parent.children_processes.remove(&pid);
                }
            }
        }
    }

    fn terminate(&mut self, pid: u32, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess) {
        if let Some(pinfo) = self.info_table.remove(&pid) {
            for cpid in pinfo.children_processes.iter() {
                self.terminate(*cpid, deserializer);
            }
        }
        if let Some(mut ser_proc) = self.process_table.remove(&pid) {
            ser_proc.deserialized_process(deserializer).kill();
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessInfo {
    pid: u32,
    parent_pid: Option<u32>,
    children_processes: BTreeSet<u32>,
    process_type_id: u32,
}

impl ProcessInfo {
    fn new(pid: u32, parent_pid: Option<u32>, process_type_id: u32) -> Self {
        ProcessInfo {
            pid,
            parent_pid,
            children_processes: BTreeSet::new(),
            process_type_id,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub enum TaskType {
    Start,
    Run,
    Join(Option<ReturnValue>),
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Scheduler {
    task_queue: VecDeque<Task>,
    next_tick_tasks: VecDeque<Task>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::should_implement_trait)] // Implement iter?
    pub fn next(&mut self) -> Option<Task> {
        self.task_queue.pop_front()
    }

    pub fn launch_process(&mut self, pid: u32) {
        self.schedule(pid, TaskType::Start);
    }

    pub fn reschedule(&mut self, pid: u32) {
        self.schedule(pid, TaskType::Run);
    }

    pub fn schedule_next_tick(&mut self, pid: u32) {
        self.next_tick_tasks.push_back(Task {
            ty: TaskType::Run,
            pid,
        });
    }

    pub fn join_process(&mut self, parent_pid: u32, result: Option<ReturnValue>) {
        self.schedule(parent_pid, TaskType::Join(result));
    }

    pub fn send_message(&mut self, receiver_pid: u32, msg: Message) {
        self.schedule(receiver_pid, TaskType::ReceiveMessage(msg))
    }

    pub fn next_tick(&mut self) {
        self.task_queue.append(&mut self.next_tick_tasks);
    }

    fn schedule(&mut self, pid: u32, ty: TaskType) {
        // Todo: Might want to check whether the process has already been scheduled?
        self.task_queue.push_back(Task { ty, pid });
    }
}

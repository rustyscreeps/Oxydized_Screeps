use oxydized_screeps::kernel::SysCall;
use oxydized_screeps::*;

use bincode;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct ParentProcess {
    h: String,
    s: bool,
}

impl Process for ParentProcess {
    type Content = ();
    type Return = String;

    fn start(&mut self, mut sc: SysCall<Self::Content, Self::Return>) -> PResult<Self::Return> {
        let cproc = Box::new(ChildProcess::new());
        sc.fork(vec![cproc]);
        PResult::YieldTick
    }

    fn run(&mut self, mut sc: SysCall<Self::Content, Self::Return>) -> PResult<Self::Return> {
        self.s = true;
        let cproc = Box::new(ChildProcess::new());
        sc.fork(vec![cproc]);
        PResult::YieldTick
    }

    fn join(
        &mut self,
        _: SysCall<Self::Content, Self::Return>,
        return_value: Option<Self::Return>,
    ) -> PSignalResult<Self::Return> {
        if let Some(value) = return_value {
            if self.s {
                return PSignalResult::Done(Some(format!("{}{}", self.h, value)));
            }
        }
        PSignalResult::None
    }

    fn type_id(&self) -> u32 {
        1
    }

    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

impl ParentProcess {
    fn new() -> Self {
        ParentProcess {
            h: "Hello, ".to_owned(),
            s: false,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct ChildProcess {
    w: String,
    s: bool,
    n_run: u32,
}

impl Process for ChildProcess {
    type Content = ();
    type Return = String;

    fn start(&mut self, _: SysCall<Self::Content, Self::Return>) -> PResult<Self::Return> {
        self.s = true;
        PResult::Sleep(3)
    }

    fn run(&mut self, _: SysCall<Self::Content, Self::Return>) -> PResult<Self::Return> {
        self.n_run += 1;
        PResult::Done(Some(self.w.clone()))
    }

    fn type_id(&self) -> u32 {
        2
    }

    fn to_bytes(&self) -> Vec<u8> {
        bincode::serialize(self).unwrap()
    }
}

impl ChildProcess {
    fn new() -> Self {
        ChildProcess {
            w: "World".to_owned(),
            s: false,
            n_run: 0,
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

fn deserialize_process(type_id: u32, bytes: &[u8]) -> BoxedProcess<(), String> {
    match type_id {
        1 => Box::new(ParentProcess::from_bytes(bytes)),
        2 => Box::new(ChildProcess::from_bytes(bytes)),
        _ => panic!("bad process number"),
    }
}

#[test]
fn empty_kernel() {
    let mut ker = Kernel::<(), String>::new(10);

    for _ in 0..10 {
        ker.run_next(&deserialize_process);
    }

    let s = bincode::serialize(&ker).unwrap();
    let mut de_ker: Kernel<(), String> = bincode::deserialize(&s).unwrap();

    de_ker.next_tick();
}

#[test]
fn lauch_single_process() {
    let mut ker = Kernel::<(), String>::new(10);

    ker.launch_process(Box::new(ChildProcess::new()), None);

    for _ in 0..10 {
        ker.run_next(&deserialize_process);
    }

    let s = bincode::serialize(&ker).unwrap();
    let mut de_ker: Kernel<(), String> = bincode::deserialize(&s).unwrap();

    de_ker.next_tick();
}

#[test]
fn lauch_child_process() {
    let mut ker = Kernel::<(), String>::new(10);
    ker.launch_process(Box::new(ParentProcess::new()), None);

    for _ in 0..10 {
        while ker.run_next(&deserialize_process) {}

        let s = bincode::serialize(&ker).unwrap();
        ker = bincode::deserialize(&s).unwrap();
        ker.next_tick()
    }
}

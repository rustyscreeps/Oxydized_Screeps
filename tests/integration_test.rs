use oxidized_screeps::*;

use bincode;
use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Debug)]
struct ParentProcess {
    h: String,
    s: bool,
}

impl Process for ParentProcess {
    fn start(&mut self) -> PResult {
        let cproc = Box::new(ChildProcess::new());
        PResult::Fork(vec![cproc], PResult::Yield.into())
    }

    fn run(&mut self) -> PResult {
        self.s = true;
        let cproc = Box::new(ChildProcess::new());
        PResult::Fork(vec![cproc], PResult::Yield.into())
    }

    fn join(&mut self, return_value: ReturnValue) -> PSignalResult{
        if self.s {
            PSignalResult::Done(ReturnValue::new(&self.h))
        } else {
            PSignalResult::None
        }
    }

    fn receive(&mut self, msg: Message) -> PSignalResult {
        PSignalResult::None
    }

    fn kill(&mut self) {}

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

    fn from_bytes(bytes: &Vec<u8>) -> Self {
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
    fn start(&mut self) -> PResult {
        self.s = true;
        PResult::Sleep(3)
    }

    fn run(&mut self) -> PResult {
        self.n_run += 1;
        PResult::Done(ReturnValue::new(&self.w))
    }

    fn kill(&mut self) {}

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

    fn from_bytes(bytes: &Vec<u8>) -> Self {
        bincode::deserialize(bytes).unwrap()
    }
}

fn deserialize_process(type_id: u32, bytes: &Vec<u8>) -> Box<dyn Process> {
    match type_id {
        1 => Box::new(ParentProcess::from_bytes(bytes)),
        2 => Box::new(ChildProcess::from_bytes(bytes)),
        _ => panic!("bad process number"),
    }
}


#[test]
fn empty_kernel() {
    let mut ker = Kernel::new(10);

    for _ in 0..10 {
        ker.run_next(&deserialize_process);
    }

    let s = bincode::serialize(&ker).unwrap();
    let mut de_ker: Kernel = bincode::deserialize(&s).unwrap();

    de_ker.next_tick();
}

#[test]
fn lauch_single_process() {
    let mut ker = Kernel::new(10);

    ker.launch_process(Box::new(ChildProcess::new()), None);

    for _ in 0..10 {
        ker.run_next(&deserialize_process);
    }

    let s = bincode::serialize(&ker).unwrap();
    let mut de_ker: Kernel = bincode::deserialize(&s).unwrap();

    de_ker.next_tick();
}

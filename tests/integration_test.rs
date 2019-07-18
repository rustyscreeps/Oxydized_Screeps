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
}

#[derive(Serialize, Deserialize, Debug)]
struct ChildProcess {
    w: String,
    s: bool,
}

    impl Process for ChildProcess {
    fn start(&mut self) -> PResult {
        self.s = true;
        PResult::Sleep(3)
    }

    fn run(&mut self) -> PResult {
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
        }
    }
}




#[test]
fn schedule_a_process() {
    assert_eq!(2 + 2, 4);
}

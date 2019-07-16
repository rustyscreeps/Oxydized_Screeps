pub mod kernel;
pub mod process;

pub use crate::{
    kernel::{Kernel},
    process::{Process},
};


// #[cfg(test)]
// mod tests {
//     use serde::{Deserialize, Serialize};

//     use super::*;

//     #[derive(Serialize, Deserialize, Debug)]
//     struct ParentProcess {
//         h: String,
//         s: bool,
//     }

//     impl Process for ParentProcess {
//         fn start(&mut self) -> Option<ProcessResult> {

//         }

//         fn run(&mut self) -> Option<ProcessResult>;

//         fn join(&mut self, return_value: ReturnValue) -> Option<ProcessResult>;

//         fn kill(&mut self);

//         fn receive(&mut self, msg: Message) -> Option<ProcessResult>;

//         fn type_string(&self) -> String;
// }

//     #[derive(Serialize, Deserialize, Debug)]
//     struct ChildProcess {
//         w: String,
//         s: bool,
//     }




//     #[test]
//     fn schedule_a_process() {
//         assert_eq!(2 + 2, 4);
//     }
// }

//! Some doc
//!
//!

use std::fmt;

use erased_serde::{self, serialize_trait_object, __internal_serialize_trait_object};
use serde;

pub trait Process: erased_serde::Serialize {
    fn start(&mut self) -> Option<ProcessResult>;

    fn run(&mut self) -> Option<ProcessResult>;

    fn join(&mut self, return_value: ReturnValue) -> Option<ProcessResult>;

    fn kill(&mut self);

    fn receive(&mut self, msg: Message) -> Option<ProcessResult>;

    fn type_string(&self) -> String;
}

serialize_trait_object!(Process);

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Message ();

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct ReturnValue {
    pub pid: u32,
    pub value: String,
}

pub enum ProcessResult {
    Done(ReturnValue),
    Yield,
    Sleep(u32),
    Fork(Vec<Box<dyn Process>>, Box<ProcessResult>),
    Error(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
#[serde(from = "Vec<u8>")]
pub enum MaybeSerializedProcess {
    Ser(Vec<u8>),
    #[serde(skip_deserializing)]
    De(Box<dyn Process>),
}

impl MaybeSerializedProcess {
    #[allow(clippy::borrowed_box)]
    pub fn deserialized_process(&mut self, deserializer: &impl Fn(&Vec<u8>) -> Box<dyn Process>) -> &mut Box<dyn Process> {
        match self {
            MaybeSerializedProcess::Ser(bytes) => {
                let process = deserializer(bytes);
                *self = MaybeSerializedProcess::De(process);
                self.deserialized_process(deserializer)
            },
            MaybeSerializedProcess::De(process) => process,
        }
    }
}

impl From<Vec<u8>> for MaybeSerializedProcess {
    fn from(v: Vec<u8>) -> Self {
        MaybeSerializedProcess::Ser(v)
    }
}

impl fmt::Debug for Box<dyn Process> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process {{ type: {} }}", self.type_string())
    }
}

// impl<'de> serde::Deserialize<'de> for Box<dyn Process> {
//     fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
//     where
//         D: serde::Deserializer<'de> {
//             // Figure out how to capture users' types.
//             Err(serde::de::Error::custom("Figure out how to implement this!"))
//     }
// }

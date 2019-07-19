//! Some doc
//!
//!

use std::fmt;

// use erased_serde::{self, serialize_trait_object, __internal_serialize_trait_object};
// use serde;
use serde::{Serialize, Deserialize, Serializer, Deserializer};

pub trait Process {
    fn start(&mut self) -> PResult {
        PResult::Yield
    }

    fn run(&mut self) -> PResult;

    #[allow(unused_variables)]
    fn join(&mut self, return_value: ReturnValue) -> PSignalResult{
        PSignalResult::None
    }

    #[allow(unused_variables)]
    fn receive(&mut self, msg: Message) -> PSignalResult {
        PSignalResult::None
    }

    fn kill(&mut self) {}

    fn type_id(&self) -> u32;

    fn to_bytes(&self) -> Vec<u8>;
}

// serialize_trait_object!(Process);

#[derive(Serialize, Deserialize, Debug)]
pub struct Message ();

#[derive(Serialize, Deserialize, Debug)]
pub struct ReturnValue {
    pub value: String,
}

impl ReturnValue {
    pub fn new(value: &str) -> Self {
        ReturnValue {
            value: value.to_owned()
        }
    }
}

pub enum PResult {
    Done(ReturnValue),
    Yield,
    Sleep(u32),
    Wait,
    Fork(Vec<Box<dyn Process>>, Box<Self>),
    Error(String),
}

pub enum PSignalResult {
    Done(ReturnValue), // Short-circuit the `run` method
    Fork(Vec<Box<dyn Process>>, Box<Self>),
    Error(String),
    None, // Do nothing.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializedProcess {
    type_id: u32,
    bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum MaybeSerializedProcess {
    Ser(SerializedProcess),
    De(Box<dyn Process>),
}

impl MaybeSerializedProcess {

    // Still unclear whether you'd ever want to go from De to Ser

    // pub fn serialize(&mut self) {
    //     match self {
    //         MaybeSerializedProcess::Ser(_) => (),
    //         MaybeSerializedProcess::De(obj) => {
    //             let bytes = obj.to_bytes();
    //             *self = MaybeSerializedProcess::Ser(bytes);
    //         },
    //     };
    // }
    //
    // pub fn serialized_bytes(&mut self) -> &Vec<u8> {
    //     match self {
    //         MaybeSerializedProcess::Ser(bytes) => bytes,
    //         MaybeSerializedProcess::De(obj) => {
    //             obj.to_bytes()
    //         },
    //     }
    // }

    pub fn deserialize(&mut self, deserializer: &impl Fn(u32, &[u8]) -> Box<dyn Process>){
        match self {
            MaybeSerializedProcess::Ser(sp) => {
                let process = deserializer(sp.type_id, &sp.bytes);
                *self = MaybeSerializedProcess::De(process);
            },
            MaybeSerializedProcess::De(_) => (),
        }
    }

    #[allow(clippy::borrowed_box)]
    pub fn deserialized_process(&mut self, deserializer: &impl Fn(u32, &[u8]) -> Box<dyn Process>) -> &mut Box<dyn Process> {
        self.deserialize(deserializer);
        match self {
            MaybeSerializedProcess::De(process) => process,
            _ => panic!("Deserialization of a process failed!")
        }
    }
}

impl Serialize for MaybeSerializedProcess {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MaybeSerializedProcess::Ser(sp) => sp.serialize(serializer),
            MaybeSerializedProcess::De(obj) => {
                SerializedProcess {
                    type_id: obj.type_id(),
                    bytes: obj.to_bytes(),
                }.serialize(serializer)
            }
        }
    }
}

impl<'de> Deserialize<'de> for MaybeSerializedProcess {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de> {
        SerializedProcess::deserialize(deserializer)
            .map(|sp| MaybeSerializedProcess::Ser(sp))
    }
}

// impl From<Vec<u8>> for MaybeSerializedProcess {
//     fn from(v: Vec<u8>) -> Self {
//         MaybeSerializedProcess::Ser(v)
//     }
// }

// impl From<MaybeSerializedProcess> for Vec<u8> {
//     fn from(v: MaybeSerializedProcess) -> Self {
//         match v {
//             MaybeSerializedProcess::Ser(vec) => vec,
//             MaybeSerializedProcess::De(obj) => obj.to_bytes(),
//         }
//     }
// }

impl fmt::Debug for Box<dyn Process> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process {{ type: {} }}", self.type_id())
    }
}

//! Some doc
//!
//!

use std::fmt;

// use erased_serde::{self, serialize_trait_object, __internal_serialize_trait_object};
// use serde;
use serde::{Serialize, Deserialize, Serializer};

pub trait Process {
    fn start(&mut self) -> PResult {
        PResult::Yield
    }

    fn run(&mut self) -> PResult;

    fn join(&mut self, return_value: ReturnValue) -> PSignalResult{
        PSignalResult::None
    }

    fn receive(&mut self, msg: Message) -> PSignalResult {
        PSignalResult::None
    }

    fn kill(&mut self) {}

    fn type_string(&self) -> String;

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

#[derive(Deserialize, Debug)]
#[serde(untagged)]
#[serde(from = "Vec<u8>")]
pub enum MaybeSerializedProcess {
    Ser(Vec<u8>),
    #[serde(skip)]
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

    pub fn deserialize(&mut self, deserializer: &impl Fn(&Vec<u8>) -> Box<dyn Process>){
        match self {
            MaybeSerializedProcess::Ser(bytes) => {
                let process = deserializer(bytes);
                *self = MaybeSerializedProcess::De(process);
            },
            MaybeSerializedProcess::De(_) => (),
        }
    }

    #[allow(clippy::borrowed_box)]
    pub fn deserialized_process(&mut self, deserializer: &impl Fn(&Vec<u8>) -> Box<dyn Process>) -> &mut Box<dyn Process> {
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
            MaybeSerializedProcess::Ser(vec) => vec.serialize(serializer),
            MaybeSerializedProcess::De(obj) => obj.to_bytes().serialize(serializer)
        }
    }
}

impl From<Vec<u8>> for MaybeSerializedProcess {
    fn from(v: Vec<u8>) -> Self {
        MaybeSerializedProcess::Ser(v)
    }
}

impl From<MaybeSerializedProcess> for Vec<u8> {
    fn from(v: MaybeSerializedProcess) -> Self {
        match v {
            MaybeSerializedProcess::Ser(vec) => vec,
            MaybeSerializedProcess::De(obj) => obj.to_bytes(),
        }
    }
}

impl fmt::Debug for Box<dyn Process> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process {{ type: {} }}", self.type_string())
    }
}

//! Some doc
//!
//!

use std::fmt;

// use erased_serde::{self, serialize_trait_object, __internal_serialize_trait_object};
// use serde;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::kernel::SysCall;

pub type BoxedProcess<T> = Box<dyn Process<Content=T> + Sync + Send>;

pub trait Process {
    type Content;

    #[allow(unused_variables)]
    fn start(&mut self, syscall: SysCall<Self::Content>) -> PResult {
        PResult::Yield
    }

    fn run(&mut self, syscall: SysCall<Self::Content>) -> PResult;

    #[allow(unused_variables)]
    fn join(&mut self, syscall: SysCall<Self::Content>, return_value: Option<ReturnValue>) -> PSignalResult {
        PSignalResult::None
    }

    #[allow(unused_variables)]
    fn receive(&mut self, syscall: SysCall<Self::Content>, msg: Message<Self::Content>) -> PSignalResult {
        PSignalResult::None
    }

    fn kill(&mut self) {}

    fn type_id(&self) -> u32;

    fn to_bytes(&self) -> Vec<u8>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ReturnValue {
    pub value: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message<T> {
    sender_pid: u32,
    msg: T,
}

impl ReturnValue {
    pub fn new(value: &str) -> Self {
        ReturnValue {
            value: value.to_owned(),
        }
    }
}

pub enum PResult {
    // Todo: Maybe add a way to track state (u8) which could be matched on as entry points
    Done(Option<ReturnValue>),
    Yield,
    YieldTick,
    Sleep(u32),
    Wait,
    Error(String),
}

pub enum PSignalResult {
    Done(Option<ReturnValue>), // Short-circuit the `run` method
    Error(String),
    None, // Do nothing.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializedProcess {
    type_id: u32,
    bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum MaybeSerializedProcess<T> {
    Ser(SerializedProcess),
    De(BoxedProcess<T>),
}

impl<T> MaybeSerializedProcess<T> {
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

    pub fn deserialize(&mut self, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess<T>) {
        match self {
            MaybeSerializedProcess::Ser(sp) => {
                let process = deserializer(sp.type_id, &sp.bytes);
                *self = MaybeSerializedProcess::De(process);
            }
            MaybeSerializedProcess::De(_) => (),
        }
    }

    #[allow(clippy::borrowed_box)]
    pub fn deserialized_process(
        &mut self,
        deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess<T>,
    ) -> &mut BoxedProcess<T> {
        self.deserialize(deserializer);
        match self {
            MaybeSerializedProcess::De(process) => process,
            _ => panic!("Deserialization of a process failed!"),
        }
    }
}

impl<T> Serialize for MaybeSerializedProcess<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            MaybeSerializedProcess::Ser(sp) => sp.serialize(serializer),
            MaybeSerializedProcess::De(obj) => SerializedProcess {
                type_id: obj.type_id(),
                bytes: obj.to_bytes(),
            }
            .serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for MaybeSerializedProcess<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        SerializedProcess::deserialize(deserializer).map(MaybeSerializedProcess::Ser)
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

impl<T> fmt::Debug for BoxedProcess<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process {{ type: {} }}", self.type_id())
    }
}

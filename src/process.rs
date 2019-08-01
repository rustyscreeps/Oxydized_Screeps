//! Some doc
//!
//!

use std::fmt;

// use erased_serde::{self, serialize_trait_object, __internal_serialize_trait_object};
// use serde;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::kernel::SysCall;

pub type BoxedProcess<M, R> = Box<dyn Process<Content = M, Return = R> + Sync + Send>;

pub trait Process {
    type Content;
    type Return;

    #[allow(unused_variables)]
    fn start(&mut self, syscall: SysCall<Self::Content, Self::Return>) -> PResult<Self::Return> {
        PResult::Yield
    }

    fn run(&mut self, syscall: SysCall<Self::Content, Self::Return>) -> PResult<Self::Return>;

    #[allow(unused_variables)]
    fn join(
        &mut self,
        syscall: SysCall<Self::Content, Self::Return>,
        return_value: Option<Self::Return>,
    ) -> PSignalResult<Self::Return> {
        PSignalResult::None
    }

    #[allow(unused_variables)]
    fn receive(
        &mut self,
        syscall: SysCall<Self::Content, Self::Return>,
        msg: Message<Self::Content>,
    ) -> PSignalResult<Self::Return> {
        PSignalResult::None
    }

    fn kill(&mut self) {}

    fn type_id(&self) -> u32;

    fn to_bytes(&self) -> Vec<u8>;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Message<T> {
    sender_pid: u32,
    msg: T,
}


pub enum PResult<R> {
    // Todo: Maybe add a way to track state (u8) which could be matched on as entry points
    Done(Option<R>),
    Yield,
    YieldTick,
    Sleep(u32),
    Wait,
    Error(String),
}

pub enum PSignalResult<R> {
    Done(Option<R>), // Short-circuit the `run` method
    Error(String),
    None, // Do nothing.
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SerializedProcess {
    type_id: u32,
    bytes: Vec<u8>,
}

#[derive(Debug)]
pub enum MaybeSerializedProcess<M, R> {
    Ser(SerializedProcess),
    De(BoxedProcess<M, R>),
}

impl<M, R> MaybeSerializedProcess<M, R> {
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

    pub fn deserialize(&mut self, deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess<M, R>) {
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
        deserializer: &impl Fn(u32, &[u8]) -> BoxedProcess<M, R>,
    ) -> &mut BoxedProcess<M, R> {
        self.deserialize(deserializer);
        match self {
            MaybeSerializedProcess::De(process) => process,
            _ => panic!("Deserialization of a process failed!"),
        }
    }
}

impl<M, R> Serialize for MaybeSerializedProcess<M, R> {
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

impl<'de, M, R> Deserialize<'de> for MaybeSerializedProcess<M, R> {
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

impl<M, R> fmt::Debug for BoxedProcess<M, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Process {{ type: {} }}", self.type_id())
    }
}

//! Some doc
//!
//!

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

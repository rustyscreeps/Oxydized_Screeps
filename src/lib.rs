pub mod kernel;
pub mod process;

pub use crate::{
    kernel::{Kernel},
    process::{Process, PResult, PSignalResult, ReturnValue, Message, BoxedProcess},
};


#[cfg(test)]
mod tests {

}

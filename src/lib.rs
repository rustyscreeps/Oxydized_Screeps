pub mod kernel;
pub mod process;

pub use crate::{
    kernel::{Kernel},
    process::{Process, PResult, PSignalResult, ReturnValue, Message},
};


#[cfg(test)]
mod tests {

}

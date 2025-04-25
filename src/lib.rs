#![no_std]

#[cfg(feature = "async")]
pub mod asynch;
#[cfg(feature = "blocking")]
pub mod blocking;
pub mod error;
pub mod types;

mod utils;

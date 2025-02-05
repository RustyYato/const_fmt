#![cfg_attr(not(test), no_std)]

#[macro_use]
#[doc(hidden)]
pub mod macros;

mod buffer;
mod byte_buffer;

pub use buffer::{Buffer, BufferWriteFailed};
pub use byte_buffer::{ByteBuffer, Concat};

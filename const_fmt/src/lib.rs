#![cfg_attr(not(test), no_std)]

mod buffer;
mod byte_buffer;

pub use buffer::Buffer;
pub use byte_buffer::{ByteBuffer, Concat};

mod buffer;
mod byte_buffer;

pub use buffer::Buffer;
pub use byte_buffer::{ByteBuffer, Concat};

pub fn asm(buffer: &mut Buffer<[u8; 20]>, x: u32) {
    buffer.write_u32(x);
}

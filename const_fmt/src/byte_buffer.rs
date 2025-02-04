pub trait Seal {}

pub unsafe trait ByteBuffer: Seal {}

impl<const N: usize> Seal for [u8; N] {}
unsafe impl<const N: usize> ByteBuffer for [u8; N] {}

#[repr(C)]
pub struct Concat<A, B> {
    a: A,
    b: B,
}

impl<A: Seal, B: Seal> Seal for Concat<A, B> {}
unsafe impl<A: ByteBuffer, B: ByteBuffer> ByteBuffer for Concat<A, B> {}

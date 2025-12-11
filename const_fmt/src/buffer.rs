#![allow(clippy::identity_op)]

use core::{mem::MaybeUninit, num::NonZero};

use cfg_if::cfg_if;

use crate::{ByteBuffer, Concat};

#[repr(C)]
pub struct Buffer<B> {
    len: usize,
    buffer: MaybeUninit<B>,
}

macro_rules! write_uint {
    ($ty:ident $writefun:ident) => {
        pub const fn $writefun(&mut self, value: $ty) -> Result<(), BufferWriteFailed> {
            // this imp function exists so that we don't duplicate this logic
            // on every instantiation of Buffer. Instead all instantiations
            // of Buffer will share this same implementation with some small
            // adjustments at the beginning and end
            // this will likely be inlined if there aren't many copies of it
            const fn imp(
                value: NonZero<$ty>,
                remaining_capacity: usize,
                buffer_ptr: *mut u8,
            ) -> Result<usize, BufferWriteFailed> {
                let mut len = value.ilog10() as usize + 1;
                let mut value = value.get();

                if len > remaining_capacity {
                    return Err(BufferWriteFailed);
                }

                let mut ptr = unsafe { buffer_ptr.add(len).cast::<[u8; 4]>() };
                let total_len = len as usize;

                while value >= 10000 {
                    let index = (value % 10000) as usize;

                    unsafe {
                        ptr = ptr.sub(1);
                        ptr.write(LOOKUP_10000.as_ptr().cast::<[u8; 4]>().add(index).read())
                    }

                    value /= 10000;
                    len -= 4;
                }

                // value is guaranteed to be < 10000 here
                unsafe { write_lt_10000_unchecked(buffer_ptr, value as u16, len) }

                Ok(total_len)
            }

            let Some(value) = NonZero::new(value) else {
                return self.push_str("0");
            };

            let ptr = unsafe { self.as_mut_ptr().add(self.len) };
            self.len += tri!(imp(value, self.remaining_capacity(), ptr));
            Ok(())
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub struct BufferWriteFailed;

impl Buffer<[u8; 0]> {
    pub const fn new<const N: usize>() -> Buffer<[u8; N]> {
        Buffer::create()
    }
}

impl<B: ByteBuffer> Buffer<B> {
    const fn create() -> Self {
        Self {
            len: 0,
            buffer: MaybeUninit::uninit(),
        }
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub const fn as_str(&self) -> &str {
        let ptr = self.as_ptr();
        let len = self.len();

        unsafe { core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, len)) }
    }

    pub const fn capacity(&self) -> usize {
        core::mem::size_of::<B>()
    }

    pub const fn len(&self) -> usize {
        let len = self.len;
        unsafe { core::hint::assert_unchecked(len <= self.capacity()) }
        len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub const fn remaining_capacity(&self) -> usize {
        unsafe { self.capacity().unchecked_sub(self.len) }
    }

    const fn as_ptr(&self) -> *const u8 {
        (&raw const self.buffer).cast()
    }

    const fn as_mut_ptr(&mut self) -> *mut u8 {
        (&raw mut self.buffer).cast()
    }

    const unsafe fn push_str_unchecked(&mut self, s: &str) {
        unsafe {
            self.as_mut_ptr()
                .add(self.len)
                .copy_from_nonoverlapping(s.as_ptr(), s.len());
            self.len += s.len();
        }
    }

    pub const fn push_str(&mut self, s: &str) -> Result<(), BufferWriteFailed> {
        if s.len() > self.remaining_capacity() {
            return Err(BufferWriteFailed);
        }

        unsafe { self.push_str_unchecked(s) };

        Ok(())
    }

    pub const fn write_char(&mut self, value: char) -> Result<(), BufferWriteFailed> {
        const unsafe fn imp(ptr: *mut u8, value: char) {
            let mut buf = [0; 4];
            value.encode_utf8(&mut buf);

            unsafe {
                match value.len_utf8() {
                    1 => ptr.write(buf[0]),
                    2 => ptr.cast::<[u8; 2]>().write([buf[0], buf[1]]),
                    3 => ptr.cast::<[u8; 3]>().write([buf[0], buf[1], buf[2]]),
                    4 => ptr.cast::<[u8; 4]>().write(buf),
                    _ => unreachable!(),
                }
            }
        }

        if value.len_utf8() > self.remaining_capacity() {
            return Err(BufferWriteFailed);
        }

        unsafe {
            let ptr = self.as_mut_ptr().add(self.len);
            self.len += value.len_utf8();

            imp(ptr, value);
        }

        Ok(())
    }

    pub const fn write_u8(&mut self, value: u8) -> Result<(), BufferWriteFailed> {
        // u8_ilog10 is taken from Rust stdlib core::num::int_log10 module v1.86.0
        #[inline]
        pub const fn u8_ilog10(val: u8) -> u32 {
            let val = val as u32;

            // For better performance, avoid branches by assembling the solution
            // in the bits above the low 8 bits.

            // Adding c1 to val gives 10 in the top bits for val < 10, 11 for val >= 10
            const C1: u32 = 0b11_00000000 - 10; // 758
            // Adding c2 to val gives 01 in the top bits for val < 100, 10 for val >= 100
            const C2: u32 = 0b10_00000000 - 100; // 412

            // Value of top bits:
            //            +c1  +c2  1&2
            //     0..=9   10   01   00 = 0
            //   10..=99   11   01   01 = 1
            // 100..=255   11   10   10 = 2
            ((val + C1) & (val + C2)) >> 8
        }

        let len = u8_ilog10(value) as usize + 1;

        if len > self.remaining_capacity() {
            return Err(BufferWriteFailed);
        }

        let ptr = unsafe { self.as_mut_ptr().add(self.len) };
        self.len += len;
        unsafe { write_lt_10000_unchecked(ptr, value as u16, len) };

        Ok(())
    }

    write_uint! { u16 write_u16 }
    write_uint! { u32 write_u32 }
    write_uint! { u64 write_u64 }
    write_uint! { u128 write_u128 }

    cfg_if! {
        if #[cfg(target_pointer_width = "16")] {
            pub const fn write_usize(&mut self, value: usize) -> Result<(), BufferWriteFailed> {
                self.write_u16(value as _)
            }
        } else if #[cfg(target_pointer_width = "32")] {
            pub const fn write_usize(&mut self, value: usize) -> Result<(), BufferWriteFailed> {
                self.write_u32(value as _)
            }
        } else if #[cfg(target_pointer_width = "64")] {
            pub const fn write_usize(&mut self, value: usize) -> Result<(), BufferWriteFailed> {
                self.write_u64(value as _)
            }
        } else {
            write_uint! { usize write_usize }
        }
    }

    const fn push_neg(&mut self) -> Result<(), BufferWriteFailed> {
        self.push_str("-")
    }

    pub const fn write_i8(&mut self, value: i8) -> Result<(), BufferWriteFailed> {
        if value < 0 {
            tri!(self.push_neg())
        }

        self.write_u8(value.unsigned_abs())
    }

    pub const fn write_i16(&mut self, value: i16) -> Result<(), BufferWriteFailed> {
        if value < 0 {
            tri!(self.push_neg())
        }

        self.write_u16(value.unsigned_abs())
    }

    pub const fn write_i32(&mut self, value: i32) -> Result<(), BufferWriteFailed> {
        if value < 0 {
            tri!(self.push_neg())
        }

        self.write_u32(value.unsigned_abs())
    }

    pub const fn write_i64(&mut self, value: i64) -> Result<(), BufferWriteFailed> {
        if value < 0 {
            tri!(self.push_neg())
        }

        self.write_u64(value.unsigned_abs())
    }

    pub const fn write_i128(&mut self, value: i128) -> Result<(), BufferWriteFailed> {
        if value < 0 {
            tri!(self.push_neg())
        }

        self.write_u128(value.unsigned_abs())
    }

    pub const fn write_isize(&mut self, value: isize) -> Result<(), BufferWriteFailed> {
        if value < 0 {
            tri!(self.push_neg())
        }

        self.write_usize(value.unsigned_abs())
    }

    pub const fn append<A: ByteBuffer>(&self, other: &Buffer<A>) -> Buffer<Concat<B, A>> {
        let mut out = Buffer::create();
        unsafe { out.push_str_unchecked(self.as_str()) };
        unsafe { out.push_str_unchecked(other.as_str()) };
        out
    }
}

const unsafe fn write_lt_10000_unchecked(ptr: *mut u8, value: u16, len: usize) {
    unsafe {
        // point to the current end of the buffer
        let lookup = LOOKUP_10000
            .as_ptr()
            .cast::<[u8; 4]>()
            .add(value as usize)
            .read();

        // always write all values since it's faster than checking
        // if the byte should be written
        ptr.write(lookup[0]);
        // increment pointer if there are no more digits to skip
        let ptr = ptr.add((len >= 4) as usize);
        ptr.write(lookup[1]);
        // increment pointer if there are no more digits to skip
        let ptr = ptr.add((len >= 3) as usize);
        ptr.write(lookup[2]);
        // increment pointer if there are no more digits to skip
        let ptr = ptr.add((len >= 2) as usize);
        ptr.write(lookup[3]);
    }
}

static LOOKUP_10000: [u8; 40000] = {
    let mut lookup = [0; 40000];

    let mut i = 0;

    while i < 10000 {
        let v = i;
        lookup[4 * i + 3] = (v % 10) as u8 + b'0';
        lookup[4 * i + 2] = ((v / 10) % 10) as u8 + b'0';
        lookup[4 * i + 1] = ((v / 100) % 10) as u8 + b'0';
        lookup[4 * i + 0] = (v / 1000) as u8 + b'0';

        i += 1;
    }

    lookup
};

#[test]
fn test_all_u8() {
    use std::fmt::Write;

    let mut s = String::new();
    for i in 0..=u8::MAX {
        let mut buffer = Buffer::<[u8; 3]>::create();
        let _ = buffer.write_u8(i);
        s.clear();
        let _ = write!(s, "{i}");
        assert_eq!(buffer.as_str(), s);
    }
}

#[test]
fn test_all_u16() {
    use std::fmt::Write;

    let mut s = String::new();
    for i in 0..=u16::MAX {
        let mut buffer = Buffer::<[u8; 5]>::create();
        let _ = buffer.write_u16(i);
        s.clear();
        let _ = write!(s, "{i}");
        assert_eq!(buffer.as_str(), s);
    }
}

#[test]
#[ignore = "slow"]
fn test_all_u32() {
    use std::fmt::Write;

    let mut s = String::new();
    for i in 0..=u32::MAX {
        let mut buffer = Buffer::<[u8; 9]>::create();
        let _ = buffer.write_u32(i);
        s.clear();
        let _ = write!(s, "{i}");
        assert_eq!(buffer.as_str(), s);
    }
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(4)]
fn prove_u8() {
    let x: u8 = kani::any();

    let mut buffer = Buffer::<[u8; 20]>::create();
    buffer.write_u8(x);

    assert_eq!(buffer.as_str().parse::<u8>(), Ok(x));
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(6)]
fn prove_u16() {
    let x: u16 = kani::any();

    let mut buffer = Buffer::<[u8; 20]>::create();
    buffer.write_u16(x);

    assert_eq!(buffer.as_str().parse::<u16>(), Ok(x));
}

#[cfg(kani)]
#[kani::proof]
#[kani::unwind(10)]
fn prove_u32() {
    let x: u32 = kani::any();

    let mut buffer = Buffer::<[u8; 20]>::create();
    buffer.write_u32(x);

    let mut buf = [0u8; 20];

    write!(&mut buf[..], "{x}");

    assert_eq!(buffer.as_str().as_bytes());

    assert_eq!(buffer.as_str().parse::<u32>(), Ok(x));
}

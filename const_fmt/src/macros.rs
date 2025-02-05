macro_rules! tri {
    ($value:expr) => {
        match $value {
            Ok(x) => x,
            Err(x) => return Err(x),
        }
    };
}

use core::convert::Infallible;
use core::marker::PhantomData;

use crate::{Buffer, BufferWriteFailed, ByteBuffer};

pub const fn get_writer<T: Writer>(_: impl FnOnce(Infallible) -> T + Copy) -> T {
    Writer::INIT
}

pub trait Writer {
    const INIT: Self;
}

pub trait ConstFormat {
    type Writer: Writer;
}

pub trait Selection {
    type Writer: Writer;

    fn select(&self, inf: Infallible) -> Self::Writer {
        match inf {}
    }
}

pub struct Selector<'a, T>(pub &'a T);

impl<T: ConstFormat> Selection for &Selector<'_, T> {
    type Writer = T::Writer;
}

impl<T> Selection for Selector<'_, T> {
    type Writer = ConstFormatNotImplemented<T>;
}

pub struct ConstFormatNotImplemented<T>(PhantomData<T>);

impl<T> Writer for ConstFormatNotImplemented<T> {
    const INIT: Self = Self(PhantomData);
}

#[doc(hidden)]
#[macro_export]
macro_rules! get_writer {
    ($val:ident) => {{
        use $crate::macros::Selection;

        get_writer(|inf| (&&&&$crate::macros::Selector(&$val)).select(inf))
    }};
}

pub struct StdWriter<T>(PhantomData<T>);

macro_rules! int {
    ($int:ident $func:ident) => {
        impl ConstFormat for $int {
            type Writer = StdWriter<Self>;
        }

        impl Writer for StdWriter<$int> {
            const INIT: Self = Self(PhantomData);
        }

        impl StdWriter<$int> {
            pub fn display<B: ByteBuffer>(
                self,
                value: &$int,
                buffer: &mut Buffer<B>,
            ) -> Result<(), BufferWriteFailed> {
                buffer.$func(*value)
            }
        }
    };
}

int!(u8 write_u8);
int!(u16 write_u16);
int!(u32 write_u32);
int!(u64 write_u64);
int!(u128 write_u128);
int!(usize write_usize);

int!(i8 write_i8);
int!(i16 write_i16);
int!(i32 write_i32);
int!(i64 write_i64);
int!(i128 write_i128);
int!(isize write_isize);

int!(char write_char);

impl ConstFormat for &str {
    type Writer = StdWriter<Self>;
}

impl Writer for StdWriter<&str> {
    const INIT: Self = Self(PhantomData);
}

impl StdWriter<&str> {
    pub fn display<B: ByteBuffer>(
        self,
        value: &str,
        buffer: &mut Buffer<B>,
    ) -> Result<(), BufferWriteFailed> {
        buffer.push_str(value)
    }
}

#[test]
fn test() {
    let x = 0u8;

    let mut buffer = Buffer::new::<20>();
    get_writer!(x).display(&x, &mut buffer).unwrap();

    assert_eq!(buffer.as_str(), "0");
}

use std::{fmt::Display, str::from_utf8};

use crate::buffer;

/// Private level API to get a chunk from buffer
///
/// ! DANGER
///
/// In release, the [`TAIL`] wraps around back to the start of the buffer when
/// there isn't sufficient space left inside of [`BUFFER`]. If this happens,
/// the buffer might overwrite previous data with anything.
///
/// In debug, the method panics when we reach the end of the buffer
#[doc(hidden)]
pub fn get_chunk_as_mut(chunk_size: usize) -> &'static mut [u8] {
    unsafe {
        buffer::BUFFER
            .get_mut()
            .expect("BUFFER not init, did you run init?")
            .get_chunk_as_mut(chunk_size)
    }
}

/// Allows specification of a custom way to serialize the Struct.
/// Additionally, this stores the contents serialized onto a static buffer, which does
/// not require allocation and could speed things up.
pub trait Serialize {
    fn encode(&self, write_buf: &'static mut [u8]) -> Store;
    fn buffer_size_required(&self) -> usize;
}

/// Function pointer which decodes a byte buffer back into `String` representation
pub type DecodeFn = fn(&[u8]) -> String;

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        /// Contains the decode function required to decode `buffer` back into a `String`
        /// representation.
        ///
        /// Store **SHOULD NOT** implement `Clone`, in debug, otherwise, there might be
        /// double updating of the tail of the buffer in `Drop` causing the tail to overrun
        /// the head, even though it actually did not
        pub struct Store {
            decode_fn: DecodeFn,
            buffer: &'static [u8],
        }
    } else {
        /// Contains the decode function required to decode `buffer` back into a `String`
        /// representation.
        #[derive(Clone)]
        pub struct Store {
            decode_fn: DecodeFn,
            buffer: &'static [u8],
        }
    }
}

impl Store {
    pub fn new(decode_fn: DecodeFn, buffer: &'static [u8]) -> Store {
        Store { decode_fn, buffer }
    }

    pub fn as_string(&self) -> String {
        (self.decode_fn)(self.buffer)
    }
}

impl Display for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

cfg_if::cfg_if! {
    if #[cfg(debug_assertions)] {
        impl Drop for Store {
            /// Increments the buffer read_idx by length of buffer acquired, since we are
            /// always guaranteed to read in-order from front to back, this is safe,
            /// unless Store gets cloned, this only happens in Debug
            fn drop(&mut self) {
                unsafe {
                    buffer::BUFFER
                        .get_mut()
                        .expect("Unable to get BUFFER, has it been init?")
                        .dealloc(self.buffer.len())
                }
            }
        }
    }
}

macro_rules! gen_encode_decode {
    ($name:ident, $primitive:ty) => {
        pub fn $name(val: $primitive, write_buf: &'static mut [u8]) -> Store {
            assert!(std::mem::size_of::<$primitive>() == write_buf.len());

            fn decode(read_buf: &[u8]) -> String {
                let x = <$primitive>::from_le_bytes(read_buf.try_into().unwrap());
                format!("{}", x)
            }

            let size = std::mem::size_of::<$primitive>();
            let (x, _) = write_buf.split_at_mut(size);
            x.copy_from_slice(&val.to_le_bytes());
            Store::new(decode, x)
        }
    };
}

gen_encode_decode!(encode_i32, i32);
gen_encode_decode!(encode_i64, i64);
gen_encode_decode!(encode_f32, f32);
gen_encode_decode!(encode_f64, f64);
gen_encode_decode!(encode_usize, usize);

pub fn encode_str(val: &str, write_buf: &'static mut [u8]) -> Store {
    assert!(val.len() == write_buf.len());
    fn decode(read_buf: &[u8]) -> String {
        let x = from_utf8(read_buf).unwrap();
        x.to_string()
    }
    write_buf.copy_from_slice(val.as_bytes());
    Store::new(decode, write_buf)
}

/// Eager evaluation of String!
pub fn encode_debug<T: std::fmt::Debug>(val: T, write_buf: &'static mut [u8]) -> Store {
    let val_string = format!("{:?}", val);
    assert!(val_string.len() == write_buf.len());

    fn decode(read_buf: &[u8]) -> String {
        let x = from_utf8(read_buf).unwrap();
        x.to_string()
    }

    write_buf.copy_from_slice(val_string.as_bytes());
    Store::new(decode, write_buf)
}

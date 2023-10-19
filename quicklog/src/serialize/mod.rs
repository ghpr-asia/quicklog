use std::{fmt::Display, str::from_utf8};

pub mod buffer;

/// Allows specification of a custom way to serialize the Struct.
/// Additionally, this stores the contents serialized into a buffer, which does
/// not require allocation and could speed things up.
pub trait Serialize {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> Store<'buf>;
    fn buffer_size_required(&self) -> usize;
}

/// Function pointer which decodes a byte buffer back into `String` representation
pub type DecodeFn = fn(&[u8]) -> String;

/// Contains the decode function required to decode `buffer` back into a `String`
/// representation.
#[derive(Clone)]
pub struct Store<'buf> {
    decode_fn: DecodeFn,
    buffer: &'buf [u8],
}

impl Store<'_> {
    pub fn new(decode_fn: DecodeFn, buffer: &[u8]) -> Store {
        Store { decode_fn, buffer }
    }

    pub fn as_string(&self) -> String {
        (self.decode_fn)(self.buffer)
    }
}

impl Display for Store<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

macro_rules! gen_encode_decode {
    ($name:ident, $primitive:ty) => {
        pub fn $name(val: $primitive, write_buf: &mut [u8]) -> Store {
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

pub fn encode_str<'buf>(val: &str, write_buf: &'buf mut [u8]) -> Store<'buf> {
    assert!(val.len() == write_buf.len());
    fn decode(read_buf: &[u8]) -> String {
        let x = from_utf8(read_buf).unwrap();
        x.to_string()
    }
    write_buf.copy_from_slice(val.as_bytes());
    Store::new(decode, write_buf)
}

/// Eager evaluation into a String for debug structs
pub fn encode_debug<T: std::fmt::Debug>(val: T, write_buf: &mut [u8]) -> Store {
    let val_string = format!("{:?}", val);
    assert!(val_string.len() == write_buf.len());

    fn decode(read_buf: &[u8]) -> String {
        let x = from_utf8(read_buf).unwrap();
        x.to_string()
    }

    write_buf.copy_from_slice(val_string.as_bytes());
    Store::new(decode, write_buf)
}

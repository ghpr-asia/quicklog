use std::{fmt::Display, str::from_utf8};

pub mod buffer;

/// Allows specification of a custom way to serialize the Struct.
/// Additionally, this stores the contents serialized into a buffer, which does
/// not require allocation and could speed things up.
pub trait Serialize {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> Store<'buf>;
    fn decode(read_buf: &[u8]) -> (String, &[u8]);
    fn buffer_size_required(&self) -> usize;
}

/// Function pointer which decodes a byte buffer back into `String` representation
pub type DecodeFn = fn(&[u8]) -> (String, &[u8]);

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
        let (s, _) = (self.decode_fn)(self.buffer);
        s
    }
}

impl Display for Store<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

macro_rules! gen_serialize {
    ($primitive:ty) => {
        impl Serialize for $primitive {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> Store<'buf> {
                assert!(std::mem::size_of::<$primitive>() == write_buf.len());

                let size = std::mem::size_of::<$primitive>();
                let (x, _) = write_buf.split_at_mut(size);
                x.copy_from_slice(&self.to_le_bytes());
                Store::new(Self::decode, x)
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let (chunk, rest) = read_buf.split_at(std::mem::size_of::<$primitive>());
                let x = <$primitive>::from_le_bytes(chunk.try_into().unwrap());

                (format!("{}", x), rest)
            }

            fn buffer_size_required(&self) -> usize {
                std::mem::size_of::<$primitive>()
            }
        }
    };
}

gen_serialize!(i32);
gen_serialize!(i64);
gen_serialize!(isize);
gen_serialize!(f32);
gen_serialize!(f64);
gen_serialize!(u32);
gen_serialize!(u64);
gen_serialize!(usize);

impl Serialize for &str {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> Store<'buf> {
        assert!(self.len() == write_buf.len());
        write_buf.copy_from_slice(self.as_bytes());
        Store::new(Self::decode, write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let x = from_utf8(read_buf).unwrap();
        (x.to_string(), &[])
    }

    fn buffer_size_required(&self) -> usize {
        self.len()
    }
}

/// Eager evaluation into a String for debug structs
pub fn encode_debug<T: std::fmt::Debug>(val: T, write_buf: &mut [u8]) -> Store {
    let val_string = format!("{:?}", val);
    // TODO: change back to strict equality when Serialize implemented, to use
    // `buffer_size_required`
    assert!(val_string.len() <= write_buf.len());

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let x = from_utf8(read_buf).unwrap();
        (x.to_string(), &[])
    }

    let (chunk, _) = write_buf.split_at_mut(val_string.len());
    chunk.copy_from_slice(val_string.as_bytes());
    Store::new(decode, chunk)
}

#[cfg(test)]
mod tests {
    use crate::serialize::encode_debug;

    use super::Serialize;

    macro_rules! assert_primitive_encode_decode {
        ($primitive:ty, $val:expr) => {{
            const BUF_SIZE: usize = std::mem::size_of::<$primitive>();
            let mut buf = [0u8; BUF_SIZE];

            let x: $primitive = $val;
            let x_store = x.encode(&mut buf);
            assert_eq!(format!("{}", x), format!("{}", x_store));
        }};
    }

    #[test]
    fn serialize_primitives() {
        assert_primitive_encode_decode!(i32, -1);
        assert_primitive_encode_decode!(i64, -123);
        assert_primitive_encode_decode!(isize, -1234);
        assert_primitive_encode_decode!(f32, 1.23);
        assert_primitive_encode_decode!(f64, 1.23456);
        assert_primitive_encode_decode!(u32, 999);
        assert_primitive_encode_decode!(u64, 9999);
        assert_primitive_encode_decode!(usize, 99999);
    }

    #[test]
    fn serialize_multiple_primitives() {
        let mut buf = [0; 128];
        let a: i32 = -1;
        let b: u32 = 999;
        let c: usize = 100000;

        let (a_chunk, chunk) = buf.split_at_mut(a.buffer_size_required());
        let (b_chunk, chunk) = chunk.split_at_mut(b.buffer_size_required());
        let (c_chunk, _) = chunk.split_at_mut(c.buffer_size_required());

        let a_store = a.encode(a_chunk);
        let b_store = b.encode(b_chunk);
        let c_store = c.encode(c_chunk);

        assert_eq!(
            format!("{} {} {}", a, b, c),
            format!("{} {} {}", a_store, b_store, c_store)
        )
    }

    #[test]
    fn serialize_str() {
        let mut buf = [0; 128];
        let s = "hello world";
        let (s_chunk, _) = buf.split_at_mut(s.buffer_size_required());
        let store = s.encode(s_chunk);

        assert_eq!(s, format!("{}", store).as_str())
    }

    #[test]
    fn serialize_debug() {
        #[derive(Debug)]
        #[allow(unused)]
        struct DebugStruct {
            s: &'static str,
        }

        let mut buf = [0; 128];
        let s = DebugStruct { s: "Hello World" };
        let store = encode_debug(&s, &mut buf);

        assert_eq!(format!("{:?}", s), format!("{}", store))
    }
}

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

macro_rules! gen_serialize {
    ($primitive:ty) => {
        impl Serialize for $primitive {
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> Store<'buf> {
                assert!(std::mem::size_of::<$primitive>() == write_buf.len());
                fn decode(read_buf: &[u8]) -> String {
                    let x = <$primitive>::from_le_bytes(read_buf.try_into().unwrap());
                    format!("{}", x)
                }

                let size = std::mem::size_of::<$primitive>();
                let (x, _) = write_buf.split_at_mut(size);
                x.copy_from_slice(&self.to_le_bytes());
                Store::new(decode, &*x)
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

#[cfg(test)]
mod tests {
    use super::Serialize;

    macro_rules! assert_primitive_encode_decode {
        ($primitive:ty, $val:expr) => {{
            const BUF_SIZE: usize = std::mem::size_of::<$primitive>();
            let mut buf = [0u8; BUF_SIZE];

            let x: $primitive = $val;
            let x_store = x.encode(&mut buf);
            assert_eq!(format!("{}", x), (x_store.decode_fn)(&buf));
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

        let a_size = std::mem::size_of::<i32>();
        let b_size = std::mem::size_of::<u32>();
        let c_size = std::mem::size_of::<usize>();

        let (a_chunk, chunk) = buf.split_at_mut(a_size);
        let (b_chunk, chunk) = chunk.split_at_mut(b_size);
        let (c_chunk, _) = chunk.split_at_mut(c_size);

        let a_store = a.encode(a_chunk);
        let b_store = b.encode(b_chunk);
        let c_store = c.encode(c_chunk);

        let a_str = (a_store.decode_fn)(a_store.buffer);
        let b_str = (b_store.decode_fn)(b_store.buffer);
        let c_str = (c_store.decode_fn)(c_store.buffer);

        assert_eq!(
            format!("{} {} {}", a, b, c),
            format!("{} {} {}", a_str, b_str, c_str)
        )
    }
}

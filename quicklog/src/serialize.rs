use std::{fmt::Display, str::from_utf8};

// TODO: Allow this to be specified in some env var
const MAX_CAPACITY: usize = 1_000_000;

/// Byte buffer used to store data serialized onto it
static mut BUFFER: [u8; MAX_CAPACITY] = [0_u8; MAX_CAPACITY];

/// Tail end of [`BUFFER`]
static mut TAIL: Option<&'static mut [u8]> = None;

/// Private level API to get a chunk from buffer
///
/// ! DANGER
///
/// TODO: Find some way to make this safer, perhaps using `ringbuf` or `bytes`
/// to manage [`BUFFER`] instead of hand writing this.
///
/// The [`TAIL`] wraps around back to the start of the buffer when there isn't
/// sufficient space left inside of [`BUFFER`]. If this happens, the buffer
/// might overwrite previous data with anything.
#[doc(hidden)]
pub fn get_chunk_as_mut(chunk_size: usize) -> &'static mut [u8] {
    unsafe {
        let buf = match TAIL.as_deref_mut() {
            Some(tail) if tail.len() < chunk_size => &mut BUFFER,
            Some(tail) => tail,
            None => &mut BUFFER,
        };

        let (head, new_tail) = buf.split_at_mut(chunk_size);
        TAIL = Some(new_tail);

        head
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

/// Contains the decode function required to decode `buffer` back into a `String`
/// representation.
///
/// implements `Clone` and `Display`
#[derive(Clone)]
pub struct Store {
    decode_fn: DecodeFn,
    buffer: &'static [u8],
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

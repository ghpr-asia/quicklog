use std::{fmt::Display, mem::size_of, str::from_utf8};

/// Allows specification of a custom way to serialize the Struct.
///
/// This is the key trait to implement to improve logging performance. While
/// `Debug` and `Display` usages are eagerly formatted on the hot path,
/// `Serialize` usages copy the minimal required bytes to a separate buffer,
/// and then allow for formatting when flushing elsewhere. Consider ensuring
/// that all logging arguments implement `Serialize` for best performance.
///
/// Furthermore, you would usually not be required to implement `Serialize` by
/// hand for most types. The option that would work for most use cases would be
/// [deriving `Serialize`](crate::Serialize), similar to how `Debug` is
/// derived on user-defined types. Although, do note that all fields on the user
/// struct must also derive/implement `Serialize` (similar to `Debug` again).
///
/// For instance, this would work since all fields have a `Serialize`
/// implementation:
/// ```
/// use quicklog::Serialize;
///
/// #[derive(Serialize)]
/// struct SerializeStruct {
///     a: usize,
///     b: i32,
///     c: &'static str,
/// }
/// ```
///
/// But a field with a type that does not implement `Serialize` will fail to compile:
/// ```compile_fail
/// use quicklog::Serialize;
///
/// struct NoSerializeStruct {
///     a: &'static str,
///     b: &'static str,
/// }
///
/// #[derive(Serialize)]
/// struct SerializeStruct {
///     a: usize,
///     b: i32,
///     // doesn't implement `Serialize`!
///     c: NoSerializeStruct,
/// }
/// ```
pub trait Serialize {
    /// Describes how to encode the implementing type into a byte buffer.
    /// Assumes that `write_buf` has enough capacity to encode argument in.
    ///
    /// Returns a [Store](crate::serialize::Store) and the remainder of `write_buf`
    /// passed in that was not written to.
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]);
    /// Describes how to decode the implementing type from a byte buffer.
    ///
    /// Returns a formatted String after parsing the byte buffer, as well as
    /// the remainder of `read_buf` pass in that was not read.
    fn decode(read_buf: &[u8]) -> (String, &[u8]);
    /// The number of bytes required to `encode` the type into a byte buffer.
    fn buffer_size_required(&self) -> usize;
}

/// **WARNING: this is part of the public API and is primarily to aid in macro
/// codegen.**
///
/// Helper trait for splitting the output of decoding for collections of types
/// implementing [`Serialize`].
#[doc(hidden)]
pub trait SerializeTpl: Serialize {
    /// Collects the outputs of [`Serialize::decode`] in an output buffer.
    fn decode_each<'buf>(read_buf: &'buf [u8], out: &mut Vec<String>) -> &'buf [u8];
}

/// Function pointer which decodes a byte buffer back into `String` representation
pub type DecodeFn = fn(&[u8]) -> (String, &[u8]);

/// Function pointer which decodes a byte buffer and stores the results in an
/// output buffer.
pub type DecodeEachFn = for<'buf> fn(&'buf [u8], &mut Vec<String>) -> &'buf [u8];

/// Number of bytes it takes to store the size of a type.
pub const SIZE_LENGTH: usize = size_of::<usize>();

/// Contains the decode function required to decode `buffer` back into a `String`
/// representation.
#[derive(Clone)]
pub struct Store<'buf> {
    pub(crate) decode_fn: DecodeFn,
    pub(crate) buffer: &'buf [u8],
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
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
                let size = size_of::<$primitive>();
                let (x, rest) = write_buf.split_at_mut(size);
                x.copy_from_slice(&self.to_le_bytes());

                (Store::new(Self::decode, x), rest)
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let (chunk, rest) = read_buf.split_at(size_of::<$primitive>());
                let x = <$primitive>::from_le_bytes(chunk.try_into().unwrap());

                (format!("{}", x), rest)
            }

            fn buffer_size_required(&self) -> usize {
                size_of::<$primitive>()
            }
        }
    };
}

gen_serialize!(i8);
gen_serialize!(i16);
gen_serialize!(i32);
gen_serialize!(i64);
gen_serialize!(i128);
gen_serialize!(isize);

gen_serialize!(u8);
gen_serialize!(u16);
gen_serialize!(u32);
gen_serialize!(u64);
gen_serialize!(u128);
gen_serialize!(usize);

gen_serialize!(f32);
gen_serialize!(f64);

impl Serialize for bool {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        let (chunk, rest) = write_buf.split_at_mut(size_of::<bool>());
        chunk.copy_from_slice(&(*self as u8).to_le_bytes());

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = read_buf.split_at(size_of::<bool>());
        let x = u8::from_le_bytes(chunk.try_into().unwrap()) != 0;

        (format!("{}", x), rest)
    }

    fn buffer_size_required(&self) -> usize {
        size_of::<bool>()
    }
}

impl Serialize for char {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        let (chunk, rest) = write_buf.split_at_mut(size_of::<char>());
        chunk.copy_from_slice(&(*self as u32).to_le_bytes());

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = read_buf.split_at(size_of::<char>());
        // Assuming that we encoded this char
        let c = char::from_u32(u32::from_le_bytes(chunk.try_into().unwrap())).unwrap();

        (format!("{}", c), rest)
    }

    fn buffer_size_required(&self) -> usize {
        size_of::<char>()
    }
}

impl Serialize for &str {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        let str_len = self.len();
        let (chunk, rest) = write_buf.split_at_mut(str_len + SIZE_LENGTH);
        let (len_chunk, str_chunk) = chunk.split_at_mut(SIZE_LENGTH);

        len_chunk.copy_from_slice(&str_len.to_le_bytes());
        str_chunk.copy_from_slice(self.as_bytes());

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (len_chunk, chunk) = read_buf.split_at(SIZE_LENGTH);
        let str_len = usize::from_le_bytes(len_chunk.try_into().unwrap());

        let (str_chunk, rest) = chunk.split_at(str_len);
        let s = from_utf8(str_chunk).unwrap();

        (s.to_string(), rest)
    }

    fn buffer_size_required(&self) -> usize {
        SIZE_LENGTH + self.len()
    }
}

impl Serialize for String {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        self.as_str().encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <&str as Serialize>::decode(read_buf)
    }

    fn buffer_size_required(&self) -> usize {
        self.as_str().buffer_size_required()
    }
}

impl<T: Serialize> Serialize for &T {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        (*self).encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <T as Serialize>::decode(read_buf)
    }

    fn buffer_size_required(&self) -> usize {
        (*self).buffer_size_required()
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
        let (len_chunk, mut vec_chunk) = chunk.split_at_mut(SIZE_LENGTH);
        len_chunk.copy_from_slice(&self.len().to_le_bytes());

        for i in self {
            (_, vec_chunk) = i.encode(vec_chunk);
        }

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (len_chunk, mut chunk) = read_buf.split_at(SIZE_LENGTH);
        let vec_len = usize::from_le_bytes(len_chunk.try_into().unwrap());

        let mut vec = Vec::with_capacity(vec_len);
        let mut decoded;
        for _ in 0..vec_len {
            // TODO(speed): very slow! should revisit whether really want `decode` to return
            // String.
            (decoded, chunk) = T::decode(chunk);
            vec.push(decoded)
        }

        (format!("{:?}", vec), chunk)
    }

    fn buffer_size_required(&self) -> usize {
        self.get(0).map(|a| a.buffer_size_required()).unwrap_or(0) * self.len() + SIZE_LENGTH
    }
}

/// Generates a format string with normal format specifiers for each value
/// passed in. Intended for limited dynamic construction of format strings.
///
/// # Examples
///
/// ```ignore
/// let x = repeat_fmt!(1, 3.15, "hello world");
/// assert_eq!(x, "{}, {}, {}");
/// ```
#[doc(hidden)]
macro_rules! repeat_fmt {
    (@ ( $($acc:tt)* )) => {
        stringify!($($acc),*)
    };
    (@ ( $($acc:tt)* ) $arg:expr) => {
        repeat_fmt!(@ ( $($acc)* {} ))
    };
    (@ ( $($acc:tt)* ) $arg:expr, $($rest:expr),*) => {
        repeat_fmt!(@ ( $($acc)* {} ) $($rest),*)
    };
    ($($arg:tt),*) => {
        repeat_fmt!(@ () $($arg),*)
    };
}

macro_rules! tuple_serialize {
    ($($name:ident)+) => {
        impl<$($name: Serialize),*> Serialize for ($($name,)*) {
            #[allow(non_snake_case)]
            #[allow(unused)]
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
                let ($(ref $name,)*) = *self;
                let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
                let (_, mut tail) = chunk.split_at_mut(0);
                $( (_, tail) = $name.encode(tail); )*

                (Store::new(Self::decode, chunk), rest)
            }

            #[allow(non_snake_case)]
            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                $(let (ref $name, read_buf) = <$name as Serialize>::decode(read_buf);)*
                (format!(concat!("(", repeat_fmt!($($name),*), ")"), $($name),*), read_buf)
            }

            #[allow(non_snake_case)]
            fn buffer_size_required(&self) -> usize {
                let ($(ref $name,)*) = *self;
                let mut size = 0;
                $( size += $name.buffer_size_required(); )*
                size
            }
        }
    };
}

tuple_serialize!(A);
tuple_serialize!(A B);
tuple_serialize!(A B C);
tuple_serialize!(A B C D);
tuple_serialize!(A B C D E);
tuple_serialize!(A B C D E F);
tuple_serialize!(A B C D E F G);
tuple_serialize!(A B C D E F G H);
tuple_serialize!(A B C D E F G H I);
tuple_serialize!(A B C D E F G H I J);
tuple_serialize!(A B C D E F G H I J K);
tuple_serialize!(A B C D E F G H I J K L);

#[doc(hidden)]
macro_rules! tuple_serialize_each {
    ($($name:ident)+) => {
        impl<$($name: Serialize),*> SerializeTpl for ($($name,)*) {
            #[allow(non_snake_case)]
            fn decode_each<'buf>(read_buf: &'buf [u8], out: &mut Vec<String>) -> &'buf [u8] {
                $(
                    let ($name, read_buf) = <$name as Serialize>::decode(read_buf);
                    out.push($name);
                 )*

                read_buf
            }
        }
    };
}

tuple_serialize_each!(A);
tuple_serialize_each!(A B);
tuple_serialize_each!(A B C);
tuple_serialize_each!(A B C D);
tuple_serialize_each!(A B C D E);
tuple_serialize_each!(A B C D E F);
tuple_serialize_each!(A B C D E F G);
tuple_serialize_each!(A B C D E F G H);
tuple_serialize_each!(A B C D E F G H I);
tuple_serialize_each!(A B C D E F G H I J);
tuple_serialize_each!(A B C D E F G H I J K);
tuple_serialize_each!(A B C D E F G H I J K L);

/// Eager evaluation into a String for debug structs
pub fn encode_debug<T: std::fmt::Debug>(val: T, write_buf: &mut [u8]) -> (Store, &mut [u8]) {
    let val_string = format!("{:?}", val);
    let str_len = val_string.len();

    let (chunk, rest) = write_buf.split_at_mut(str_len + SIZE_LENGTH);
    let (len_chunk, str_chunk) = chunk.split_at_mut(SIZE_LENGTH);
    len_chunk.copy_from_slice(&str_len.to_le_bytes());
    str_chunk.copy_from_slice(val_string.as_bytes());

    (Store::new(<&str as Serialize>::decode, chunk), rest)
}

#[cfg(test)]
mod tests {
    use crate as quicklog;
    use crate::{
        serialize::{encode_debug, Serialize},
        Serialize,
    };
    use std::mem::size_of;

    macro_rules! assert_primitive_encode_decode {
        ($primitive:ty, $val:expr) => {{
            const BUF_SIZE: usize = size_of::<$primitive>();
            let mut buf = [0u8; BUF_SIZE];

            let x: $primitive = $val;
            let (x_store, _) = x.encode(&mut buf);
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

        let (a_store, chunk) = a.encode(&mut buf);
        let (b_store, chunk) = b.encode(chunk);
        let (c_store, _) = c.encode(chunk);

        assert_eq!(
            format!("{} {} {}", a, b, c),
            format!("{} {} {}", a_store, b_store, c_store)
        )
    }

    #[test]
    fn serialize_str() {
        let mut buf = [0; 128];
        let s = "hello world";
        let (store, _) = s.encode(&mut buf);

        assert_eq!(s, format!("{}", store).as_str())
    }

    #[test]
    fn serialize_string() {
        let mut buf = [0; 128];
        let s = "hello world".to_string();
        let (store, _) = s.encode(&mut buf);

        assert_eq!(s, format!("{}", store))
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
        let (store, _) = encode_debug(&s, &mut buf);

        assert_eq!(format!("{:?}", s), format!("{}", store))
    }

    #[test]
    fn serialize_bool_char() {
        let a = true;
        let b = 'b';
        let c = 'ß';
        let mut buf = [0; 128];
        let (a_store, rest) = a.encode(&mut buf);
        let (b_store, rest) = b.encode(rest);
        let (c_store, _) = c.encode(rest);

        assert_eq!(format!("{}", a), format!("{}", a_store));
        assert_eq!(format!("{}", b), format!("{}", b_store));
        assert_eq!(format!("{}", c), format!("{}", c_store));
    }

    #[test]
    fn serialize_tuple() {
        let a = "hello world".to_string();
        let b: usize = 100000;
        let c: i32 = -999;

        #[derive(Serialize)]
        struct SerializeStruct {
            d: usize,
            e: String,
        }
        let d = 1;
        let e = "some struct".to_string();
        let s = SerializeStruct { d: 1, e: e.clone() };

        let mut buf = [0; 256];
        let (store, _) = (&a, b, c, s).encode(&mut buf);
        assert_eq!(
            format!(
                "({}, {}, {}, SerializeStruct {{ d: {}, e: {} }})",
                a, b, c, d, e
            ),
            format!("{}", store)
        );
    }

    #[test]
    fn serialize_vec() {
        let a = vec!["hello world", "bye world"];
        let mut buf = [0; 256];
        let (a_store, _) = a.encode(&mut buf);

        assert_eq!(format!("{:?}", a), format!("{}", a_store));
    }
}

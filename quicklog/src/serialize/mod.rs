use std::{
    borrow::Cow,
    mem::{size_of, MaybeUninit},
    str::from_utf8,
};

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
    /// Returns the remainder of `write_buf` passed in that was not written to.
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8];
    /// Describes how to decode the implementing type from a byte buffer.
    ///
    /// Returns a formatted String after parsing the byte buffer, as well as
    /// the remainder of `read_buf` pass in that was not read.
    fn decode(read_buf: &[u8]) -> (String, &[u8]);
    /// The number of bytes required to `encode` the type into a byte buffer.
    #[inline]
    fn buffer_size_required(&self) -> usize
    where
        Self: Sized,
    {
        size_of::<Self>()
    }
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
pub(crate) const SIZE_LENGTH: usize = size_of::<usize>();

macro_rules! gen_serialize {
    ($primitive:ty) => {
        impl Serialize for $primitive {
            #[inline]
            fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
                let buf_ptr = write_buf.as_mut_ptr();
                let n = size_of::<$primitive>();
                let remaining = write_buf.len() - n;

                // SAFETY: We requested the exact amount required from the queue, so
                // should not run out of space here.
                unsafe {
                    buf_ptr.copy_from_nonoverlapping(self.to_le_bytes().as_ptr(), n);
                    std::slice::from_raw_parts_mut(buf_ptr.add(n).cast(), remaining)
                }
            }

            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                let (chunk, rest) = read_buf.split_at(size_of::<$primitive>());
                let x = <$primitive>::from_le_bytes(chunk.try_into().unwrap());

                (format!("{}", x), rest)
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
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let buf_ptr = write_buf.as_mut_ptr();
        let n = size_of::<Self>();
        let remaining = write_buf.len() - n;

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            buf_ptr.copy_from_nonoverlapping((*self as u8).to_le_bytes().as_ptr(), n);
            std::slice::from_raw_parts_mut(buf_ptr.add(n), remaining)
        }
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = read_buf.split_at(size_of::<bool>());
        let x = u8::from_le_bytes(chunk.try_into().unwrap()) != 0;

        (format!("{}", x), rest)
    }
}

impl Serialize for char {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let buf_ptr = write_buf.as_mut_ptr();
        let n = size_of::<Self>();
        let remaining = write_buf.len() - n;

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            buf_ptr.copy_from_nonoverlapping((*self as u32).to_le_bytes().as_ptr(), n);
            std::slice::from_raw_parts_mut(buf_ptr.add(n), remaining)
        }
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = read_buf.split_at(size_of::<char>());
        // Assuming that we encoded this char
        let c = char::from_u32(u32::from_le_bytes(chunk.try_into().unwrap())).unwrap();

        (format!("{}", c), rest)
    }
}

impl Serialize for &str {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let str_len = self.len();
        let buf_ptr = write_buf.as_mut_ptr();
        let remaining = write_buf.len() - SIZE_LENGTH - str_len;

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            buf_ptr.copy_from_nonoverlapping(str_len.to_le_bytes().as_ptr(), SIZE_LENGTH);
            let s_ptr = buf_ptr.add(SIZE_LENGTH);
            s_ptr.copy_from_nonoverlapping(self.as_bytes().as_ptr(), str_len);

            std::slice::from_raw_parts_mut(s_ptr.add(str_len), remaining)
        }
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (len_chunk, chunk) = read_buf.split_at(SIZE_LENGTH);
        let str_len = usize::from_le_bytes(len_chunk.try_into().unwrap());

        let (str_chunk, rest) = chunk.split_at(str_len);
        let s = from_utf8(str_chunk).unwrap();

        (s.to_string(), rest)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        SIZE_LENGTH + self.len()
    }
}

impl Serialize for Cow<'_, str> {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.as_ref().encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <&str as Serialize>::decode(read_buf)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.as_ref().buffer_size_required()
    }
}

impl Serialize for String {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.as_str().encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <&str as Serialize>::decode(read_buf)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.as_str().buffer_size_required()
    }
}

impl<T: Serialize> Serialize for &T {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        (*self).encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <T as Serialize>::decode(read_buf)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        (*self).buffer_size_required()
    }
}

impl<const N: usize, T: Serialize> Serialize for [T; N] {
    #[inline]
    fn encode<'buf>(&self, mut write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        for i in self {
            write_buf = i.encode(write_buf);
        }

        write_buf
    }

    fn decode(mut read_buf: &[u8]) -> (String, &[u8]) {
        let decoded = {
            let mut decoded_all: [MaybeUninit<String>; N] =
                unsafe { MaybeUninit::uninit().assume_init() };
            let mut decoded;

            for elem in &mut decoded_all[..] {
                // TODO(speed): very slow! should revisit whether really want
                // `decode` to return String.
                (decoded, read_buf) = T::decode(read_buf);
                elem.write(decoded);
            }

            // NOTE: transmute for const arrays doesn't seem to work currently: Need
            // https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.array_assume_init
            // which is unstable
            decoded_all.map(|x| unsafe { x.assume_init() })
        };

        (format!("{:?}", decoded), read_buf)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.get(0).map(|a| a.buffer_size_required()).unwrap_or(0) * self.len()
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let n_elems = self.len();
        let buf_ptr = write_buf.as_mut_ptr();
        let buf_len = write_buf.len();

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        let mut rest = unsafe {
            buf_ptr.copy_from_nonoverlapping(n_elems.to_le_bytes().as_ptr(), SIZE_LENGTH);
            std::slice::from_raw_parts_mut(buf_ptr.add(SIZE_LENGTH), buf_len - SIZE_LENGTH)
        };

        for i in self {
            rest = i.encode(rest);
        }

        rest
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

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.get(0).map(|a| a.buffer_size_required()).unwrap_or(0) * self.len() + SIZE_LENGTH
    }
}

impl<T: Serialize> Serialize for Box<T> {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.as_ref().encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <T as Serialize>::decode(read_buf)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.as_ref().buffer_size_required()
    }
}

impl<T: Serialize> Serialize for Option<T> {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let buf_ptr = write_buf.as_mut_ptr();
        let n = size_of::<u8>();

        if let Some(v) = self.as_ref() {
            // SAFETY: We requested the exact amount required from the queue, so
            // should not run out of space here.
            let value_chunk = unsafe {
                buf_ptr.write(1u8);
                std::slice::from_raw_parts_mut(buf_ptr.add(n), write_buf.len() - n)
            };

            return v.encode(value_chunk);
        }

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            buf_ptr.write(2u8);
            std::slice::from_raw_parts_mut(buf_ptr.add(n), write_buf.len() - n)
        }
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (tag_chunk, mut chunk) = read_buf.split_at(size_of::<u8>());
        let tag = u8::from_le_bytes(tag_chunk.try_into().unwrap());
        let result = match tag {
            1 => {
                let (value, rest) = <T as Serialize>::decode(chunk);
                chunk = rest;

                format!("Some({})", value)
            }
            2 => "None".to_string(),
            // TODO: better error handling for `Serialize`, in general
            _ => panic!("unexpected bytes read"),
        };

        (result, chunk)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        size_of::<u8>()
            + self
                .as_ref()
                .map(|t| t.buffer_size_required())
                .unwrap_or_default()
    }
}

impl<T: Serialize, E: Serialize> Serialize for Result<T, E> {
    #[inline]
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let buf_ptr = write_buf.as_mut_ptr();
        let n = size_of::<u8>();

        match self {
            Ok(v) => {
                // SAFETY: We requested the exact amount required from the queue, so
                // should not run out of space here.
                let value_chunk = unsafe {
                    buf_ptr.write(1u8);
                    std::slice::from_raw_parts_mut(buf_ptr.add(n), write_buf.len() - n)
                };

                v.encode(value_chunk)
            }
            Err(v) => {
                // SAFETY: We requested the exact amount required from the queue, so
                // should not run out of space here.
                let value_chunk = unsafe {
                    buf_ptr.write(2u8);
                    std::slice::from_raw_parts_mut(buf_ptr.add(n), write_buf.len() - n)
                };

                v.encode(value_chunk)
            }
        }
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (tag_chunk, mut chunk) = read_buf.split_at(size_of::<u8>());
        let tag = u8::from_le_bytes(tag_chunk.try_into().unwrap());
        let result = match tag {
            1 => {
                let (value, rest) = <T as Serialize>::decode(chunk);
                chunk = rest;

                format!("Ok({})", value)
            }
            2 => {
                let (value, rest) = <E as Serialize>::decode(chunk);
                chunk = rest;

                format!("Err({})", value)
            }
            // TODO: better error handling for `Serialize`, in general
            _ => panic!("unexpected bytes read"),
        };

        (result, chunk)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        size_of::<u8>()
            + match self {
                Ok(v) => v.buffer_size_required(),
                Err(v) => v.buffer_size_required(),
            }
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
            #[inline]
            fn encode<'buf>(&self, mut write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
                let ($(ref $name,)*) = *self;
                $( write_buf = $name.encode(write_buf); )*

                write_buf
            }

            #[allow(non_snake_case)]
            fn decode(read_buf: &[u8]) -> (String, &[u8]) {
                $(let (ref $name, read_buf) = <$name as Serialize>::decode(read_buf);)*
                (format!(concat!("(", repeat_fmt!($($name),*), ")"), $($name),*), read_buf)
            }

            #[allow(non_snake_case)]
            #[inline]
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
pub fn encode_debug<T: std::fmt::Debug>(val: T, write_buf: &mut [u8]) -> &mut [u8] {
    let val_string = format!("{:?}", val);
    let str_len = val_string.len();
    let remaining = write_buf.len() - SIZE_LENGTH - str_len;
    let buf_ptr = write_buf.as_mut_ptr();

    // SAFETY: We requested the exact amount required from the queue, so
    // should not run out of space here.
    unsafe {
        buf_ptr.copy_from_nonoverlapping(str_len.to_le_bytes().as_ptr(), SIZE_LENGTH);
        let s_ptr = buf_ptr.add(SIZE_LENGTH);
        s_ptr.copy_from_nonoverlapping(val_string.as_bytes().as_ptr(), str_len);
        std::slice::from_raw_parts_mut(s_ptr.add(str_len), remaining)
    }
}

pub fn decode_debug(read_buf: &[u8]) -> (String, &[u8]) {
    let (len_chunk, rest) = read_buf.split_at(SIZE_LENGTH);
    let len = usize::from_le_bytes(len_chunk.try_into().unwrap());
    let (str_chunk, rest) = rest.split_at(len);

    (std::str::from_utf8(str_chunk).unwrap().to_string(), rest)
}

#[cfg(test)]
mod tests {
    use crate::serialize::decode_debug;
    use crate::{self as quicklog};
    use crate::{
        serialize::{encode_debug, Serialize},
        Serialize,
    };
    use std::borrow::Cow;
    use std::mem::size_of;

    use super::DecodeFn;

    const fn get_decode<T: Serialize>(_: &T) -> DecodeFn {
        T::decode
    }

    macro_rules! decode_and_assert {
        ($decode:expr, $buf:expr) => {{
            let (out, rest) = get_decode(&$decode)($buf);
            assert_eq!(format!("{}", $decode), out);
            rest
        }};

        ($decode:expr, $expected:expr, $buf:expr) => {{
            let (out, rest) = get_decode(&$decode)($buf);
            assert_eq!($expected, out);
            rest
        }};
    }

    macro_rules! assert_primitive_encode_decode {
        ($primitive:ty, $val:expr) => {{
            const BUF_SIZE: usize = size_of::<$primitive>();
            let mut buf = [0u8; BUF_SIZE];

            let x: $primitive = $val;
            _ = x.encode(&mut buf);
            decode_and_assert!(x, &buf);
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

        let chunk = a.encode(&mut buf);
        let chunk = b.encode(chunk);
        _ = c.encode(chunk);

        let rest = decode_and_assert!(a, &buf);
        let rest = decode_and_assert!(b, rest);
        _ = decode_and_assert!(c, rest);
    }

    #[test]
    fn serialize_str() {
        let mut buf = [0; 128];
        let s = "hello world";
        let v = Cow::from("hello world 2");
        let rest = s.encode(&mut buf);
        let _ = v.encode(rest);

        let rest = decode_and_assert!(s, &buf);
        _ = decode_and_assert!(v, rest);
    }

    #[test]
    fn serialize_string() {
        let mut buf = [0; 128];
        let s = "hello world".to_string();
        let _ = s.encode(&mut buf);

        decode_and_assert!(s, &buf);
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
        _ = encode_debug(&s, &mut buf);
        let (out, _) = decode_debug(&buf);

        assert_eq!(format!("{:?}", s), out,)
    }

    #[test]
    fn serialize_bool_char() {
        let a = true;
        let b = 'b';
        let c = 'ß';
        let mut buf = [0; 128];
        {
            let rest = a.encode(&mut buf);
            let rest = b.encode(rest);
            _ = c.encode(rest);
        }

        let rest = decode_and_assert!(a, &buf);
        let rest = decode_and_assert!(b, rest);
        decode_and_assert!(c, rest);
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
        _ = (&a, b, &c, &s).encode(&mut buf);

        decode_and_assert!(
            (&a, b, c, s),
            format!(
                "({}, {}, {}, SerializeStruct {{ d: {}, e: {} }})",
                a, b, c, d, e
            ),
            &buf
        );
    }

    #[test]
    fn serialize_arr() {
        let a = ["hello world", "bye world"];
        let mut buf = [0; 256];
        _ = a.encode(&mut buf);

        decode_and_assert!(a, format!("{:?}", a), &buf);
    }

    #[test]
    fn serialize_vec() {
        let a = vec!["hello world", "bye world"];
        let mut buf = [0; 256];
        _ = a.encode(&mut buf);

        decode_and_assert!(a, format!("{:?}", a), &buf);
    }

    #[test]
    fn serialize_ref() {
        let a = &5;
        let b = Box::new(vec!["1", "2", "3"]);
        let mut buf = [0; 256];
        let rest = a.encode(&mut buf);
        _ = b.encode(rest);

        let rest = decode_and_assert!(a, &buf);
        decode_and_assert!(b, format!("{:?}", b), rest);
    }

    #[test]
    fn serialize_option() {
        let a: Option<usize> = Some(5);
        let b: Option<bool> = None;
        let c: Option<Vec<&str>> = Some(vec!["1", "2", "3"]);
        let mut buf = [0; 256];

        let rest = a.encode(&mut buf);
        let rest = b.encode(rest);
        _ = c.encode(rest);

        let rest = decode_and_assert!(a, format!("{:?}", a), &buf);
        let rest = decode_and_assert!(b, format!("{:?}", b), rest);
        _ = decode_and_assert!(c, format!("{:?}", c), rest);
    }

    #[test]
    fn serialize_res() {
        #[allow(unused)]
        #[derive(Debug, Serialize)]
        enum SomeError {
            A,
            B,
        }

        let a: Result<usize, usize> = Ok(5);
        let b: Result<String, SomeError> = Err(SomeError::A);
        let mut buf = [0; 256];

        let rest = a.encode(&mut buf);
        let _ = b.encode(rest);

        let rest = decode_and_assert!(a, format!("{:?}", a), &buf);
        _ = decode_and_assert!(b, format!("{:?}", b), rest);
    }
}

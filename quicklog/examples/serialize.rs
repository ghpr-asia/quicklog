use std::mem::size_of;

use quicklog::queue::Metadata;
use quicklog::serialize::{Serialize, Store};
use quicklog::{flush, info, init, with_flush, with_formatter, PatternFormatter, Serialize};
use quicklog_flush::stdout_flusher::StdoutFlusher;

/// Struct deriving `Serialize` by using the derive-macro.
///
/// This generates a `Serialize` implementation similar to the custom
/// implementation below. As seen below, it can be slightly tedious to have to
/// hand-write a custom implementation, so the most convenient option would be
/// to use the `Serialize` derive macro.
///
/// However, in some cases, one can possibly squeeze out slightly more
/// performance when certain assumptions (see below) can be made about the
/// user-defined type. This is a valid use case for providing a custom
/// `Serialize` implementation.
#[derive(Serialize)]
struct SerializeStruct {
    a: usize,
    b: i32,
    c: u64,
}

/// Struct providing a custom implementation of `Serialize`.
///
/// We could also derive `Serialize` for this struct, but we can possibly take
/// advantage of the fact that the `&str` in `a` has `'static` lifetime. This
/// can save us a few bytes of copying during encoding, since one could simply
/// write the reference into the byte buffer.
///
/// Granted, when writing `encode` manually, this is prone to error. But when
/// maximum performance is needed, this is one way to avoid doing more work than
/// (absolutely) necessary.
struct ManualSerializeStruct {
    a: &'static str,
    b: usize,
    c: String,
}

impl Serialize for ManualSerializeStruct {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        /// Cast reference to an arbitrary `T` into a byte slice.
        ///
        /// Basically converts the reference to pointer representation,
        /// then to a pointer to bytes. Then, restores it as a byte slice
        /// with the same number of bytes as the size of `T`.
        fn any_as_bytes<T: Sized>(a: &T) -> &[u8] {
            unsafe {
                std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
            }
        }

        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
        let (a_chunk, chunk_rest) = chunk.split_at_mut(size_of::<&str>());
        let (b_chunk, c_chunk) = chunk_rest.split_at_mut(size_of::<usize>());

        // Since str is 'static, just copy pointer
        a_chunk.copy_from_slice(any_as_bytes(&self.a));
        b_chunk.copy_from_slice(&self.b.to_le_bytes());

        // When decoding, we need to know how many bytes to read from the buffer
        // in order to restore the contents of `self.c`. So, we first encode the
        // length of `self.c`, then the contents.
        //
        // Alternatively, in a separate context, if we somehow could guarantee
        // that the size of `c` is always SOME_LEN, then we could theoretically
        // save on encoding the length of `c` into the buffer. We assume the
        // more generic case here instead of this edge case.
        //
        // Fortunately, an implementation of `Serialize` for `String` types is
        // already provided out-of-the-box. So, here we can piggy back on that
        // implementation.
        _ = self.c.encode(c_chunk);

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        const STR_SZ: usize = size_of::<&str>();

        let (ab_chunk, rest) = read_buf.split_at(STR_SZ + size_of::<usize>());
        let (a_chunk, b_chunk) = ab_chunk.split_at(STR_SZ);

        // Recall that we encoded the **reference** itself, not the entire
        // str slice!
        // Casting an address to a reference is unsafe due to having to ensure
        // alignment and adherence to usual reference semantics (non-dangling,
        // no mutable aliasing).
        let a: &str =
            unsafe { std::mem::transmute::<[u8; STR_SZ], &str>((*a_chunk).try_into().unwrap()) };
        // Recover `usize`
        let b = usize::from_le_bytes(b_chunk.try_into().unwrap());

        // Recover `String`, utilizing the default-provided implementation
        let (c, rest) = <String as Serialize>::decode(rest);

        (
            format!("ManualSerializeStruct {{ a: {}, b: {}, c: {} }}", a, b, c),
            rest,
        )
    }

    fn buffer_size_required(&self) -> usize {
        size_of::<&str>() + size_of::<usize>() * 2 + self.c.len()
    }
}

/// The default `QuickLogFormatter` outputs the timestamp as well. For this
/// example, to keep things simple, we just return the plain decoded string.
pub struct PlainFormatter;

impl PatternFormatter for PlainFormatter {
    fn custom_format(
        &mut self,
        _: chrono::DateTime<chrono::Utc>,
        _: &Metadata,
        log_record: &str,
    ) -> String {
        format!("{}\n", log_record)
    }
}

fn main() {
    init!();
    with_flush!(StdoutFlusher);
    with_formatter!(PlainFormatter);

    let derive = SerializeStruct { a: 1, b: -2, c: 3 };
    let manual = ManualSerializeStruct {
        a: "Hello world 1",
        b: 50,
        c: "Hello world 2".to_string(),
    };

    // Prints "Struct deriving Serialize: derive=SerializeStruct { a: 1, b: -2, c: 3
    // }"
    info!(^derive, "Struct deriving Serialize:");
    // Prints "Struct implementing custom Serialize: manual=ManualSerializeStruct {
    // a: Hello world 1, b: 50, c: Hello world 2 }"
    info!(^manual, "Struct implementing custom Serialize:");

    flush!();
}

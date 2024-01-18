use std::mem::size_of;

use quicklog::formatter::PatternFormatter;
use quicklog::queue::Metadata;
use quicklog::serialize::Serialize;
use quicklog::{flush, info, init, with_formatter, ReadError, ReadResult, Serialize};

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
struct ManualSerializeStruct<'buf> {
    a: &'static str,
    b: usize,
    c: String,
    d: &'buf [u8],
}

impl Serialize for ManualSerializeStruct<'_> {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
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
        let buf_ptr = write_buf.as_mut_ptr();
        let a_sz = size_of::<&str>();
        let b_sz = size_of::<usize>();

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        let mut buf = unsafe {
            // Since str is 'static, just copy pointer
            buf_ptr.copy_from_nonoverlapping(any_as_bytes(&self.a).as_ptr(), a_sz);

            let b_ptr = buf_ptr.add(a_sz);
            b_ptr.copy_from_nonoverlapping(self.b.to_le_bytes().as_ptr(), b_sz);

            std::slice::from_raw_parts_mut(b_ptr.add(b_sz), write_buf.len() - a_sz - b_sz)
        };

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
        buf = self.c.encode(buf);

        let buf_ptr = buf.as_mut_ptr();
        let d_len = self.d.len();

        // SAFETY: We requested the exact amount required from the queue, so
        // should not run out of space here.
        unsafe {
            // For dynamically-sized buffers, note that we need to encode
            // the *length* of the buffer as well. Otherwise, during decoding, we
            // wouldn't know how many bytes to read to restore the contents of the
            // buffer.
            buf_ptr
                .copy_from_nonoverlapping(self.d.len().to_le_bytes().as_ptr(), size_of::<usize>());
            let p = buf_ptr.add(size_of::<usize>());
            p.copy_from_nonoverlapping(self.d.as_ptr(), d_len);
            std::slice::from_raw_parts_mut(p.add(d_len), buf.len() - size_of::<usize>() - d_len)
        }
    }

    fn decode(read_buf: &[u8]) -> ReadResult<(String, &[u8])> {
        const STR_SZ: usize = size_of::<&str>();

        fn try_split_at(buf: &[u8], n: usize) -> ReadResult<(&[u8], &[u8])> {
            Ok((
                buf.get(..n).ok_or_else(ReadError::insufficient_bytes)?,
                buf.get(n..).ok_or_else(ReadError::insufficient_bytes)?,
            ))
        }

        let (ab_chunk, rest) = try_split_at(read_buf, STR_SZ + size_of::<usize>())?;
        let (a_chunk, b_chunk) = try_split_at(ab_chunk, STR_SZ)?;

        // Recall that we encoded the **reference** itself, not the entire
        // str slice!
        // Casting an address to a reference is unsafe due to having to ensure
        // alignment and adherence to usual reference semantics (non-dangling,
        // no mutable aliasing).
        let a: &str = unsafe { std::mem::transmute::<[u8; STR_SZ], &str>((*a_chunk).try_into()?) };
        // Recover `usize`
        let b = usize::from_le_bytes(b_chunk.try_into()?);

        // Recover `String`, utilizing the default-provided implementation
        let (c, rest) = <String as Serialize>::decode(rest)?;

        // To recover our buffer, we first need to read off how many bytes it
        // originally contained, then restore that many bytes.
        let (d_len_chunk, rest) = try_split_at(rest, size_of::<usize>())?;
        let d_len = usize::from_le_bytes(d_len_chunk.try_into()?);
        let (d, rest) = try_split_at(rest, d_len)?;

        Ok((
            format!(
                "ManualSerializeStruct {{ a: {}, b: {}, c: {}, d: {:?} }}",
                a, b, c, d
            ),
            rest,
        ))
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        // Manually compute size of pointer, since we just copy the pointer
        // in this case (instead of utilizing the default `Serialize`
        // implementation for &str)
        let a_len = size_of::<&str>();

        a_len
            + self.b.buffer_size_required()
            + self.c.buffer_size_required()
            + (self.d.len() + size_of::<usize>())
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
        _: &[String],
        log_record: &str,
    ) -> String {
        format!("{}\n", log_record)
    }
}

fn main() {
    init!();
    with_formatter!(PlainFormatter);

    let derive = SerializeStruct { a: 1, b: -2, c: 3 };
    let manual = ManualSerializeStruct {
        a: "Hello world 1",
        b: 50,
        c: "Hello world 2".to_string(),
        d: &[1, 2, 3, 4, 5],
    };

    // Prints "Struct deriving Serialize: derive=SerializeStruct { a: 1, b: -2, c: 3
    // }"
    info!(derive, "Struct deriving Serialize:");

    // Same as above, using different syntax
    info!("Struct deriving Serialize: derive={:^}", derive);

    // Prints "Struct implementing custom Serialize: manual=ManualSerializeStruct {
    // a: Hello world 1, b: 50, c: Hello world 2, d: [1, 2, 3, 4, 5] }"
    info!(manual, "Struct implementing custom Serialize:");

    // Same as above, using different syntax
    info!("Struct implementing custom Serialize: manual={:^}", manual);

    while let Ok(()) = flush!() {}
}

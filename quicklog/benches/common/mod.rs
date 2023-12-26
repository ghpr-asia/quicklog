use quicklog::serialize::Serialize;

#[macro_export]
macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {
        loop_with_cleanup!($bencher, $loop_f, quicklog::logger().flush_noop())
    };

    ($bencher:expr, $loop_f:expr, $cleanup_f:expr) => {{
        quicklog::init!();

        $bencher.iter_custom(|iters| {
            let start = std::time::Instant::now();

            for _i in 0..iters {
                $loop_f;
                _ = $cleanup_f;
            }

            let end = start.elapsed();

            end
        })
    }};
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub(crate) struct BigStruct {
    pub(crate) vec: [i32; 100],
    pub(crate) some: &'static str,
}

impl Serialize for BigStruct {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        // BigStruct is Copy, so we can just memcpy the whole struct
        let buf_ptr = write_buf.as_mut_ptr();
        let bytes = any_as_bytes(self);
        let n = bytes.len();
        let remaining = write_buf.len() - n;

        unsafe {
            buf_ptr.copy_from_nonoverlapping(bytes.as_ptr(), n);
            std::slice::from_raw_parts_mut(buf_ptr.add(n), remaining)
        }
    }

    fn decode(buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = buf.split_at(std::mem::size_of::<Self>());
        let bs: &BigStruct =
            unsafe { &*std::mem::transmute::<_, *const BigStruct>(chunk.as_ptr()) };

        (format!("{:?}", bs), rest)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Nested {
    pub(crate) vec: Vec<BigStruct>,
}

impl Serialize for Nested {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.vec.encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (vec, rest) = <Vec<BigStruct> as Serialize>::decode(read_buf);

        (format!("Nested {{ vec: {:?} }}", vec), rest)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.vec.buffer_size_required()
    }
}

pub(crate) fn any_as_bytes<T: Sized>(a: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>()) }
}

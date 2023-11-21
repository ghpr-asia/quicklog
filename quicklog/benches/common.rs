use quicklog::serialize::{Serialize, Store};

#[macro_export]
macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {
        loop_with_cleanup!($bencher, $loop_f, { quicklog::flush!() })
    };

    ($bencher:expr, $loop_f:expr, $cleanup_f:expr) => {{
        quicklog::init!();

        $bencher.iter_custom(|iters| {
            let start = quicklog::Quicklog::now();

            for _i in 0..iters {
                $loop_f;
            }

            let end = start.elapsed();

            $cleanup_f;

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
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        // BigStruct is Copy, so we can just memcpy the whole struct
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
        chunk.copy_from_slice(any_as_bytes(self));

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = buf.split_at(std::mem::size_of::<Self>());
        let bs: &BigStruct =
            unsafe { &*std::mem::transmute::<_, *const BigStruct>(chunk.as_ptr()) };

        (format!("{:?}", bs), rest)
    }

    fn buffer_size_required(&self) -> usize {
        std::mem::size_of::<Self>()
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Nested {
    pub(crate) vec: Vec<BigStruct>,
}

impl Serialize for Nested {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        self.vec.encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (vec, rest) = <Vec<BigStruct> as Serialize>::decode(read_buf);

        (format!("Nested {{ vec: {:?} }}", vec), rest)
    }

    fn buffer_size_required(&self) -> usize {
        self.vec.buffer_size_required()
    }
}

pub(crate) fn any_as_bytes<T: Sized>(a: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>()) }
}

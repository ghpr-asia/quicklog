use quicklog::serialize::{Serialize, Store, SIZE_LENGTH};

#[macro_export]
macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {
        loop_with_cleanup!($bencher, $loop_f, { quicklog::flush!() })
    };

    ($bencher:expr, $loop_f:expr, $cleanup_f:expr) => {{
        quicklog::init!();

        $bencher.iter_custom(|iters| {
            let start = quanta::Instant::now();

            for _i in 0..iters {
                $loop_f;
            }

            let end = quanta::Instant::now() - start;

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
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
        let (len_chunk, mut vec_chunk) = chunk.split_at_mut(SIZE_LENGTH);

        len_chunk.copy_from_slice(&self.vec.len().to_le_bytes());

        for i in &self.vec {
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
            (decoded, chunk) = BigStruct::decode(chunk);
            vec.push(decoded)
        }

        (format!("Nested {{ vec: {:?} }}", vec), chunk)
    }

    fn buffer_size_required(&self) -> usize {
        self.vec
            .get(0)
            .map(|a| a.buffer_size_required())
            .unwrap_or(0)
            * self.vec.len()
            + SIZE_LENGTH
    }
}

pub(crate) fn any_as_bytes<T: Sized>(a: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>()) }
}

use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use delog::render::DefaultRenderer;
use quanta::Instant;
use quicklog::{
    serialize::{Serialize, Store, SIZE_LENGTH},
    with_flush,
};
use quicklog_flush::noop_flusher::NoopFlusher;

macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {
        loop_with_cleanup!($bencher, $loop_f, { quicklog::flush!() })
    };

    ($bencher:expr, $loop_f:expr, $cleanup_f:expr) => {{
        quicklog::init!();

        $bencher.iter_custom(|iters| {
            let start = Instant::now();

            for _i in 0..iters {
                $loop_f;
            }

            let end = Instant::now() - start;

            $cleanup_f;

            end
        })
    }};
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct BigStruct {
    vec: [i32; 100],
    some: &'static str,
}

#[derive(Debug, Clone)]
struct Nested {
    pub vec: Vec<BigStruct>,
}

impl Serialize for BigStruct {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        fn any_as_bytes<T: Sized>(a: &T) -> &[u8] {
            unsafe {
                std::slice::from_raw_parts(a as *const T as *const u8, std::mem::size_of::<T>())
            }
        }

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

        (format!("{:?}", vec), chunk)
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

pub struct NoopWriter();

impl std::io::Write for NoopWriter {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Ok(_buf.len())
    }

    fn write_all(&mut self, _buf: &[u8]) -> std::io::Result<()> {
        Ok(())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn bench_callsite_tracing(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }

    let (non_blocking, guard) = tracing_appender::non_blocking(NoopWriter {});

    // error can just be due to the subscriber already being init in prev bench run, so we ignore it
    if let Err(_err) = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .try_init()
    {}

    b.iter(|| {
        tracing::info!("Here's some text {:?}", nested);
    });

    drop(guard);
}

#[derive(Debug)]
pub struct DelogNoopFlusher;
impl delog::Flusher for DelogNoopFlusher {
    fn flush(&self, _logs: &str) {}
}

fn bench_callsite_delog(b: &mut Bencher) {
    use delog::*;
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }

    delog!(Delogger, 4096, DelogNoopFlusher);
    static FLUSHER: DelogNoopFlusher = DelogNoopFlusher {};
    static RENDERER: DefaultRenderer = DefaultRenderer {};
    Delogger::init(delog::LevelFilter::Trace, &FLUSHER, &RENDERER).ok();

    b.iter(|| {
        log::info!("Here's some text {:?}", nested);
    });
}

fn bench_logger_nested(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, black_box(quicklog::info!(^nested, "Some data: ")));
}

fn bench_loggers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Loggers");
    group.bench_function("bench quicklog Nested", bench_logger_nested);
    group.bench_function("bench tracing Nested", bench_callsite_tracing);
    group.bench_function("bench delog Nested", bench_callsite_delog);
    group.finish();
}

criterion_group!(benches, bench_loggers);
criterion_main!(benches);

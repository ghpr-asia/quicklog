use std::str::from_utf8;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use delog::render::DefaultRenderer;
use lazy_format::make_lazy_format;
use once_cell::sync::Lazy;
use quanta::Instant;
use quicklog::serialize::{Serialize, Store};
use quicklog::with_flush;
use quicklog_clock::quanta::QuantaClock;
use quicklog_clock::Clock;
use quicklog_flush::noop_flusher::NoopFlusher;

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
    fn encode(&self, write_buf: &'static mut [u8]) -> Store {
        fn decode(buf: &[u8]) -> String {
            let (mut _head, mut tail) = buf.split_at(0);
            let mut vec = vec![];
            for _ in 0..100 {
                (_head, tail) = tail.split_at(4);
                vec.push(i32::from_le_bytes(_head.try_into().unwrap()));
            }
            let s = from_utf8(tail).unwrap();
            format!("vec: {:?}, str: {}", vec, s)
        }

        let (mut _head, mut tail) = write_buf.split_at_mut(0);
        for i in 0..100 {
            (_head, tail) = tail.split_at_mut(4);
            _head.copy_from_slice(&self.vec[i].to_le_bytes())
        }

        tail.copy_from_slice(self.some.as_bytes());

        Store::new(decode, write_buf)
    }

    fn buffer_size_required(&self) -> usize {
        std::mem::size_of::<i32>() * 100 + self.some.len()
    }
}

macro_rules! loop_with_cleanup {
    ($bencher:expr, $loop_f:expr) => {
        loop_with_cleanup!($bencher, $loop_f, {
            while quicklog::try_flush!().is_ok() {}
        })
    };

    ($bencher:expr, $loop_f:expr, $cleanup_f:expr) => {
        $bencher.iter_custom(|iters| {
            let start = Instant::now();

            for _i in 0..iters {
                $loop_f;
            }

            let end = Instant::now() - start;

            $cleanup_f;

            end
        })
    };
}

fn bench_lazy_format(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "the quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }
    b.iter(|| {
        let arg = nested.to_owned();
        black_box(make_lazy_format!(|f| {
            write!(
                f,
                concat!("[{}]\t", "{:?}"),
                quicklog::level::Level::Info,
                arg
            )
        }));
    })
}

fn bench_box_lazy_format(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "the quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }
    b.iter(|| {
        let arg = nested.to_owned();
        black_box(Box::new(make_lazy_format!(|f| {
            write!(
                f,
                concat!("[{}]\t", "{:?}"),
                quicklog::level::Level::Info,
                arg
            )
        })));
    })
}

static CLOCK: Lazy<QuantaClock> = Lazy::new(QuantaClock::new);

fn bench_clock(b: &mut Bencher) {
    b.iter(|| black_box(CLOCK.get_instant()))
}

type Object = Box<Nested>;
static mut CHANNEL: Lazy<(Sender<Object>, Receiver<Object>)> = Lazy::new(channel);

fn bench_channel_send(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    let mut senders = Vec::new();
    for _ in 0..10 {
        nested.vec.push(bs);
        unsafe {
            senders.push(CHANNEL.0.clone());
        }
    }
    loop_with_cleanup!(
        b,
        {
            let arg = nested.clone();
            unsafe {
                CHANNEL.0.send(Box::new(arg)).unwrap_or(());
            }
        },
        { while unsafe { CHANNEL.1.recv_timeout(Duration::from_millis(10)).is_ok() } {} }
    )
}

fn bench_logger_and_flush(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "the quick brown fox jumps over the lazy dog",
    });
    loop_with_cleanup!(b, quicklog::info!("Here's some text {:?}", bs));
}

fn bench_to_owned_nested_struct(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }
    b.iter(|| {
        black_box(nested.to_owned());
    })
}

fn bench_format_nested_struct(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "the quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }
    b.iter(|| {
        black_box(format!("{:?}", nested));
    })
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
    loop_with_cleanup!(b, quicklog::info!("Some data {:?}", nested));
}

fn bench_logger_serialize(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Here's some text {}", ^bs));
}

fn bench_logger_pass_by_ref(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Here's some text {:?}", &bs));
}

fn bench_logger_no_args(b: &mut Bencher) {
    with_flush!(NoopFlusher);
    loop_with_cleanup!(
        b,
        quicklog::info!("The quick brown fox jumps over the lazy dog.")
    );
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

    let (non_blocking, guard) = tracing_appender::non_blocking(NoopWriter {});

    // error can just be due to the subscriber already being init in prev bench run, so we ignore it
    if let Err(_err) = tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .try_init()
    {}

    b.iter(|| {
        tracing::info!("Here's some text {:?}", bs);
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

    delog!(Delogger, 4096, DelogNoopFlusher);
    static FLUSHER: DelogNoopFlusher = DelogNoopFlusher {};
    static RENDERER: DefaultRenderer = DefaultRenderer {};
    Delogger::init(delog::LevelFilter::Trace, &FLUSHER, &RENDERER).ok();

    b.iter(|| {
        log::info!("Here's some text {:?}", bs);
    });
}

fn bench_loggers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Loggers");
    group.bench_function("bench clock", bench_clock);
    group.bench_function("bench lazy_format", bench_lazy_format);
    group.bench_function("bench to_owned Nested", bench_to_owned_nested_struct);
    group.bench_function("bench Channel send", bench_channel_send);
    group.bench_function("bench box Nested lazy_format", bench_box_lazy_format);
    group.bench_function("bench log Nested", bench_logger_nested);
    group.bench_function("bench format Nested", bench_format_nested_struct);
    group.bench_function("bench log BigStruct serialize", bench_logger_serialize);
    group.bench_function("bench log BigStruct", bench_logger_and_flush);
    group.bench_function("bench log BigStruct ref", bench_logger_pass_by_ref);
    group.bench_function("bench tracing BigStruct", bench_callsite_tracing);
    group.bench_function("bench delog BigStruct", bench_callsite_delog);
    group.bench_function("bench log no args", bench_logger_no_args);
    group.finish();
}

criterion_group!(benches, bench_loggers);
criterion_main!(benches);

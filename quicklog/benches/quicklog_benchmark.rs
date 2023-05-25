use std::str::from_utf8;

use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use delog::render::DefaultRenderer;
use quanta::Instant;
use quicklog::{
    serialize::{Serialize, Store},
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

#[derive(Debug, Clone, Copy)]
enum Side {
    Bid,
    Ask,
}

#[derive(Debug, Clone, Copy)]
struct SmallStruct {
    symbol: &'static str,
    id: u64,
    side: Side,
}

#[derive(Debug, Clone, Copy)]
struct BigStruct {
    vec: [i32; 100],
    some: &'static str,
}

#[derive(Debug, Clone)]
struct Nested {
    pub vec: Vec<BigStruct>,
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
    loop_with_cleanup!(b, quicklog::info!("Here's some text {:?}", nested));
}

fn bench_logger_bigstruct(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Some data {:?}", bs));
}

fn bench_callsite_delog_bigstruct(b: &mut Bencher) {
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

fn bench_callsite_tracing_bigstruct(b: &mut Bencher) {
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

fn bench_logger_small(b: &mut Bencher) {
    let bs = black_box(SmallStruct {
        symbol: "SOME_PAIR",
        id: 12345678,
        side: Side::Ask,
    });
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!("Some data {:?}", bs));
}

fn bench_callsite_delog_small(b: &mut Bencher) {
    use delog::*;
    let bs = black_box(SmallStruct {
        symbol: "SOME_PAIR",
        id: 12345678,
        side: Side::Ask,
    });

    delog!(Delogger, 4096, DelogNoopFlusher);
    static FLUSHER: DelogNoopFlusher = DelogNoopFlusher {};
    static RENDERER: DefaultRenderer = DefaultRenderer {};
    Delogger::init(delog::LevelFilter::Trace, &FLUSHER, &RENDERER).ok();

    b.iter(|| {
        log::info!("Here's some text {:?}", bs);
    });
}

fn bench_callsite_tracing_small(b: &mut Bencher) {
    let bs = black_box(SmallStruct {
        symbol: "SOME_PAIR",
        id: 12345678,
        side: Side::Ask,
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

fn bench_loggers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Loggers");
    group.bench_function("bench quicklog", bench_logger_nested);
    group.bench_function("bench tracing", bench_callsite_tracing);
    group.bench_function("bench delog", bench_callsite_delog);
    group.bench_function("bench quicklog BigStruct", bench_logger_bigstruct);
    group.bench_function("bench tracing BigStruct", bench_callsite_tracing_bigstruct);
    group.bench_function("bench delog BigStruct", bench_callsite_delog_bigstruct);
    group.bench_function("bench quicklog SmallStruct", bench_logger_small);
    group.bench_function("bench tracing SmallStruct", bench_callsite_tracing_small);
    group.bench_function("bench delog SmallStruct", bench_callsite_delog_small);
}

criterion_group!(benches, bench_loggers);
criterion_main!(benches);

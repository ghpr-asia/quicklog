#![allow(dead_code, unused_imports)]

use std::fs::{File, OpenOptions};
use std::io::LineWriter;
use std::str::from_utf8;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, spawn};

use std::time::{Duration, Instant};

use criterion::{black_box, criterion_group, criterion_main, BatchSize, Bencher, Criterion};
use delog::render::DefaultRenderer;
use quicklog::serialize::{Serialize, Store};
use quicklog::{with_clock, with_flush};
use quicklog_clock::quanta::QuantaClock;
use quicklog_flush::noop_flusher::NoopFlusher;

#[derive(Debug, Clone, Copy)]
struct BigStruct {
    vec: [i32; 100],
    some: &'static str,
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

fn bench_logger_and_flush(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    with_flush!(NoopFlusher);
    let write_thread = thread::spawn(move || {
        quicklog::flush!();
    });
    b.iter(|| {
        quicklog::info!("Here's some text {:?}", bs);
    });
    write_thread
        .join()
        .expect("Unable to join quicklog flush thread");
}

fn bench_logger_serialize_and_flush(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    with_flush!(NoopFlusher);
    let write_thread = thread::spawn(move || {
        quicklog::flush!();
    });
    b.iter(|| {
        quicklog::info!("Here's some text {}", ^bs);
    });
    write_thread
        .join()
        .expect("Unable to join quicklog flush thread");
}

fn bench_logger_and_flush_pass_by_ref(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    with_flush!(NoopFlusher);
    let write_thread = thread::spawn(move || {
        quicklog::flush!();
    });
    b.iter(|| {
        quicklog::info!("Here's some text {:?}", &bs);
    });
    write_thread
        .join()
        .expect("Unable to join quicklog flush thread");
}

fn bench_logger_no_args_and_flush(b: &mut Bencher) {
    with_flush!(NoopFlusher);
    let write_thread = thread::spawn(move || {
        quicklog::flush!();
    });
    b.iter(|| {
        quicklog::info!("The quick brown fox jumps over the lazy dog.");
    });
    write_thread
        .join()
        .expect("Unable to join quicklog flush thread");
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
    group.bench_function(
        "bench logger serialize with noop flush",
        bench_logger_serialize_and_flush,
    );
    group.bench_function("bench logger with noop flush", bench_logger_and_flush);
    group.bench_function(
        "bench logger with noop flush, pass by ref",
        bench_logger_and_flush_pass_by_ref,
    );
    group.bench_function(
        "bench logger no args with noop flush",
        bench_logger_no_args_and_flush,
    );
    group.bench_function("bench tracing", bench_callsite_tracing);
    group.bench_function("bench delog", bench_callsite_delog);
    group.finish();
}

criterion_group!(benches, bench_loggers);
criterion_main!(benches);

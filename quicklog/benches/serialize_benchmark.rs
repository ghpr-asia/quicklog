use std::mem::{size_of, MaybeUninit};

use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use quicklog::serialize::{Serialize, Store};

mod common;

use common::any_as_bytes;

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
struct ArrStruct<const N: usize> {
    arr: [u64; N],
}

impl<const N: usize> ArrStruct<N> {
    fn new() -> Self {
        let mut i: u64 = 0;
        let mut arr: [MaybeUninit<u64>; N] = unsafe { MaybeUninit::uninit().assume_init() };

        for elem in &mut arr {
            elem.write(i);
            i = i.wrapping_add(1);
        }

        // NOTE: transmute for const arrays doesn't seem to work currently: Need
        // https://doc.rust-lang.org/std/mem/union.MaybeUninit.html#method.array_assume_init
        // which is unstable
        let arr = arr.map(|x| unsafe { x.assume_init() });
        Self { arr }
    }
}

impl<const N: usize> Serialize for ArrStruct<N> {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> (Store<'buf>, &'buf mut [u8]) {
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());
        chunk.copy_from_slice(any_as_bytes(self));

        (Store::new(Self::decode, chunk), rest)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (chunk, rest) = read_buf.split_at(size_of::<u64>() * N);
        let arr: &[u64] = unsafe { std::slice::from_raw_parts(chunk.as_ptr().cast(), N) };

        (format!("ArrStruct {{ arr: {:?} }}", arr), rest)
    }

    fn buffer_size_required(&self) -> usize {
        size_of::<u64>() * N
    }
}

fn bench_64(b: &mut Bencher) {
    let arr = black_box(ArrStruct::<{ 64 / size_of::<u64>() }>::new());
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(b, black_box(quicklog::info!(arr, "Some data:")));
}

fn bench_128(b: &mut Bencher) {
    let arr = black_box(ArrStruct::<{ 128 / size_of::<u64>() }>::new());
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(b, black_box(quicklog::info!(arr, "Some data:")));
}

fn bench_256(b: &mut Bencher) {
    let arr = black_box(ArrStruct::<{ 256 / size_of::<u64>() }>::new());
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(b, black_box(quicklog::info!(arr, "Some data:")));
}

fn bench_512(b: &mut Bencher) {
    let arr = black_box(ArrStruct::<{ 512 / size_of::<u64>() }>::new());
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(b, black_box(quicklog::info!(arr, "Some data:")));
}

fn bench_1024(b: &mut Bencher) {
    let arr = black_box(ArrStruct::<{ 1024 / size_of::<u64>() }>::new());
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(b, black_box(quicklog::info!(arr, "Some data:")));
}

fn bench_3x4(b: &mut Bencher) {
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(
        b,
        black_box(quicklog::info!(a = 1u32, b = 2u32, c = 3u32, "Some data:"))
    );
}

fn bench_2x4_string(b: &mut Bencher) {
    quicklog::with_flush!(quicklog::NoopFlusher);
    loop_with_cleanup!(
        b,
        black_box(quicklog::info!(
            a = 1u32,
            b = 2u32,
            c = "The quick brown fox jumps over the lazy dog",
            "Some data:"
        ))
    );
}

fn bench_serialize(c: &mut Criterion) {
    let mut group = c.benchmark_group("Serialize");
    group.bench_function("64B", bench_64);
    group.bench_function("128B", bench_128);
    group.bench_function("256B", bench_256);
    group.bench_function("512B", bench_512);
    group.bench_function("1024B", bench_1024);
    group.bench_function("3x4", bench_3x4);
    group.bench_function("2x4 + string", bench_2x4_string);
    group.finish();
}

criterion_group!(benches, bench_serialize);
criterion_main!(benches);

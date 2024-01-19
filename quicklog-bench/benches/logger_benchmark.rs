use std::fmt::Display;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use lazy_format::make_lazy_format;
use quicklog::{init, with_flush, NoopFlusher};
use recycle_box::{coerce_box, RecycleBox};

mod common;
use common::{BigStruct, Nested};

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

fn bench_recycle_box_lazy_format(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "the quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    for _ in 0..10 {
        nested.vec.push(bs)
    }

    let mut recycle_box_arena = Vec::new();

    for _ in 0..10 {
        let arg = nested.to_owned();

        let lf = RecycleBox::new(make_lazy_format!(|f| {
            write!(
                f,
                concat!("[{}]\t", "{:?}"),
                quicklog::level::Level::Info,
                arg
            )
        }));

        let lf_display: RecycleBox<dyn Display> = coerce_box!(lf);

        recycle_box_arena.push(lf_display);
    }

    b.iter(|| {
        let arg = nested.to_owned();
        let box_from_arena = recycle_box_arena.pop().unwrap();
        let return_box_to_arena: RecycleBox<dyn Display> = coerce_box!(RecycleBox::recycle(
            box_from_arena,
            make_lazy_format!(|f| {
                write!(
                    f,
                    concat!("[{}]\t", "{:?}"),
                    quicklog::level::Level::Info,
                    arg
                )
            })
        ));
        // This operation should not be counted as it happens elsewhere
        recycle_box_arena.push(black_box(return_box_to_arena));
    })
}

fn bench_clock(b: &mut Bencher) {
    b.iter(|| black_box(quicklog::now()))
}

fn bench_channel_send(b: &mut Bencher) {
    type Object = Box<Nested>;
    let channel: (Sender<Object>, Receiver<Object>) = channel();
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    let mut nested = black_box(Nested { vec: Vec::new() });
    let mut senders = Vec::new();
    for _ in 0..10 {
        nested.vec.push(bs);
        senders.push(channel.0.clone());
    }
    loop_with_cleanup!(
        b,
        {
            let arg = nested.clone();
            channel.0.send(Box::new(arg)).unwrap_or(());
        },
        while channel.1.recv_timeout(Duration::from_millis(10)).is_ok() {}
    )
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

fn bench_logger_no_args(b: &mut Bencher) {
    init!();
    with_flush!(NoopFlusher);
    loop_with_cleanup!(
        b,
        quicklog::info!("The quick brown fox jumps over the lazy dog.")
    );
}

fn bench_logger_serialize(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    init!();
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!(bs, "Here's some text"));
}

fn bench_logger_and_flush(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "the quick brown fox jumps over the lazy dog",
    });
    loop_with_cleanup!(b, quicklog::info!(?bs, "Here's some text"));
}

fn bench_logger_pass_by_ref(b: &mut Bencher) {
    let bs = black_box(BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    });
    init!();
    with_flush!(NoopFlusher);
    loop_with_cleanup!(b, quicklog::info!(a = ?&bs, "Here's some text"));
}

fn bench_loggers(c: &mut Criterion) {
    let mut group = c.benchmark_group("Loggers");
    group.bench_function("bench clock", bench_clock);
    group.bench_function("bench lazy_format", bench_lazy_format);
    group.bench_function("bench to_owned Nested", bench_to_owned_nested_struct);
    group.bench_function("bench Channel send", bench_channel_send);
    group.bench_function("bench box Nested lazy_format", bench_box_lazy_format);
    group.bench_function("bench format Nested", bench_format_nested_struct);
    group.bench_function("bench log BigStruct serialize", bench_logger_serialize);
    group.bench_function("bench log BigStruct", bench_logger_and_flush);
    group.bench_function("bench log BigStruct ref", bench_logger_pass_by_ref);
    group.bench_function("bench log no args", bench_logger_no_args);
    group.bench_function(
        "bench recycle box lazy format",
        bench_recycle_box_lazy_format,
    );
    group.finish();
}

criterion_group!(benches, bench_loggers);
criterion_main!(benches);

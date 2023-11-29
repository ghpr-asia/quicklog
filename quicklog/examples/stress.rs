use std::time::Duration;

use quicklog::{flush, info, init, queue::FlushError, with_flush, NoopFlusher};

#[allow(unused)]
#[derive(Debug)]
struct Message {
    a: usize,
    b: &'static str,
    c: &'static str,
}

// Sample script to stress the queue for edge cases.
fn main() {
    init!();
    with_flush!(NoopFlusher);

    let message = Message { a: 0x3c000ffd_usize, b: "0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", c: "hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world hello world" };

    std::thread::spawn(|| loop {
        std::thread::sleep(Duration::from_micros(200));
        match flush!() {
            Ok(()) | Err(FlushError::Empty) => {}
            Err(e) => {
                eprintln!("unexpected error: {:?}", e)
            }
        }
    });

    loop {
        info!("Some message: {:?}", message);
        std::thread::sleep(Duration::from_nanos(15));
    }
}

mod chunk;
mod log;

use std::{
    cell::Cell,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::utils::likely;
pub use chunk::*;
pub use log::*;

use crossbeam_utils::CachePadded;

#[derive(Debug, PartialEq, Eq)]
pub enum QueueError {
    NotEnoughSpace,
}

/// Single-producer, single-consumer queue.
pub struct Queue {
    _buf: Vec<u8>,
    atomic_writer_pos: CachePadded<AtomicUsize>,
    atomic_reader_pos: CachePadded<AtomicUsize>,
}

impl Queue {
    #[allow(clippy::new_ret_no_self)]
    pub(crate) fn new(capacity: usize) -> (Producer, Consumer) {
        let capacity = next_power_of_two(capacity);

        // Double capacity to allow write overflow
        let mut buffer = Vec::with_capacity(2 * capacity);
        let buf = buffer.as_mut_ptr();
        let shared_map = Arc::new(Self {
            _buf: buffer,
            atomic_writer_pos: CachePadded::default(),
            atomic_reader_pos: CachePadded::default(),
        });
        let mask = capacity - 1;

        (
            Producer {
                queue: shared_map.clone(),
                buf,
                mask,
                writer_pos: Cell::default(),
                reader_pos: Cell::default(),
            },
            Consumer {
                queue: shared_map,
                buf,
                mask,
                writer_pos: Cell::default(),
                reader_pos: Cell::default(),
            },
        )
    }
}

/// Writer to a queue.
pub struct Producer {
    queue: Arc<Queue>,
    buf: *mut u8,
    mask: usize,
    writer_pos: Cell<usize>,
    reader_pos: Cell<usize>,
}

impl Producer {
    /// Returns a slice from the queue for writing. Errors if the remaining
    /// space in the queue is less than `n`.
    #[inline]
    pub(crate) fn prepare_write(&mut self, n: usize) -> Result<&mut [u8], QueueError> {
        let tail = self.writer_pos.get();
        let head = self.reader_pos.get();

        let mask = self.mask;
        let capacity = mask + 1;
        let remaining = capacity.saturating_sub(tail.wrapping_sub(head));
        if likely(remaining >= n) {
            return Ok(unsafe {
                std::slice::from_raw_parts_mut(self.buf.add(tail & mask), remaining)
            });
        }

        let head = self.queue.atomic_reader_pos.load(Ordering::Acquire);
        self.reader_pos.set(head);

        let remaining = capacity.saturating_sub(tail.wrapping_sub(head));
        if remaining >= n {
            Ok(unsafe { std::slice::from_raw_parts_mut(self.buf.add(tail & mask), remaining) })
        } else {
            Err(QueueError::NotEnoughSpace)
        }
    }

    /// Advances the local pointer for this writer.
    #[inline]
    pub(crate) fn finish_write(&mut self, n: usize) {
        let writer_pos = self.writer_pos.get();
        self.writer_pos.set(writer_pos.wrapping_add(n));
    }

    /// Commits written slots to be available for reading.
    #[inline]
    pub(crate) fn commit_write(&mut self) {
        self.queue
            .atomic_writer_pos
            .store(self.writer_pos.get(), Ordering::Release);
    }
}

/// Reader of a queue.
pub struct Consumer {
    queue: Arc<Queue>,
    buf: *const u8,
    mask: usize,
    writer_pos: Cell<usize>,
    reader_pos: Cell<usize>,
}

impl Consumer {
    /// Returns a slice from the queue for reading. Errors if there is nothing
    /// to read from the queue.
    #[inline]
    pub(crate) fn prepare_read(&mut self) -> Result<&[u8], QueueError> {
        let tail = self.writer_pos.get();
        let head = self.reader_pos.get();

        let available = tail.wrapping_sub(head);
        let mask = self.mask;
        if available != 0 {
            return Ok(unsafe { std::slice::from_raw_parts(self.buf.add(head & mask), available) });
        }

        let tail = self.queue.atomic_writer_pos.load(Ordering::Acquire);
        self.writer_pos.set(tail);

        let available = tail.wrapping_sub(head);
        if available != 0 {
            Ok(unsafe { std::slice::from_raw_parts(self.buf.add(head & mask), available) })
        } else {
            Err(QueueError::NotEnoughSpace)
        }
    }

    /// Advances the local pointer for this reader.
    #[inline]
    pub(crate) fn finish_read(&mut self, n: usize) {
        let reader_pos = self.reader_pos.get();
        self.reader_pos.set(reader_pos.wrapping_add(n));
    }

    /// Commits read slots to be available for writing.
    #[inline]
    pub(crate) fn commit_read(&mut self) {
        self.queue
            .atomic_reader_pos
            .store(self.reader_pos.get(), Ordering::Release);
    }
}

/// Rounds up `n` to the next higher power of two.
fn next_power_of_two(n: usize) -> usize {
    if n == 0 {
        return 1;
    }

    if n & (n - 1) == 0 {
        return n;
    }

    let mut m = n - 1;
    let mut res = 1;
    while m > 0 {
        m >>= 1;
        res <<= 1;
    }

    res
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::Ordering;

    use crate::queue::QueueError;

    use super::Queue;

    #[test]
    fn read_write() {
        let (mut producer, mut consumer) = Queue::new(64);

        // Loop to fill up queue and empty it multiple times
        for _ in 0..256 {
            // Multiple writes to saturate queue
            let _buf = producer.prepare_write(32).unwrap();
            producer.finish_write(32);
            producer.commit_write();

            let _buf = producer.prepare_write(32).unwrap();
            producer.finish_write(32);
            producer.commit_write();

            // When queue is full, then cannot get write buffer
            assert_eq!(producer.prepare_write(1), Err(QueueError::NotEnoughSpace));

            // Multiple reads to empty queue
            let buf = consumer.prepare_read().unwrap();
            assert_eq!(buf.len(), 64);
            consumer.finish_read(32);
            consumer.commit_read();

            let buf = consumer.prepare_read().unwrap();
            assert_eq!(buf.len(), 32);
            consumer.finish_read(32);
            consumer.commit_read();

            // When queue is empty, then cannot get read buffer
            assert_eq!(consumer.prepare_read(), Err(QueueError::NotEnoughSpace));
        }
    }

    #[test]
    fn read_write_overflow() {
        const DATA_SIZE: usize = 256;

        // Tests that the queue indices wrap around properly
        let (mut producer, mut consumer) = Queue::new(DATA_SIZE);

        let data = {
            let mut data = Vec::with_capacity(DATA_SIZE);
            for i in 0..DATA_SIZE {
                data.push(i as u8);
            }
            data
        };

        let mut result = [0; DATA_SIZE];
        let start_pos = usize::MAX - 128;
        for i in 0..128 {
            // In each iteration, pretend that we start at a point where we have
            // written to the queue many times, and the write/read indices are on
            // the verge of overflow
            producer
                .queue
                .atomic_writer_pos
                .store(start_pos + i, Ordering::Relaxed);
            producer
                .queue
                .atomic_reader_pos
                .store(start_pos + i, Ordering::Relaxed);

            producer.writer_pos.set(start_pos + i);
            producer.reader_pos.set(start_pos + i);
            consumer.writer_pos.set(start_pos + i);
            consumer.reader_pos.set(start_pos + i);

            // Writing/reading an amount of data from the queue that will cause
            // the indices to overflow
            let buf = producer.prepare_write(DATA_SIZE).unwrap();
            buf.copy_from_slice(&data);
            producer.finish_write(DATA_SIZE);
            producer.commit_write();

            // When queue is full, then cannot get write buffer
            assert_eq!(producer.prepare_write(1), Err(QueueError::NotEnoughSpace));

            let buf = consumer.prepare_read().unwrap();
            result.copy_from_slice(buf);
            consumer.finish_read(DATA_SIZE);
            consumer.commit_read();

            // When queue is empty, then cannot get read buffer
            assert_eq!(consumer.prepare_read(), Err(QueueError::NotEnoughSpace));

            // Check that indices wrap around nicely and data doesn't get
            // corrupted in weird ways
            assert_eq!(result, data.as_slice());
            assert_eq!(
                producer.writer_pos.get(),
                start_pos.wrapping_add(DATA_SIZE + i)
            );
            assert_eq!(
                consumer.reader_pos.get(),
                start_pos.wrapping_add(DATA_SIZE + i)
            );
            result.fill(0);

            // Check one more set of write/reads
            let buf = producer.prepare_write(DATA_SIZE).unwrap();
            buf.copy_from_slice(&data);
            producer.finish_write(DATA_SIZE);
            producer.commit_write();
            assert_eq!(producer.prepare_write(1), Err(QueueError::NotEnoughSpace));

            let buf = consumer.prepare_read().unwrap();
            result.copy_from_slice(buf);
            consumer.finish_read(DATA_SIZE);
            consumer.commit_read();
            assert_eq!(consumer.prepare_read(), Err(QueueError::NotEnoughSpace));

            assert_eq!(result, data.as_slice());
            assert_eq!(
                producer.writer_pos.get(),
                start_pos.wrapping_add(DATA_SIZE * 2 + i)
            );
            assert_eq!(
                consumer.reader_pos.get(),
                start_pos.wrapping_add(DATA_SIZE * 2 + i)
            );
            result.fill(0);
        }
    }
}

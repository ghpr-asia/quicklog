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

#[derive(Debug)]
pub enum QueueError {
    NotEnoughSpace,
}

impl From<QueueError> for WriteError {
    fn from(value: QueueError) -> Self {
        match value {
            QueueError::NotEnoughSpace => WriteError::NotEnoughSpace,
        }
    }
}

/// Single-producer, single-consumer queue.
pub struct Queue {
    _buf: Vec<u8>,
    atomic_writer_pos: CachePadded<AtomicUsize>,
    atomic_reader_pos: CachePadded<AtomicUsize>,
}

impl Queue {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(capacity: usize) -> (Producer, Consumer) {
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
    pub fn prepare_write(&mut self, n: usize) -> Result<&mut [u8], QueueError> {
        let tail = self.writer_pos.get();
        let head = self.reader_pos.get();

        let capacity = self.capacity();
        let remaining = capacity - (tail - head);
        let mask = self.mask;
        if likely(remaining >= n) {
            return Ok(unsafe {
                std::slice::from_raw_parts_mut(self.buf.add(tail & mask), remaining)
            });
        }

        let head = self.queue.atomic_reader_pos.load(Ordering::Acquire);
        self.reader_pos.set(head);

        let remaining = capacity - (tail - head);
        if remaining >= n {
            Ok(unsafe { std::slice::from_raw_parts_mut(self.buf.add(tail & mask), remaining) })
        } else {
            Err(QueueError::NotEnoughSpace)
        }
    }

    /// Advances the local pointer for this writer.
    pub fn finish_write(&mut self, n: usize) {
        let writer_pos = self.writer_pos.get();
        self.writer_pos.set(writer_pos + n);
    }

    /// Commits written slots to be available for reading.
    pub fn commit_write(&mut self) {
        self.queue
            .atomic_writer_pos
            .store(self.writer_pos.get(), Ordering::Release);
    }

    pub fn writer_pos(&self) -> usize {
        self.writer_pos.get()
    }

    #[inline]
    fn capacity(&self) -> usize {
        self.mask + 1
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
    pub fn prepare_read(&mut self) -> Result<&[u8], QueueError> {
        let tail = self.writer_pos.get();
        let head = self.reader_pos.get();

        let available = tail - head;
        let mask = self.mask;
        if available != 0 {
            return Ok(unsafe { std::slice::from_raw_parts(self.buf.add(head & mask), available) });
        }

        let tail = self.queue.atomic_writer_pos.load(Ordering::Acquire);
        self.writer_pos.set(tail);

        let available = tail - head;
        if available != 0 {
            Ok(unsafe { std::slice::from_raw_parts(self.buf.add(head & mask), available) })
        } else {
            Err(QueueError::NotEnoughSpace)
        }
    }

    /// Advances the local pointer for this reader.
    pub fn finish_read(&mut self, n: usize) {
        let reader_pos = self.reader_pos.get();
        self.reader_pos.set(reader_pos + n);
    }

    /// Commits read slots to be available for writing.
    pub fn commit_read(&mut self) {
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

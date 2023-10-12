// Some functions only emitted in macros
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use quicklog::{
    serialize::{Serialize, Store},
    LogRecord, PatternFormatter,
};
use quicklog_flush::Flush;

pub(crate) struct VecFlusher {
    pub(crate) vec: &'static mut Vec<String>,
}

impl VecFlusher {
    pub fn new(vec: &'static mut Vec<String>) -> VecFlusher {
        VecFlusher { vec }
    }
}

impl Flush for VecFlusher {
    fn flush_one(&mut self, display: String) {
        self.vec.push(display);
    }
}

pub(crate) struct TestFormatter;

impl TestFormatter {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

impl PatternFormatter for TestFormatter {
    fn custom_format(&mut self, time: DateTime<Utc>, log_record: LogRecord) -> String {
        format!(
            "[{:?}][{}]\t{}\n",
            time, log_record.level, log_record.log_line
        )
    }
}

pub(crate) fn message_from_log_line(log_line: &str) -> String {
    log_line
        .split('\t')
        .last()
        .map(|s| s.chars().take(s.len() - 1).collect::<String>())
        .unwrap()
}

pub(crate) fn message_and_level_from_log_line(log_line: &str) -> String {
    let timestamp_end_idx = log_line.find(']').unwrap() + 1;
    log_line
        .chars()
        .skip(timestamp_end_idx)
        .take(log_line.len() - timestamp_end_idx - 1)
        .collect::<String>()
}

pub(crate) fn from_log_lines<F: Fn(&str) -> String>(lines: &[String], f: F) -> Vec<String> {
    lines.iter().map(|s| f(s.as_str())).collect::<Vec<_>>()
}

#[derive(Clone, Debug)]
pub(crate) struct Something {
    pub(crate) some_str: &'static str,
}

impl std::fmt::Display for Something {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Something display: {}", self.some_str)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct NestedSomething {
    pub(crate) thing: Something,
}

#[derive(Clone)]
pub(crate) struct A {
    pub(crate) price: u64,
    pub(crate) symbol: &'static str,
    pub(crate) exch_id: u64,
}

impl A {
    pub(crate) fn get_price(&self) -> u64 {
        self.price
    }

    pub(crate) fn get_exch_id(&self) -> u64 {
        self.exch_id
    }

    pub(crate) fn get_symbol(&self) -> &'static str {
        self.symbol
    }
}

#[derive(Clone)]
pub(crate) struct SerializeStruct {
    pub(crate) symbol: String,
}

impl Serialize for SerializeStruct {
    fn encode(&self, write_buf: &'static mut [u8]) -> Store {
        fn decode(read_buf: &[u8]) -> String {
            let x = std::str::from_utf8(read_buf).unwrap();
            x.to_string()
        }
        write_buf.copy_from_slice(self.symbol.as_bytes());
        Store::new(decode, write_buf)
    }

    fn buffer_size_required(&self) -> usize {
        self.symbol.len()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BigStruct {
    pub(crate) vec: [i32; 100],
    pub(crate) some: &'static str,
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
            let s = std::str::from_utf8(tail).unwrap();
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

pub(crate) struct SimpleStruct {
    some_str: &'static str,
}

impl std::fmt::Display for SimpleStruct {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.some_str)
    }
}

#[macro_export]
macro_rules! setup {
    () => {
        quicklog::init!();
        static mut VEC: Vec<String> = Vec::new();
        let vec_flusher = unsafe { common::VecFlusher::new(&mut VEC) };
        quicklog::logger().use_flush(Box::new(vec_flusher));
        quicklog::logger().use_formatter(Box::new(common::TestFormatter::new()))
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! helper_assert {
    (@ $f:expr, $format_string:expr, $check_f:expr) => {
        $f;
        quicklog::flush!();
        let output = unsafe { common::from_log_lines(&VEC, $check_f) };
        assert_eq!(output, vec![$format_string]);
        unsafe {
            let _ = &VEC.clear();
        }
    };
}

#[macro_export]
macro_rules! assert_message_equal {
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_from_log_line) };
}

#[macro_export]
macro_rules! assert_message_with_level_equal {
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_and_level_from_log_line) };
}

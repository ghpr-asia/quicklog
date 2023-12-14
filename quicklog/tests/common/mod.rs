// Some functions only emitted in macros
#![allow(dead_code)]

use chrono::{DateTime, Utc};
use quicklog::{
    formatter::PatternFormatter,
    queue::Metadata,
    serialize::{DecodeFn, Serialize},
    Flush,
};

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

impl PatternFormatter for TestFormatter {
    fn custom_format(
        &mut self,
        time: DateTime<Utc>,
        metadata: &Metadata,
        _: &[String],
        log_record: &str,
    ) -> String {
        format!("[{:?}][{}]\t{}\n", time, metadata.level, log_record)
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

impl Serialize for Something {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.some_str.encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        let (output, rest) = <&str as Serialize>::decode(read_buf);

        (format!("Something {{ some_str: {} }}", output), rest)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.some_str.buffer_size_required()
    }
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
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.symbol.as_str().encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> (String, &[u8]) {
        <&str as Serialize>::decode(read_buf)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        self.symbol.as_str().buffer_size_required()
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct BigStruct {
    pub(crate) vec: [i32; 100],
    pub(crate) some: &'static str,
}

impl Serialize for BigStruct {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        let (chunk, rest) = write_buf.split_at_mut(self.buffer_size_required());

        let elm_size = std::mem::size_of::<i32>();
        let (vec_chunk, str_chunk) = chunk.split_at_mut(self.vec.len() * elm_size);
        let (mut _head, mut _tail) = vec_chunk.split_at_mut(0);
        for i in 0..self.vec.len() {
            (_head, _tail) = _tail.split_at_mut(elm_size);
            _head.copy_from_slice(&self.vec[i].to_le_bytes())
        }

        _ = self.some.encode(str_chunk);

        rest
    }

    fn decode(buf: &[u8]) -> (String, &[u8]) {
        let (mut _head, mut tail) = buf.split_at(0);
        let mut arr = [0; 100];
        let elm_size = std::mem::size_of::<i32>();
        for i in &mut arr {
            (_head, tail) = tail.split_at(elm_size);
            *i = i32::from_le_bytes(_head.try_into().unwrap());
        }
        let (s, rest) = <&str as Serialize>::decode(tail);

        (format!("vec: {:?}, str: {}", arr, s), rest)
    }

    #[inline]
    fn buffer_size_required(&self) -> usize {
        std::mem::size_of::<i32>() * 100 + self.some.buffer_size_required()
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

pub(crate) const fn get_decode<T: Serialize>(_: &T) -> DecodeFn {
    T::decode
}

#[macro_export]
macro_rules! setup {
    () => {
        quicklog::init!();
        static mut VEC: Vec<String> = Vec::new();
        let vec_flusher = unsafe { common::VecFlusher::new(&mut VEC) };
        quicklog::logger().use_flush(Box::new(vec_flusher));
        quicklog::logger().use_formatter(Box::new(common::TestFormatter))
    };
}

#[macro_export]
macro_rules! flush_all {
    () => {
        loop {
            match quicklog::flush!() {
                Ok(()) => {}
                Err(quicklog::queue::FlushError::Empty) => break,
                Err(e) => panic!("{:?}", e),
            }
        }
    };
}

#[macro_export]
macro_rules! decode_and_assert {
    ($decode:expr, $buf:expr) => {{
        let (out, rest) = $crate::common::get_decode(&$decode)($buf);
        assert_eq!(format!("{}", $decode), out);
        rest
    }};

    ($decode:expr, $expected:expr, $buf:expr) => {{
        let (out, rest) = $crate::common::get_decode(&$decode)($buf);
        assert_eq!($expected, out);
        rest
    }};
}

pub(crate) mod json {
    #[macro_export]
    macro_rules! assert_json_fields {
        ($f:expr, $expected:expr) => {
            $f;
            flush_all!();
            let output = unsafe { VEC.get(0).and_then(|s| s.strip_suffix('\n')).unwrap() };
            let fields = extract_fields(output).unwrap();
            assert_eq!(fields, $expected);
            unsafe { VEC.clear() };
        };
    }

    #[macro_export]
    macro_rules! assert_json_no_message {
        ($f:expr) => {
            $f;
            flush_all!();
            let output = unsafe { VEC.get(0).and_then(|s| s.strip_suffix('\n')).unwrap() };
            let fields = extract_fields(output).unwrap();
            assert!(fields.find("message").is_none());
            unsafe { VEC.clear() };
        };
    }

    pub(crate) fn extract_fields(s: &str) -> Option<&str> {
        let key = "\"fields\":";
        let idx = s.find(key)?;
        // Exclude surrounding braces
        Some(&s[(idx + key.len())..(s.len() - 1)])
    }

    pub(crate) fn construct_json_fields(args: &[(&str, &str)]) -> String {
        let mut s = String::new();
        if args.is_empty() {
            return s;
        }

        s.push('{');

        let num_args = args.len();
        for (idx, arg) in args.iter().enumerate() {
            s.push('"');
            s.push_str(arg.0);
            s.push_str("\":\"");
            s.push_str(arg.1);
            s.push('"');

            if idx < num_args - 1 {
                s.push(',');
            }
        }
        s.push('}');
        s
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! helper_assert {
    (@ $f:expr, $format_string:expr, $check_f:expr) => {
        $f;
        flush_all!();
        let output = unsafe { common::from_log_lines(&VEC, $check_f) };
        assert_eq!(output, vec![$format_string]);
        unsafe {
            let _ = &VEC.clear();
        }
    };
}

#[macro_export]
macro_rules! assert_no_messages {
    () => {
        assert_eq!(quicklog::flush!(), Err(quicklog::queue::FlushError::Empty));
    };
}

#[macro_export]
macro_rules! assert_messages {
    ($($messages:expr),*) => {{
        let lines = unsafe { common::from_log_lines(&VEC, common::message_from_log_line) };
        let mut lines = lines.iter();
        $(
            let line = lines.next().unwrap().as_str();
            assert_eq!(line, $messages);
         )*
        unsafe {
            _ = VEC.clear();
        }
    }};
}

#[macro_export]
macro_rules! assert_message_equal {
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_from_log_line) };
}

#[macro_export]
macro_rules! assert_message_with_level_equal {
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_and_level_from_log_line) };
}

// Some functions only emitted in macros
#![allow(dead_code)]

use std::fmt::Write;

use quicklog::{
    fmt::{LogContext, PatternFormatter, Writer},
    serialize::Serialize,
    Flush, ReadResult,
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
    fn custom_format(&self, ctx: LogContext<'_>, writer: &mut Writer) -> std::fmt::Result {
        writeln!(
            writer,
            "[{:?}][{}][{}]\t{}",
            ctx.timestamp(),
            ctx.metadata().level(),
            ctx.metadata().target(),
            ctx.full_message(),
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
    let level_end_idx = log_line[timestamp_end_idx..].find(']').unwrap() + 1;
    let msg_idx = log_line.find('\t').unwrap();

    log_line[timestamp_end_idx..(timestamp_end_idx + level_end_idx)].to_string()
        + &log_line[msg_idx..(log_line.len() - 1)]
}

pub(crate) fn message_and_target_from_log_line(log_line: &str) -> String {
    let timestamp_end_idx = log_line.find(']').unwrap() + 1;
    let level_end_idx = log_line[timestamp_end_idx..].find(']').unwrap() + 1;
    log_line
        .chars()
        .skip(timestamp_end_idx + level_end_idx)
        .take(log_line.len() - level_end_idx - timestamp_end_idx - 1)
        .collect::<String>()
}

pub(crate) fn from_log_lines<F: Fn(&str) -> String>(lines: &[String], f: F) -> Vec<String> {
    lines.iter().map(|s| f(s.as_str())).collect::<Vec<_>>()
}

#[derive(Clone, Debug)]
pub(crate) struct Something {
    pub(crate) some_str: &'static str,
}

impl Something {
    pub(crate) fn some_str(&self) -> &'static str {
        self.some_str
    }
}

impl Serialize for Something {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.some_str.encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> ReadResult<(String, &[u8])> {
        let (output, rest) = <&str as Serialize>::decode(read_buf)?;

        Ok((format!("Something {{ some_str: {} }}", output), rest))
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

#[derive(Clone)]
pub(crate) struct SerializeStruct {
    pub(crate) symbol: String,
}

impl Serialize for SerializeStruct {
    fn encode<'buf>(&self, write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        self.symbol.as_str().encode(write_buf)
    }

    fn decode(read_buf: &[u8]) -> ReadResult<(String, &[u8])> {
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
    fn encode<'buf>(&self, mut write_buf: &'buf mut [u8]) -> &'buf mut [u8] {
        write_buf = self.vec.encode(write_buf);

        self.some.encode(write_buf)
    }

    fn decode(buf: &[u8]) -> ReadResult<(String, &[u8])> {
        let (mut _head, mut tail) = buf.split_at(0);
        let mut arr = [0; 100];
        let elm_size = std::mem::size_of::<i32>();
        for i in &mut arr {
            (_head, tail) = tail.split_at(elm_size);
            *i = i32::from_le_bytes(_head.try_into()?);
        }
        let (s, rest) = <&str as Serialize>::decode(tail)?;

        Ok((format!("vec: {:?}, str: {}", arr, s), rest))
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

#[macro_export]
macro_rules! setup {
    (target_filter = $filter:expr) => {
        static mut VEC: Vec<String> = Vec::new();
        let vec_flusher = unsafe { common::VecFlusher::new(&mut VEC) };
        quicklog::init!(quicklog::config()
            .formatter(common::TestFormatter)
            .flusher(vec_flusher)
            .target_filter($filter));
    };
    (formatter = $fmt:expr) => {
        static mut VEC: Vec<String> = Vec::new();
        let vec_flusher = unsafe { common::VecFlusher::new(&mut VEC) };
        quicklog::init!(quicklog::config().formatter($fmt).flusher(vec_flusher));
    };
    ($config:expr) => {
        static mut VEC: Vec<String> = Vec::new();
        let vec_flusher = unsafe { common::VecFlusher::new(&mut VEC) };
        quicklog::init!($config);
    };
    () => {
        static mut VEC: Vec<String> = Vec::new();
        let vec_flusher = unsafe { common::VecFlusher::new(&mut VEC) };
        quicklog::init!(quicklog::config()
            .formatter(common::TestFormatter)
            .flusher(vec_flusher));
    };
}

#[macro_export]
macro_rules! flush_all {
    () => {
        loop {
            match quicklog::flush!() {
                Ok(()) => {}
                Err(quicklog::FlushError::Empty) => break,
                Err(e) => panic!("{:?}", e),
            }
        }
    };
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
        assert_eq!(quicklog::flush!(), Err(quicklog::FlushError::Empty));
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
    ($format_string:expr) => { assert_message_equal!({}, $format_string); };
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_from_log_line) };
}

#[macro_export]
macro_rules! assert_message_with_level_equal {
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_and_level_from_log_line) };
}

#[macro_export]
macro_rules! assert_message_with_target_equal {
    ($f:expr, $format_string:expr) => { helper_assert!(@ $f, $format_string, common::message_and_target_from_log_line) };
}

#[macro_export]
macro_rules! first_field_from_log_line {
    () => {{
        let line = unsafe { common::from_log_lines(&VEC, |s| s.to_string()) };
        unsafe {
            _ = VEC.clear();
        }
        let end_bracket_idx = line[0].find(']').unwrap();
        line.get(0)
            .and_then(|l| l.get(1..end_bracket_idx))
            .unwrap()
            .to_string()
    }};
}

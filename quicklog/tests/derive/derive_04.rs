// Testing structs with combination of primitives and &str.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct {
    a: usize,
    some_str: &'static str,
    b: i32,
}

fn main() {
    let s = TestStruct {
        a: 999,
        some_str: "hello world",
        b: -32,
    };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        format!(
            "TestStruct {{ a: {}, some_str: {}, b: {} }}",
            s.a, s.some_str, s.b
        ),
        &buf
    );
}

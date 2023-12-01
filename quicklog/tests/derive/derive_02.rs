// Testing structs with multiple primitive fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct {
    a: usize,
    b: i32,
    c: u32,
}

fn main() {
    let s = TestStruct {
        a: 0,
        b: -999,
        c: 2,
    };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        format!("TestStruct {{ a: {}, b: {}, c: {} }}", s.a, s.b, s.c),
        &buf
    );
}

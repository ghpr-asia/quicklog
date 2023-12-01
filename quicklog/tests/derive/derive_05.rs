// Testing structs with unnamed fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct(usize, &'static str, i32);

fn main() {
    let s = TestStruct(999, "hello world", -32);
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(s, format!("TestStruct({}, {}, {})", s.0, s.1, s.2), &buf);
}

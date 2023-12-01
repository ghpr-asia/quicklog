// Testing structs with a simple primitive field.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct {
    size: usize,
}

fn main() {
    let s = TestStruct { size: 0 };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(s, format!("TestStruct {{ size: {} }}", s.size), &buf);
}

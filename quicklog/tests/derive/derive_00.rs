// Testing structs with no fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct;

fn main() {
    let a = TestStruct;
    let mut buf = [0; 128];
    _ = a.encode(&mut buf);

    decode_and_assert!(a, "TestStruct".to_string(), &buf);
}

#![allow(clippy::disallowed_names)]

// Testing structs with collections.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct {
    a: Vec<String>,
    b: Vec<&'static str>,
}

fn main() {
    let s = TestStruct {
        a: vec!["1".to_string(), "2".to_string()],
        b: vec!["3", "4", "5"],
    };
    let mut buf = [0; 256];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        "TestStruct { a: [\"1\", \"2\"], b: [\"3\", \"4\", \"5\"] }".to_string(),
        &buf
    );
}

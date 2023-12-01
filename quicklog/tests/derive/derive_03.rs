// Testing structs with &str types of different lifetimes.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
struct TestStruct<'a> {
    some_str: &'static str,
    another_str: &'a str,
}

fn main() {
    let another_string = "Hello".to_string() + "there";
    let s = TestStruct {
        some_str: "hello world",
        another_str: another_string.as_str(),
    };
    let mut buf = [0; 128];

    _ = s.encode(&mut buf);
    decode_and_assert!(
        s,
        format!(
            "TestStruct {{ some_str: {}, another_str: {} }}",
            s.some_str, s.another_str
        ),
        &buf
    );
}

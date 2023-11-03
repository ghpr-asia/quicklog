// Testing structs with combination of primitives and &str.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

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

    let (store, _) = s.encode(&mut buf);
    assert_eq!(
        format!(
            "TestStruct {{ a: {}, some_str: {}, b: {} }}",
            s.a, s.some_str, s.b
        ),
        format!("{}", store)
    )
}

#![allow(clippy::disallowed_names)]

// Testing enums with multiple fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
enum TestEnum {
    Foo(String),
    Bar { a: String, b: usize },
    Baz(TestStruct),
}

#[derive(Serialize)]
struct TestStruct {
    a: usize,
    b: i32,
    c: u32,
}

fn main() {
    let foo = TestEnum::Foo("hello world".to_string());
    let bar = TestEnum::Bar {
        a: "hello bar".to_string(),
        b: 999,
    };
    let baz = TestEnum::Baz(TestStruct {
        a: 0,
        b: -999,
        c: 2,
    });
    let mut buf = [0; 256];

    let rest = foo.encode(&mut buf);
    let rest = bar.encode(rest);
    _ = baz.encode(rest);

    let rest = decode_and_assert!(foo, "Foo(hello world)", &buf);
    let rest = decode_and_assert!(bar, "Bar { a: hello bar, b: 999 }", rest);
    _ = decode_and_assert!(baz, "Baz(TestStruct { a: 0, b: -999, c: 2 })", rest);
}

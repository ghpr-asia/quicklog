#![allow(clippy::disallowed_names)]

// Testing enums with multiple fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

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

    let (foo_store, rest) = foo.encode(&mut buf);
    let (bar_store, rest) = bar.encode(rest);
    let (baz_store, _) = baz.encode(rest);

    assert_eq!(
        "Foo(hello world) Bar { a: hello bar, b: 999 } Baz(TestStruct { a: 0, b: -999, c: 2 })"
            .to_string(),
        format!("{} {} {}", foo_store, bar_store, baz_store)
    );
}

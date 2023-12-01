// Testing enums with no fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[path = "../common/mod.rs"]
mod common;

#[derive(Serialize)]
enum TestEnum {
    Foo,
    Bar,
    Baz,
}

fn main() {
    let foo = TestEnum::Foo;
    let bar = TestEnum::Bar;
    let baz = TestEnum::Baz;
    let mut buf = [0; 128];

    let rest = foo.encode(&mut buf);
    let rest = bar.encode(rest);
    let _ = baz.encode(rest);

    let rest = decode_and_assert!(foo, "Foo", &buf);
    let rest = decode_and_assert!(bar, "Bar", rest);
    _ = decode_and_assert!(baz, "Baz", rest);
}

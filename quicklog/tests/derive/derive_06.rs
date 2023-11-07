// Testing enums with no fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

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

    let (foo_store, rest) = foo.encode(&mut buf);
    let (bar_store, rest) = bar.encode(rest);
    let (baz_store, _) = baz.encode(rest);

    assert_eq!(
        "Foo Bar Baz".to_string(),
        format!("{} {} {}", foo_store, bar_store, baz_store)
    )
}

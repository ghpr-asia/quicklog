// Testing structs with &str types of different lifetimes.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

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

    let (store, _) = s.encode(&mut buf);
    assert_eq!(
        format!("{} {}", s.some_str, s.another_str),
        format!("{}", store)
    )
}

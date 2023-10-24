// Testing structs with a simple primitive field.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[derive(Serialize)]
struct TestStruct {
    size: usize,
}

fn main() {
    let s = TestStruct { size: 0 };
    let mut buf = [0; 128];

    let (store, _) = s.encode(&mut buf);
    assert_eq!(format!("{}", s.size), format!("{}", store))
}

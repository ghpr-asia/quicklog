// Testing structs with multiple primitive fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[derive(Serialize)]
struct TestStruct {
    a: usize,
    b: i32,
    c: u32,
}

fn main() {
    let s = TestStruct {
        a: 0,
        b: -999,
        c: 2,
    };
    let mut buf = [0; 128];

    let (store, _) = s.encode(&mut buf);
    assert_eq!(format!("{} {} {}", s.a, s.b, s.c), format!("{}", store))
}

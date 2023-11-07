// Testing structs with no fields.
use quicklog::serialize::Serialize as _;
use quicklog::Serialize;

#[derive(Serialize)]
struct TestStruct;

fn main() {
    let a = TestStruct;
    let mut buf = [0; 128];
    let (store, _) = a.encode(&mut buf);
    assert_eq!("TestStruct".to_string(), format!("{}", store))
}

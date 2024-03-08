// Testing structs with no fields (should be a no-op).
use quicklog::Serialize;

#[derive(Serialize)]
struct TestStruct;

fn main() {}

use quicklog::info;

#[path = "../common/mod.rs"]
mod common;

use common::SerializeStruct;

fn main() {
    let s1 = SerializeStruct {
        symbol: "Hello world 1".to_string(),
    };
    info!(?s1, "struct does not implement Debug");
}

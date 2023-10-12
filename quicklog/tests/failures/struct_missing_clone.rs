use quicklog::info;

#[path = "../common/mod.rs"]
mod common;

use common::SimpleStruct;

fn main() {
    let s1 = SimpleStruct {
        some_str: "Hello world 1",
    };
    info!(s1, "struct does not implement Clone");
}

use quicklog::info;

#[path = "../common/mod.rs"]
mod common;

use common::Something;

fn main() {
    let s1 = Something {
        some_str: "Hello world 1",
    };
    info!(^s1, "struct does not implement Serialize");
}

use quicklog::info;

#[path = "../common/mod.rs"]
mod common;

use common::Something;

fn main() {
    let s1 = Something {
        some_str: "Hello world 1",
    };
    info!(a = ?s1, "prefixed arg after fmt str: {b}", ?s1);
}

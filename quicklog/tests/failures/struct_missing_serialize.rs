use quicklog::info;

#[path = "../common/mod.rs"]
mod common;

use common::A;

fn main() {
    let s1 = A {
        price: 999,
        symbol: "Hello world 1",
        exch_id: 65,
    };
    info!(s1, "struct does not implement Serialize");
}

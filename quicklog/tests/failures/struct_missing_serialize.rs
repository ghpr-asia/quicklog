use quicklog::info;

#[path = "../common/mod.rs"]
mod common;

struct NoSerialize;

struct NoSerializeStruct {
    something: NoSerialize,
}

fn main() {
    let s = NoSerializeStruct {
        something: NoSerialize,
    };
    info!(s, "struct does not implement Serialize");
}

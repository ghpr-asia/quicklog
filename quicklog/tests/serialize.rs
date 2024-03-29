use quicklog::info;

use common::{BigStruct, SerializeStruct};

mod common;

fn main() {
    setup!();

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };
    let bs = BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    };

    assert_message_equal!(info!(^s), "s=Hello");
    assert_message_equal!(info!(^s, "with fmt string:"), "with fmt string: s=Hello");
    assert_message_equal!(
        info!(^s, ^bs, "s, bs:"),
        format!(
            "s, bs: s=Hello bs=vec: {:?}, str: {}",
            vec![1; 100],
            "The quick brown fox jumps over the lazy dog"
        )
    );
}

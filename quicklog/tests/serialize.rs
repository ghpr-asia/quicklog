use quicklog::info;

use crate::common::{BigStruct, SerializeStruct};

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

    assert_message_equal!(info!("s: {} {}", ^s, ^s), "s: Hello Hello");
    assert_message_equal!(
        info!("bs: {}", ^bs),
        format!(
            "bs: vec: {:?}, str: {}",
            vec![1; 100],
            "The quick brown fox jumps over the lazy dog"
        )
    );
}

use quicklog::info;

use crate::common::Something;

mod common;

fn main() {
    setup!();

    let s1 = Something {
        some_str: "Hello world 1",
    };
    let s2 = Something {
        some_str: "Hello world 2",
    };
    let some_str = "hello world";

    assert_message_equal!(
        info!("display {}; eager debug {}; eager display {}, eager display inner field {}", some_str, ?s2, %s1, %s1.some_str),
        format!(
            "display {}; eager debug {:?}; eager display {}, eager display inner field {}",
            some_str, s2, s1, s1.some_str
        )
    );
    assert_message_equal!(
        info!("single eager display: {}", %s2),
        format!("single eager display: {}", s2)
    );
    assert_message_equal!(
        info!("single eager display with prefix: {}", a = %s2),
        format!("single eager display with prefix: a={}", s2)
    );
}

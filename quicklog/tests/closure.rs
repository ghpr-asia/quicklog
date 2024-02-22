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

    let f = || {
        assert_message_equal!(
            info!("Hello world {} {:?}", s1, s2),
            format!("Hello world {} {:?}", s1, s2)
        );
    };

    f();
}

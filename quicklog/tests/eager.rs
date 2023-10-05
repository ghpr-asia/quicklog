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
        info!(eager.debug = ?s2, eager.display = %s1, eager.display.inner.field = %s1.some_str, "display {};", some_str),
        format!(
            "display {}; eager.debug={:?} eager.display={} eager.display.inner.field={}",
            some_str, s2, s1, s1.some_str
        )
    );
    assert_message_equal!(
        info!(%s2, "single eager display with prefix:"),
        format!("single eager display with prefix: s2={}", s2)
    );
    assert_message_equal!(
        info!(a = %s2, "single eager display with prefix and name:"),
        format!("single eager display with prefix and name: a={}", s2)
    );
}

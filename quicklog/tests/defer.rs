use quicklog::{commit, flush, info_defer};

use common::{SerializeStruct, Something};

mod common;

fn main() {
    setup!();

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };

    let s1 = Something {
        some_str: "Hello world 1",
    };

    info_defer!("hello world");
    assert_no_messages!();
    commit!();
    assert_message_equal!((), "hello world");

    info_defer!(s, "Serialize:");
    assert_no_messages!();
    commit!();
    assert_message_equal!((), "Serialize: s=Hello");

    info_defer!(debug = ?s1, display = %s1, serialize = s, "Mix:");
    assert_no_messages!();
    commit!();
    assert_message_equal!(
        (),
        format!("Mix: debug={:?} display={} serialize=Hello", s1, s1)
    );

    // Batch defer
    info_defer!("hello world 2");
    info_defer!(s, "Serialize 2:");
    info_defer!(debug = ?s1, display = %s1, serialize = s, "Mix 2:");
    assert_no_messages!();
    commit!();
    flush!();
    assert_messages!(
        "hello world 2",
        "Serialize 2: s=Hello",
        format!("Mix 2: debug={:?} display={} serialize=Hello", s1, s1).as_str()
    );
}

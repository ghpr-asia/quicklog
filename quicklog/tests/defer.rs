use quicklog::{commit, commit_on_scope_end, info_defer};

use common::{SerializeStruct, Something};

mod common;

enum FooError {
    Foo,
}

/// Tests that `commit_on_scope_end` commits a write even if the function exits early.
fn err_out() -> Result<(), FooError> {
    let s = SerializeStruct {
        symbol: String::from("Hello 2"),
    };
    commit_on_scope_end!();

    info_defer!("This should be visible after this function: {:^}", s);

    if 5 < 10 {
        return Err(FooError::Foo);
    }

    // A call to commit here is unreachable!
    // commit!();
    Ok(())
}

#[test]
fn macros_deferred() {
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
    assert_message_equal!("hello world");

    info_defer!(s, "Serialize:");
    assert_no_messages!();
    commit!();
    assert_message_equal!("Serialize: s=Hello");

    info_defer!(debug = ?s1, display = %s1, serialize = s, "Mix:");
    assert_no_messages!();
    commit!();
    assert_message_equal!(format!(
        "Mix: debug={:?} display={} serialize=Hello",
        s1, s1
    ));

    // Deferred commit
    _ = err_out();
    assert_message_equal!("This should be visible after this function: Hello 2");

    // Batch defer
    info_defer!("hello world 2");
    info_defer!(s, "Serialize 2:");
    info_defer!(debug = ?s1, display = %s1, serialize = s, "Mix 2:");
    assert_no_messages!();
    commit!();
    flush_all!();
    assert_messages!(
        "hello world 2",
        "Serialize 2: s=Hello",
        format!("Mix 2: debug={:?} display={} serialize=Hello", s1, s1).as_str()
    );
}

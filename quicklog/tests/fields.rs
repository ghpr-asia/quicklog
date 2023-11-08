use quicklog::info;

use common::{NestedSomething, Something};

mod common;

fn main() {
    setup!();

    let s1 = Something {
        some_str: "Hello world 1",
    };
    let s2 = Something {
        some_str: "Hello world 2",
    };
    let s3 = Something {
        some_str: "Hello world 3",
    };
    let s3_clone = s3.clone();
    let nested = NestedSomething {
        thing: Something {
            some_str: "hello nested",
        },
    };

    assert_message_equal!(
        info!("log one attr {}", nested.thing.some_str),
        format!("log one attr {}", nested.thing.some_str)
    );
    assert_message_equal!(
        info!("hello world {} {:?}", s1.some_str, s2.some_str),
        format!("hello world {} {:?}", s1.some_str, s2.some_str)
    );

    assert_message_equal!(
        info!(some.inner.field = s3, "inner field name with serialize"),
        format!(
            "inner field name with serialize some.inner.field=Something {{ some_str: {} }}",
            s3_clone.some_str,
        )
    );
    assert_message_equal!(
            info!(
                ?s1,
                borrow_s2_field = %s2,
                some_inner_field.inner.field.inner.arg = "hello world",
                "no name field, non-nested field, nested field:"
            ),
            format!("no name field, non-nested field, nested field: s1={:?} borrow_s2_field={} some_inner_field.inner.field.inner.arg=hello world", s1, &s2)
    );
    assert_message_equal!(
        info!(
            reuse.debug = ?s1,
            some_inner_field.some.field.included = "hello world",
            able.to.reuse.s2.borrow = s2,
            "reuse debug, nested field, able to reuse after serialize:"
        ),
        format!("reuse debug, nested field, able to reuse after serialize: reuse.debug={:?} some_inner_field.some.field.included=hello world able.to.reuse.s2.borrow=Something {{ some_str: {} }}", s1, s2.some_str)
    );
}

use quicklog::info;

use crate::common::{NestedSomething, Something};

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
        info!("pass by ref {}", some_struct.field1.innerfield.inner = &s1),
        format!("pass by ref some_struct.field1.innerfield.inner={}", &s1)
    );
    assert_message_equal!(
        info!("pass by move {}", some.inner.field = s3),
        format!("pass by move some.inner.field={}", s3_clone)
    );
    assert_message_equal!(
            info!(
                "non-nested field: {}, nested field: {}, pure lit: {}",
                borrow_s2_field = %s2,
                some_inner_field.inner.field.inner.arg = "hello world",
                "pure lit arg" = "another lit arg"
            ),
            format!("non-nested field: borrow_s2_field={}, nested field: some_inner_field.inner.field.inner.arg=hello world, pure lit: pure lit arg=another lit arg", &s2)
        );
    assert_message_equal!(
            info!(
                "pure lit: {}, reuse debug: {}, nested field: {}, able to reuse after pass by ref: {}",
                "pure lit arg" = "another lit arg",
                "able to reuse s1" = ?s1,
                some_inner_field.some.field.included = "hello world",
                able.to.reuse.s2.borrow = &s2
            ),
            format!("pure lit: pure lit arg=another lit arg, reuse debug: able to reuse s1={:?}, nested field: some_inner_field.some.field.included=hello world, able to reuse after pass by ref: able.to.reuse.s2.borrow={}", s1, &s2)
        );
}

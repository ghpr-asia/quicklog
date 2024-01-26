use quicklog::info;

use common::{BigStruct, SerializeStruct};

mod common;

#[allow(unused)]
#[derive(Copy, Clone, Debug)]
struct CopyStruct {
    a: usize,
    b: i32,
    c: u8,
}

#[test]
fn macros_serialize() {
    setup!();

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };
    let bs = BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    };
    let c = CopyStruct { a: 0, b: -1, c: 99 };

    assert_message_equal!(info!(s), "s=Hello");
    assert_message_equal!(info!(s, "with fmt string:"), "with fmt string: s=Hello");
    assert_message_equal!(
        info!(c, "copy automatically impl serialize"),
        format!("copy automatically impl serialize c={:?}", c)
    );
    assert_message_equal!(
        info!(s, "with fmt string and arg: {:^}", s),
        "with fmt string and arg: Hello s=Hello"
    );
    assert_message_equal!(
        info!(a = ?bs, s, "with fmt string and eager + serialize prefixed: {:^} {:^}", s, s),
        format!(
            "with fmt string and eager + serialize prefixed: Hello Hello a={:?} s=Hello",
            bs
        )
    );

    assert_message_equal!(
        info!(a = ?bs, b = c, "with auto copy serialize and eager: {:^} {:^}", c, c),
        format!(
            "with auto copy serialize and eager: {:?} {:?} a={:?} b={:?}",
            c, c, bs, c
        )
    );

    // NOTE: if taking reference to stack Copy-able variable, must declare new
    // variable *outside of the macro*.
    // Also, must flush before end of current function scope.
    let d = &&c;
    assert_message_equal!(
        info!(a = ?bs, b = d, "with auto copy serialize and eager: {:^} {:^}", c, c),
        format!(
            "with auto copy serialize and eager: {:?} {:?} a={:?} b={:?}",
            c, c, bs, d
        )
    );

    assert_message_equal!(
        info!(s, bs, "s, bs:"),
        format!(
            "s, bs: s=Hello bs=vec: {:?}, str: {}",
            vec![1; 100],
            "The quick brown fox jumps over the lazy dog"
        )
    );

    assert_message_equal!(
        info!(a = &s, b = &&bs, c = &&&s, "serialize with ref types:"),
        format!(
            "serialize with ref types: a=Hello b=vec: {:?}, str: {} c=Hello",
            vec![1; 100],
            "The quick brown fox jumps over the lazy dog"
        )
    );
}

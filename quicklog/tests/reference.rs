use quicklog::info;

use common::Something;

mod common;

fn main() {
    setup!();

    let s1 = Something {
        some_str: "Hello world 1",
    };
    let s2 = Something {
        some_str: "Hello world 2",
    };
    assert_message_equal!(
        info!("log single ref: {}", &s1),
        format!("log single ref: {}", &s1)
    );
    assert_message_equal!(
        info!("log multi ref: {} {:?}", &s1, &s2),
        format!("log multi ref: {} {:?}", &s1, &s2)
    );

    let s1_boxed = Box::new(s1);
    let s2_boxed = Box::new(s2);

    assert_message_equal!(
        info!("log single box ref {}", s1_boxed.as_ref()),
        format!("log single box ref {}", s1_boxed.as_ref())
    );
    assert_message_equal!(
        info!(
            "log multi box ref {} {:?}",
            s1_boxed.as_ref(),
            s2_boxed.as_ref()
        ),
        format!(
            "log multi box ref {} {:?}",
            s1_boxed.as_ref(),
            s2_boxed.as_ref()
        )
    );
}

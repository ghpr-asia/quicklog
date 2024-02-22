use quicklog::info;

use crate::common::Something;

mod common;

fn log_ref_and_move(s1: Something, s2r: &Something) {
    info!("Hello world {} {:?}", s1, s2r);
}

fn main() {
    setup!();

    let s1 = Something {
        some_str: "Hello world 1",
    };
    let s1_clone = s1.clone();
    let s2 = Something {
        some_str: "Hello world 2",
    };
    let s3 = Something {
        some_str: "Hello world 3",
    };
    let s4 = Something {
        some_str: "Hello world 4",
    };

    assert_message_equal!(
        info!("log multi move {} {:?}", s1, s2),
        format!("log multi move {} {:?}", s1, s2)
    );
    assert_message_equal!(
        log_ref_and_move(s1, &s2),
        format!("Hello world {} {:?}", s1_clone, &s2)
    );

    assert_message_equal!(
        info!("log single move {}", s3),
        format!("log single move {}", s3)
    );

    assert_message_equal!(
        info!("ref: {:?}, move: {}", &s2, s3),
        format!("ref: {:?}, move: {}", &s2, s3)
    );
    assert_message_equal!(info!("single ref: {}", &s2), format!("single ref: {}", &s2));
    assert_message_equal!(info!("single move: {}", s4), format!("single move: {}", s4));
}

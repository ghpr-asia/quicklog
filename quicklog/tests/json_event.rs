use quicklog::{event, info};

use common::{json::*, SerializeStruct, TestFormatter};

mod common;

#[test]
fn macros_json_event() {
    // Check `event` forces JSON formatting
    setup!(formatter = TestFormatter);

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };
    // Normal formatting
    assert_message_equal!(
        info!(s, "with fmt string and arg: {:^}", s),
        "with fmt string and arg: Hello s=Hello"
    );

    // JSON formatting, using `event`
    assert_json_fields!(event!(s), construct_json_fields(&[("s", "Hello")]));
    assert_json_no_message!(event!(s));
    assert_json_fields!(
        event!(s, "with fmt string and arg: {:^}", s),
        construct_json_fields(&[
            ("message", "with fmt string and arg: Hello"),
            ("s", "Hello")
        ])
    );
}

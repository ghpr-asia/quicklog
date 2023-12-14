use quicklog::{event, formatter::JsonFormatter, info, with_formatter};

use common::{json::*, SerializeStruct, Something, TestFormatter};

mod common;

fn main() {
    setup!();
    with_formatter!(JsonFormatter);

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };
    let s1 = Something {
        some_str: "Hello world 1",
    };

    assert_json_fields!(info!(s), construct_json_fields(&[("s", "Hello")]));
    assert_json_no_message!(info!(s));

    assert_json_fields!(
        info!(s, "with fmt string and arg: {:^}", s),
        construct_json_fields(&[
            ("message", "with fmt string and arg: Hello"),
            ("s", "Hello")
        ])
    );

    let s1_debug = format!("{:?}", s1);
    let s1_display = format!("{}", s1);
    assert_json_fields!(
        info!(eager.debug = ?s1, eager.display = %s1, eager.display.inner.field = %s1.some_str, "display {};", s1.some_str),
        construct_json_fields(&[
            ("message", "display Hello world 1;"),
            ("eager.debug", &s1_debug),
            ("eager.display", &s1_display),
            ("eager.display.inner.field", s1.some_str)
        ])
    );

    // Check `event` forces JSON formatting
    with_formatter!(TestFormatter);
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

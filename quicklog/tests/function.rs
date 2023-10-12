use quicklog::info;

use common::{Something, A};

mod common;

fn log_multi_ref_helper(thing: &Something, thing2: &Something) {
    info!("log multi ref {} {:?}", thing, thing2);
}

fn log_ref_helper(thing: &Something) {
    info!("log single ref: {}", thing)
}

fn main() {
    setup!();

    let s1 = Something {
        some_str: "Hello world 1",
    };
    let s2 = Something {
        some_str: "Hello world 2",
    };

    assert_message_equal!(log_ref_helper(&s1), format!("log single ref: {}", &s1));
    assert_message_equal!(
        log_multi_ref_helper(&s2, &s1),
        format!("log multi ref {} {:?}", &s2, &s1)
    );

    let a = A {
        price: 1_521_523,
        symbol: "SomeSymbol",
        exch_id: 642_153_768,
    };

    assert_message_equal!(
        info!(
            price = a.get_price(),
            symbol = ?a.get_symbol(),
            exch_id = a.get_exch_id(),
            "A:"
        ),
        format!(
            "A: price={} symbol=\"{}\" exch_id={:?}",
            a.get_price(),
            a.get_symbol(),
            a.get_exch_id()
        )
    );
    assert_message_equal!(
        info!("single call {}", a.get_price()),
        format!("single call {}", a.get_price())
    );
}

use quicklog::{debug, error, info, trace, warn};

mod common;

fn main() {
    setup!();

    assert_message_with_level_equal!(
        trace!("Hello world {}", "Another"),
        format!("[TRC]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        debug!("Hello world {}", "Another"),
        format!("[DBG]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        info!("Hello world {}", "Another"),
        format!("[INF]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        warn!("Hello world {}", "Another"),
        format!("[WRN]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        error!("Hello world {}", "Another"),
        format!("[ERR]\tHello world {}", "Another")
    );
}

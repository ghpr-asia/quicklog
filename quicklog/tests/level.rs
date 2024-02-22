use quicklog::{debug, error, info, trace, warn};

mod common;

fn main() {
    setup!();

    assert_message_with_level_equal!(
        trace!("Hello world {}", "Another"),
        format!("[TRACE]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        debug!("Hello world {}", "Another"),
        format!("[DEBUG]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        info!("Hello world {}", "Another"),
        format!("[INFO]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        warn!("Hello world {}", "Another"),
        format!("[WARN]\tHello world {}", "Another")
    );
    assert_message_with_level_equal!(
        error!("Hello world {}", "Another"),
        format!("[ERROR]\tHello world {}", "Another")
    );
}

use quicklog::{debug, error, info, trace, warn};

use common::Something;

mod common;

fn main() {
    setup!();

    // Literal
    {
        assert_message_equal!(trace!("hello world"), "hello world");
        assert_message_equal!(debug!("hello world"), "hello world");
        assert_message_equal!(info!("hello world"), "hello world");
        assert_message_equal!(warn!("hello world"), "hello world");
        assert_message_equal!(error!("hello world"), "hello world");
    }

    // Level
    {
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

    // Mixture
    {
        let s1 = Something {
            some_str: "Hello world 1",
        };
        let s2 = Something {
            some_str: "Hello world 2",
        };
        let s1_boxed = Box::new(s1.clone());
        let s2_boxed = Box::new(s2.clone());
        let some_str = "hello world";

        let now = std::time::Instant::now();
        let then = now + std::time::Duration::from_millis(30);
        let state = 42;

        // Single prefixed field without name
        assert_message_equal!(
            info!(
            %s2_boxed,
            state,
            some_time = then.duration_since(now).as_secs_f64(),
            "single eager display with prefix:"
            ),
            format!(
                "single eager display with prefix: s2_boxed={} state=42 some_time=0.03",
                s2_boxed
            )
        );
        // Single prefixed field with name
        assert_message_equal!(
            info!(a = %&s2, "single eager display with prefix and name:"),
            format!("single eager display with prefix and name: a={}", s2)
        );

        // Single prefixed field without name, with a single format arg
        assert_message_equal!(
            info!(%s2, "single eager display with prefix and arg: {}", s2),
            format!("single eager display with prefix and arg: {} s2={}", s2, s2)
        );
        // Single prefixed field with name, with a single format arg
        assert_message_equal!(
            info!(a = %s2, "single eager display with prefix and name and arg: {}", s2),
            format!(
                "single eager display with prefix and name and arg: {} a={}",
                s2, s2
            )
        );

        // Single prefixed field without name, with multiple format args
        assert_message_equal!(
            info!(%s2, "single eager display with prefix and args: {} {:?}", s2, s1),
            format!(
                "single eager display with prefix and args: {} {:?} s2={}",
                s2, s1, s2,
            )
        );
        // Single prefixed field with name, with multiple format args
        assert_message_equal!(
            info!(a = %s2, "single eager display with prefix and name and args: {} {:?}", s2, s1),
            format!(
                "single eager display with prefix and name and args: {} {:?} a={}",
                s2, s1, s2,
            )
        );

        // Multiple prefixed fields + format arg(s)
        assert_message_equal!(
            info!(?s2, eager.display = %s1.some_str(), eager.display.inner.field = %s1_boxed.as_ref(), "display {};", some_str),
            format!(
                "display {}; s2={:?} eager.display={} eager.display.inner.field={}",
                some_str,
                s2,
                s1.some_str(),
                s1_boxed.as_ref(),
            )
        );
        assert_message_equal!(
            info!(?s2, eager.display = %&s1, eager.display.inner.field = %s1.some_str, "display multiple {} {:?};", some_str, s1),
            format!(
                "display multiple {} {:?}; s2={:?} eager.display={} eager.display.inner.field={}",
                some_str, s1, s2, &s1, s1.some_str
            )
        );

        // All variables still usable
        let _ = some_str.clone();
        let _ = s1.clone();
        let _ = s2.clone();
        let _ = s1_boxed.clone();
        let _ = s2_boxed.clone();
    }
}

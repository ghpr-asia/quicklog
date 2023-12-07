use quicklog::info;

use common::{BigStruct, SerializeStruct};

mod common;

fn main() {
    setup!();

    let s = SerializeStruct {
        symbol: String::from("Hello"),
    };
    let bs = BigStruct {
        vec: [1; 100],
        some: "The quick brown fox jumps over the lazy dog",
    };

    // === Non-serialize ===
    // Implicit capture
    assert_message_equal!(info!("BigStruct: {bs:?}"), format!("BigStruct: {:?}", bs));
    // Explicit capture
    assert_message_equal!(
        info!("BigStruct: {bs:?}", bs = bs),
        format!("BigStruct: {:?}", bs)
    );
    // Both implicit and explicit
    assert_message_equal!(
        info!("BigStruct: {a:?} {bs:?}", a = bs),
        format!("BigStruct: {:?} {:?}", bs, bs)
    );
    // Multiple explicit
    assert_message_equal!(
        info!("BigStruct: {bs:?} {bs:?}", bs = bs),
        format!("BigStruct: {:?} {:?}", bs, bs)
    );

    // === Serialize ===
    // Implicit capture
    assert_message_equal!(
        info!("SerializeStruct: {s:^}"),
        "SerializeStruct: Hello".to_string()
    );
    // Explicit capture
    assert_message_equal!(
        info!("SerializeStruct: {s:^}", s = s),
        "SerializeStruct: Hello".to_string()
    );
    // Both implicit and explicit
    assert_message_equal!(
        info!("SerializeStruct: {a:^} {s:^}", a = s),
        "SerializeStruct: Hello Hello".to_string()
    );
    // Multiple explicit
    assert_message_equal!(
        info!("SerializeStruct: {s:^} {s:^}", s = s),
        "SerializeStruct: Hello Hello".to_string()
    );

    // Mixing positional and named params
    assert_message_equal!(
        info!("SerializeStruct: {ser:^} {:^} {:^} {ser:^}", s, ser = s),
        "SerializeStruct: Hello Hello Hello Hello".to_string()
    );

    // With prefix
    assert_message_equal!(
        info!(debug_impl = ?bs, "BigStruct: {bs:?}"),
        format!("BigStruct: {:?} debug_impl={:?}", bs, bs)
    );
    assert_message_equal!(
        info!(debug_impl = ?bs, "BigStruct: {bs:?}", bs = bs),
        format!("BigStruct: {:?} debug_impl={:?}", bs, bs)
    );

    assert_message_equal!(
        info!(debug_impl = ?bs, "SerializeStruct: {s:^}"),
        format!("SerializeStruct: Hello debug_impl={:?}", bs)
    );
    assert_message_equal!(
        info!(debug_impl = ?bs, "SerializeStruct: {some_struct:^}", some_struct = s),
        format!("SerializeStruct: Hello debug_impl={:?}", bs)
    );
}

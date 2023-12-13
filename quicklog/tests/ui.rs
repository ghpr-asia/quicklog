mod common;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/failures/*.rs");
    t.pass("tests/literal.rs");
    t.pass("tests/level.rs");
    t.pass("tests/closure.rs");
    t.pass("tests/reference.rs");
    t.pass("tests/move.rs");
    t.pass("tests/function.rs");
    t.pass("tests/eager.rs");
    t.pass("tests/fields.rs");
    t.pass("tests/serialize.rs");
    t.pass("tests/defer.rs");
    t.pass("tests/capture.rs");
    t.pass("tests/json.rs");
}

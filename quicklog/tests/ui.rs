mod common;

#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/failures/*.rs");
    t.pass("tests/basic.rs");
    t.pass("tests/serialize.rs");
    t.pass("tests/defer.rs");
    t.pass("tests/capture.rs");
    t.pass("tests/json.rs");
}

mod common;

#[test]
fn macro_failures() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/failures/*.rs");
}

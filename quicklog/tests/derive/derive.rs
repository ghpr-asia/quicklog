#[test]
fn derive() {
    let t = trybuild::TestCases::new();
    t.pass("tests/derive/derive_00.rs");
    t.pass("tests/derive/derive_01.rs");
    t.pass("tests/derive/derive_02.rs");
    t.pass("tests/derive/derive_03.rs");
    t.pass("tests/derive/derive_04.rs");
}

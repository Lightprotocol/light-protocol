#![cfg(feature = "mut")]
/// UI tests using trybuild to validate error messages and compilation behavior

#[test]
fn ui_tests() {
    let t = trybuild::TestCases::new();

    // Test cases that should compile successfully
    t.pass("tests/derive/ui/pass/*.rs");

    // Test cases that should fail compilation with helpful error messages
    //t.compile_fail("tests/derive/ui/fail/*.rs");
}

#[test]
fn ui_tests_zerocopy_mut() {
    let t = trybuild::TestCases::new();

    // Test ZeroCopyMut-specific cases
    t.pass("tests/derive/ui/pass_mut/*.rs");
    t.compile_fail("tests/derive/ui/fail_mut/*.rs");
}

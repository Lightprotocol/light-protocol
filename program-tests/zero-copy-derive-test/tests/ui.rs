/// UI tests using trybuild to validate error messages and compilation behavior
/// These tests verify that derive macros produce helpful error messages.
/// They are skipped in CI because compiler error messages vary between platforms.

#[test]
#[ignore = "fails in ci for unrelated reasons"]
fn ui_tests() {
    let t = trybuild::TestCases::new();

    // Test cases that should compile successfully
    t.pass("tests/ui/pass/*.rs");

    // Test cases that should fail compilation with helpful error messages
    t.compile_fail("tests/ui/fail/*.rs");
}

#[test]
#[ignore = "fails in ci for unrelated reasons"]
fn ui_tests_zerocopy_mut() {
    let t = trybuild::TestCases::new();

    // Test ZeroCopyMut-specific cases
    t.pass("tests/ui/pass_mut/*.rs");
    t.compile_fail("tests/ui/fail_mut/*.rs");
}

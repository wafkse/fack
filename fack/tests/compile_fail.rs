#[test]
fn invalid_error_declarations_fail_with_fack_diagnostics() {
    let cases = trybuild::TestCases::new();

    cases.compile_fail("tests/ui/*.rs");
}

//! Declaration expectations fixed independently of compiler capture.
use rstyle_pdg::bool_constants::{array_declaration, function_declaration};

#[test]
fn compile_time_table_declarations_keep_bounded_safe_source_contract() {
    assert!(
        function_declaration(
            "const fn build() -> [bool; 256] { let mut a = [false; 256]; a[95] = true; a }"
        )
        .is_ok()
    );
    assert!(array_declaration("const WORD: [bool; 256] = build();").is_ok());
    for source in [
        "fn build() -> [bool; 256] { [false; 256] }",
        "const unsafe fn build() -> [bool; 256] { [false; 256] }",
        "const fn build(x: bool) -> [bool; 256] { [x; 256] }",
        "const fn build<T>() -> [bool; 256] { [false; 256] }",
        "#[inline] const fn build() -> [bool; 256] { [false; 256] }",
        "const fn build() -> [bool; 256] { unsafe { [false; 256] } }",
        "const fn build() -> [bool; 256] { table!() }",
        "const fn build() -> [bool; 256] { #[allow(unused)] let a = [false; 256]; a }",
        "const fn build() -> [bool; 256] { const N: usize = 256; [false; N] }",
        "const fn build() -> [bool; 0] { [] }",
        "const fn build() -> [bool; 4097] { [false; 4097] }",
        "const fn build() -> [u8; 256] { [0; 256] }",
    ] {
        assert!(function_declaration(source).is_err(), "{source}");
    }
    for source in [
        "static WORD: [bool; 256] = build();",
        "const WORD: [bool; 256] = external::build();",
        "const WORD: [bool; 256] = build(0);",
        "const WORD: [bool; 256] = build::<u8>();",
        "const WORD: [bool; 256] = (build)();",
        "const WORD: [bool; 256] = [false; 256];",
        "const WORD: [bool; N] = build();",
        "#[cfg(test)] const WORD: [bool; 256] = build();",
    ] {
        assert!(array_declaration(source).is_err(), "{source}");
    }
}

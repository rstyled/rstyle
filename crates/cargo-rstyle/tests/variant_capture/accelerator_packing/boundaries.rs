use super::*;

#[test]
fn packing_keeps_alias_macro_effect_and_drop_boundaries_closed() {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        r#"
        type Word = u32;
        type Chain = Word;
        pub fn direct(x: Word) -> Word { x }
        pub fn chained(x: Chain) -> Chain { x }
        pub fn qualified(x: crate::Word) -> crate::Word { x }
        pub mod imported {
            use super::Word;
            pub fn alias(x: Word) -> Word { x }
        }
        #[derive(Clone, Copy, PartialEq, Debug)] pub struct Custom(u32);
        pub fn custom(a: Custom, b: Custom) { assert_eq!(a, b); }
        pub struct Bytes { data: [u8; 8] }
        pub fn hidden_write<'a>(mut a: &'a Bytes, b: &'a Bytes) -> u8 {
            assert_eq!({ a = b; 1u8 }, 1); a.data[0]
        }
        struct Guard;
        impl Drop for Guard { fn drop(&mut self) {} }
        pub fn dropping(a: &[u8]) -> u32 {
            let _guard = Guard;
            u32::from_ne_bytes(a[0..4].try_into().unwrap())
        }
        pub fn dropping_caller(a: &[u8]) -> u32 { dropping(a) }
    "#,
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        report["matching"]["diagnostics"]
    );
    for function in report["functions"].as_array().unwrap() {
        let name = function["facts"]["name"].as_str().unwrap();
        assert_eq!(
            function["outcome"],
            if name == "direct" {
                "admitted"
            } else {
                "excluded"
            },
            "{name}: {}",
            function["reasons"]
        );
    }
    check_aggregate_documents(&report);
}

#[test]
fn packing_runtime_oracle_preserves_words_snapshots_debug_activation_and_panics() {
    let directory = clone_workspace();
    let source = format!(
        "{}\n{}",
        include_str!("../../../../../eval/coverage-development/accelerator-packing.rs"),
        r#"
        #[test]
        fn independent_laws() {
            for position in 0..8 {
                for byte in 0..=255u32 {
                    let mut bytes = [0u8; 8]; bytes[position] = byte as u8;
                    let value = Accel { bytes };
                    let shift = if cfg!(target_endian = "little") { position % 4 } else { 3 - position % 4 };
                    let mut expected = [0u32; 2]; expected[position / 4] = byte * 256u32.pow(shift as u32);
                    assert_eq!(getter(&value), expected);
                    assert_eq!(saved(&value, &Accel { bytes: [255; 8] }), expected);
                    assert_eq!(selected(&value, &Accel { bytes: [255; 8] }, true), expected);
                }
            }
            assert_eq!(compare_order(0), 2);
            assert_eq!(debug_order(0), if cfg!(debug_assertions) { 2 } else { 0 });
            for n in [u32::MAX, u32::MAX - 1] {
                assert!(std::panic::catch_unwind(|| compare_order(n)).is_err());
            }
            for length in 0..4 { assert!(std::panic::catch_unwind(|| raw_caller(&[0; 4][..length])).is_err()); }
            assert_eq!(raw_caller(&[1, 0, 0, 0]), if cfg!(target_endian = "little") { 1 } else { 16777216 });
        }
    "#
    );
    let path = directory.path().join("oracle.rs");
    std::fs::write(&path, source).unwrap();
    for debug in ["yes", "no"] {
        run_oracle(directory.path(), &path, debug);
    }
}

fn run_oracle(directory: &Path, path: &Path, debug: &str) {
    let binary = directory.join(format!("oracle-{debug}"));
    let output = Command::new("rustc")
        .args([
            "--test",
            "--edition",
            "2021",
            "-C",
            "overflow-checks=yes",
            "-C",
        ])
        .arg(format!("debug-assertions={debug}"))
        .arg(path)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(Command::new(binary).status().unwrap().success());
}

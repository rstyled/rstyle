//! The slice override, its cursor and slice::iter are the reviewed authority.

#[test]
fn byte_iterator_authority_binds_pinned_slice_override_and_cursor_sources() {
    let output = std::process::Command::new("rustc")
        .args(["--print", "sysroot"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let sysroot = String::from_utf8(output.stdout).unwrap();
    let root = std::path::Path::new(sysroot.trim()).join("lib/rustlib/src/rust/library/core/src");
    let digests: std::collections::BTreeMap<String, String> = serde_json::from_str(include_str!(
        "../contracts/iterator-position-core-source-v1.json"
    ))
    .unwrap();
    assert_eq!(digests.len(), 4);
    for (path, expected) in digests {
        assert_eq!(
            rstyle_pdg::concrete_types::digest(&std::fs::read(root.join(path)).unwrap()),
            expected
        );
    }
}

mod development {
    include!("../../../eval/coverage-development/iterator-byte-position.rs");
    include!("../../../eval/coverage-development/iterator-byte-position-oracle.rs");

    #[test]
    fn compositions_preserve_first_indices_subslice_offsets_and_saved_origins() {
        let a = Accel {
            bytes: [4, 7, 9, 7, 255, 99, 99, 99],
        };
        let b = Accel {
            bytes: [0, 7, 7, 7, 7, 7, 7, 7],
        };
        let expected = [(7, Some(0)), (9, Some(1)), (255, Some(3)), (99, None)];
        for (byte, position) in expected {
            assert_eq!(first_position(a.needles(), byte), position);
            assert_eq!(position_caller(a.needles(), byte), position);
            assert_eq!(saved_position(a.needles(), byte), position.is_some());
            assert_eq!(slice_contains(a.needles(), byte), position.is_some());
            assert_eq!(caller(&a, byte), position.is_some());
            assert_eq!(saved(&a, &b, byte), position.is_some());
            assert_eq!(selected(&a, &b, true, byte), position.is_some());
            assert!(!selected(&a, &b, false, byte));
            assert_eq!(window(a.needles(), 1, 3, byte), matches!(byte, 7 | 9));
            assert!(!window(a.needles(), 2, 2, byte));
        }
        for (lo, hi) in [(3, 2), (0, 5), (5, 5)] {
            assert!(std::panic::catch_unwind(|| window(a.needles(), lo, hi, 7)).is_err());
        }
    }

    #[test]
    fn excluded_stateful_forms_are_distinct_runtime_controls() {
        let a = Accel {
            bytes: [2, 7, 9, 0, 0, 0, 0, 0],
        };
        assert!(owned(a, 7));
        assert!(mutable(&mut [7, 9], 7));
        assert!(effectful(&[7, 9], 7));
        assert!(!saved_iterator(&[7, 9], 7));
        assert!(move_capture(&[7, 9], 7));
    }
}

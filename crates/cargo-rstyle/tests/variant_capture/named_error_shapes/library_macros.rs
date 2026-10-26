//! Real Cargo composition: neighboring alloc macros retain whole-error ownership.
use super::*;

#[test]
fn pinned_library_macros_compose_with_named_error_callers() {
    let source = format!(
        "{SOURCE}\nextern crate alloc;\nextern crate std;\n\
         mod neighbor {{\n\
         use alloc::{{vec, vec::Vec, format}};\n\
         use std::thread_local;\n\
         thread_local!(static VALUE: usize = 1;);\n\
         fn build() {{ let _: Vec<u8> = vec![0; 2]; let _ = format!(\"{{}}\", 1); }}\n\
         }}"
    );
    let directory = clone_workspace();
    std::fs::write(directory.path().join("src/lib.rs"), source).unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    for name in ["arithmetic_overflow", "add", "mul"] {
        let entry = function(&report, name);
        assert_eq!(entry["outcome"], "admitted", "{}", entry["reasons"]);
        let (store, index) = shared_evidence::load(entry);
        verify_complete_pair(&store, index);
        let original = store.pair(index).unwrap();
        let mut historical = rstyle_pdg::producer::Pair {
            pdg: original.pdg.clone(),
            cfg: original.cfg.clone(),
        };
        historical.pdg.profile.version = "3.119.0".into();
        historical.cfg.profile.version = "3.119.0".into();
        assert!(
            rstyle_ccgraph::admit_produced(&historical, &store.analysis().function(index).unwrap())
                .is_err()
        );
    }
    assert_eq!(function(&report, "neighbor::build")["outcome"], "excluded");
}

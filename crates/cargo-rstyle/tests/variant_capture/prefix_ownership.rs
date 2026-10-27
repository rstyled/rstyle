//! Original local-trait ownership must retain namespace and active-source proof.
use super::*;

const BODY: &str = r#"
#[derive(Clone, Copy)] struct Span { start: usize, end: usize }
#[derive(Clone, Debug)] struct Memchr(u8);
#[cfg(debug_assertions)]
trait PrefilterI: Debug + Send + Sync + RefUnwindSafe + UnwindSafe + 'static {
    fn prefix(&self, haystack: &[u8], span: Span) -> Option<Span>;
}
#[cfg(debug_assertions)]
impl PrefilterI for Memchr {
    fn prefix(&self, haystack: &[u8], span: Span) -> Option<Span> {
        let b = *haystack.get(span.start)?;
        if self.0 == b {
            Some(Span { start: span.start, end: span.start + 1 })
        } else { None }
    }
}
"#;

const IMPORTS: &str = r#"
use core::{fmt::Debug, panic::{RefUnwindSafe, UnwindSafe}};
"#;

#[test]
fn prefix_trait_ownership_checks_core_roots_and_derive_namespaces() {
    for (setup, expected) in [
        (IMPORTS.to_string(), "admitted"),
        (
            format!("{IMPORTS}\npub use search::*; mod search {{ pub struct Other; }}"),
            "admitted",
        ),
        (
            format!(
                "{IMPORTS}\npub use search::*; mod search {{ pub type Option<T> = std::option::Option<T>; }}"
            ),
            "excluded",
        ),
        (
            format!("{IMPORTS}\ntrait Send {{}} impl Send for Memchr {{}}"),
            "excluded",
        ),
        (
            format!(
                "{IMPORTS}\nmod core {{ pub mod fmt {{ pub use std::fmt::Debug; }} pub mod panic {{ pub use std::panic::{{RefUnwindSafe, UnwindSafe}}; }} }}"
            ),
            "excluded",
        ),
        (
            "use core::panic::{RefUnwindSafe, UnwindSafe};
             use shadow::Other as Debug;
             mod shadow { pub trait Other {} }
             impl shadow::Other for Memchr {}"
                .into(),
            "excluded",
        ),
    ] {
        verify_case(&setup, expected);
    }
}

fn verify_case(setup: &str, expected: &str) {
    let directory = clone_workspace();
    std::fs::write(
        directory.path().join("src/lib.rs"),
        format!("{setup}\n{BODY}"),
    )
    .unwrap();
    let (output, report) = clones(directory.path(), &["--extraction-profile", "hir-data-v3"]);
    assert!(output.status.success(), "{}", report["capture_failures"]);
    assert_eq!(report["capture_complete"], true);
    let prefix = report["functions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["facts"]["source"]["bytes"]
                .as_str()
                .is_some_and(|source| source.starts_with("fn prefix("))
        })
        .unwrap();
    assert_eq!(prefix["outcome"], expected, "{}", prefix["reasons"]);
    if expected == "admitted" {
        let unit = super::word_classifiers::verify_admissions(&report);
        reject_inactive_trait(&unit, prefix);
    }
}

fn reject_inactive_trait(unit: &rstyle_pdg::unit_store::UnitStore, entry: &Value) {
    let analysis = unit.analysis();
    let facts = analysis.function(shared_evidence::index(entry)).unwrap();
    let pair = shared_evidence::pair(unit, entry);
    let mut inactive = facts.clone();
    assert!(
        inactive
            .build
            .cfgs
            .iter()
            .any(|cfg| cfg == "debug_assertions")
    );
    inactive.build.cfgs.retain(|cfg| cfg != "debug_assertions");
    assert!(rstyle_pdg::producer::produce(&inactive).is_err());
    assert!(rstyle_pdg::data_documents::verify(pair, &inactive).is_err());
    assert!(rstyle_ccgraph::admit_produced(pair, &inactive).is_err());
}

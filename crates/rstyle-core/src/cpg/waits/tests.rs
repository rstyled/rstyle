use super::*;

#[test]
fn every_embedded_file_reads_and_names_its_own_namespace() {
    for namespace in ["go", "py"] {
        let waits = FixedWaits::of(namespace);
        assert_eq!(waits.namespace, namespace);
        assert!(!waits.operations.is_empty(), "{namespace}");
    }
    assert!(FixedWaits::of("rust").operations.is_empty());
}

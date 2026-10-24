//! Complete export names alone cannot stand in for the entire emitted source graph.
use super::*;

#[test]
fn complete_export_roster_without_the_generated_member_graph_cannot_mint_a_receipt() {
    let fixture = generated_exports::tests::fixture();
    assert!(
        generated_exports::verify(
            generated_units::tests::projection(),
            &fixture.generated,
            &fixture.import,
        )
        .is_ok()
    );
    let (census, modules) = generated_units::tests::original_inputs();
    let mut site = fixture.import.site.clone();
    site.owner = fixture.import.site.target.clone();
    site.owner.local = true;
    site.owner.path = "convert".into();
    let producer = UnitCapture {
        identity: capture::IDENTITY.into(),
        imports: vec![],
        declarations: vec![capture::Declaration { site }],
        unsupported: vec![],
    };
    let error = verify_generated_namespace(
        GeneratedProducer {
            census: Arc::new(census),
            modules: &modules,
            namespaces: &producer,
            generated: &fixture.generated,
        },
        &fixture.import,
    )
    .err()
    .expect("Export names omit methods, trait impls, derives and constants");
    assert_eq!(
        error,
        ProofError::Contradictory("Generated unit derive attribute roster differs")
    );
}

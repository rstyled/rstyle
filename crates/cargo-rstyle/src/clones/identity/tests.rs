use super::*;
use rstyle_pdg::extraction_profile::ExtractionProfile;

fn unit(package: &str, test: bool) -> capture::Unit {
    capture::Unit {
        compiler_advisory_only: false,
        style_inputs_only: false,
        extraction_profile: ExtractionProfile::R0,
        target: capture::Target {
            package: package.into(),
            manifest_dir: format!("/{package}").into(),
            package_context: None,
            source: format!("/{package}/src/lib.rs").into(),
            name: "same_name".into(),
            kind: vec!["lib".into()],
            test: true,
        },
        test,
        invocation: vec!["--cfg".into(), "feature=\"first\"".into()],
        directory: format!("/run/{package}-{test}").into(),
        artifacts: Vec::new(),
    }
}

#[test]
fn report_handles_are_dense_qualified_labels_with_checked_bounds() {
    let first = FunctionKey::new(UnitId(7), 12).unwrap();
    assert_eq!(first.to_string(), "function:u7:f12");
    assert_eq!(
        serde_json::to_string(&first).unwrap(),
        "\"function:u7:f12\""
    );
    assert_ne!(first, FunctionKey::new(UnitId(8), 12).unwrap());
    assert_ne!(first, FunctionKey::new(UnitId(7), 13).unwrap());
    if let Some(overflow) = (u32::MAX as usize).checked_add(1) {
        assert!(FunctionKey::new(UnitId(7), overflow).is_err());
    }
}

#[test]
fn unit_assignment_preserves_context_and_ignores_completion_order() {
    let mut units = vec![unit("b", false), unit("a", true), unit("a", false)];
    let first = assign(&units)
        .unwrap()
        .into_iter()
        .map(|assignment| {
            assert!(std::ptr::eq(
                assignment.unit,
                &units[assignment.context.observed_unit]
            ));
            (
                assignment.context.id,
                assignment.unit.target.package.clone(),
                assignment.unit.test,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        first.iter().map(|entry| entry.0).collect::<Vec<_>>(),
        [UnitId(0), UnitId(1), UnitId(2)]
    );
    units.reverse();
    let reordered = assign(&units)
        .unwrap()
        .into_iter()
        .map(|assignment| {
            (
                assignment.context.id,
                assignment.unit.target.package.clone(),
                assignment.unit.test,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(first, reordered);
    let mut feature = unit("a", false);
    feature.invocation[1] = "feature=\"second\"".into();
    assert_eq!(assign(&[unit("a", false), feature]).unwrap().len(), 2);
}

#[test]
fn duplicate_and_contradictory_unit_descriptors_never_merge() {
    assert!(assign(&[unit("a", false), unit("a", false)]).is_err());
    let mut contradiction = unit("a", false);
    contradiction.target.manifest_dir = "/different-package".into();
    assert!(assign(&[unit("a", false), contradiction]).is_err());
}

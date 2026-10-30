use super::*;

fn matcher(patterns: &[&str]) -> Matcher {
    Matcher::compile(
        &patterns
            .iter()
            .map(|pattern| (*pattern).into())
            .collect::<Vec<_>>(),
    )
    .unwrap()
}

#[test]
fn workspace_relative_paths_follow_the_shared_glob_matcher() {
    let matcher = matcher(&["vendor/**", "**/generated_*.rs"]);
    for path in [
        "vendor/lib.rs",
        "src/generated_api.rs",
        "src/π/generated_api.rs",
    ] {
        let report = report_for_path(
            &format!("/workspace/{path}"),
            Path::new("/workspace"),
            &matcher,
        );
        assert!(matches!(&report, Report::Excluded { path: saved } if saved == path));
        assert!(report.excluded());
        assert!(report.complete());
    }
    let report = report_for_path("/workspace/src/lib.rs", Path::new("/workspace"), &matcher);
    assert!(matches!(&report, Report::Included { path } if path == "src/lib.rs"));
    assert!(!report.excluded());
    assert!(report.complete());
}

#[test]
fn empty_policy_includes_checked_workspace_sources() {
    let report = report_for_path(
        "/workspace/src/lib.rs",
        Path::new("/workspace"),
        &matcher(&[]),
    );
    assert!(matches!(report, Report::Included { .. }));
    assert!(!report.excluded());
    assert!(report.complete());
}

#[test]
fn outside_sources_never_match_workspace_patterns() {
    for path in ["/other/src/lib.rs", "/workspace-other/vendor/lib.rs"] {
        let report = report_for_path(path, Path::new("/workspace"), &matcher(&["**"]));
        assert!(matches!(&report, Report::NotApplicable { path: saved, .. } if saved == path));
        assert!(!report.excluded());
        assert!(report.complete());
    }
}

#[test]
fn invalid_original_or_root_paths_never_exclude() {
    for (path, root) in [
        ("src/lib.rs", "/workspace"),
        ("/workspace/src/lib.rs", "workspace"),
        ("/workspace/../other/lib.rs", "/workspace"),
        ("/workspace/./src/lib.rs", "/workspace"),
        ("/workspace/src/lib.rs", "/workspace/.."),
        ("/workspace/src/lib.rs", "/workspace/./"),
        ("/workspace", "/workspace"),
        ("", "/workspace"),
    ] {
        let report = report_for_path(path, Path::new(root), &matcher(&["**"]));
        assert!(
            matches!(report, Report::Unavailable { .. }),
            "{path} {root}"
        );
        assert!(!report.excluded());
        assert!(!report.complete());
    }
}

#[test]
fn source_availability_is_required_before_matching() {
    for source in [
        Availability::Unknown {
            reason: "budget exhausted".into(),
        },
        Availability::Unsupported {
            reason: "expanded".into(),
        },
        Availability::NotApplicable {
            reason: "foreign source".into(),
        },
    ] {
        let expected = match &source {
            Availability::Unknown { reason }
            | Availability::Unsupported { reason }
            | Availability::NotApplicable { reason } => reason,
            Availability::Known(_) => unreachable!(),
        };
        assert_eq!(unavailable_source(&source).as_ref(), Some(expected));
    }
    assert!(unavailable_source(&Availability::Known("")).is_none());
}

#[test]
fn file_paths_are_checked_lexically_without_reading_the_filesystem() {
    let report = report_for_path(
        "/does-not-exist/workspace/vendor/never-created.rs",
        Path::new("/does-not-exist/workspace"),
        &matcher(&["vendor/**"]),
    );
    assert!(report.excluded());
    assert!(report.complete());
}

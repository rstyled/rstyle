use super::*;
use crate::clones::review::density_rules::Policy;
use rstyle_core::config::SideEffectsConfig;

#[test]
fn published_baseline_roundtrips_without_overwriting_existing_destination() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("baseline.json");
    let policy = Policy::from_config(&SideEffectsConfig {
        transitive_depth: 2,
        ..SideEffectsConfig::default()
    });
    let baseline = Baseline::new(Vec::new(), policy, Vec::new(), Vec::new()).unwrap();
    publish(&path, &baseline).unwrap();
    assert_eq!(load(&path).unwrap(), baseline);

    let original = b"existing baseline must survive";
    std::fs::write(&path, original).unwrap();
    assert!(publish(&path, &baseline).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn missing_malformed_and_oversized_baselines_are_errors() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("baseline.json");
    assert!(load(&path).is_err());

    std::fs::write(&path, b"{").unwrap();
    assert!(load(&path).is_err());

    for json in [
        r#"{"schema":"rstyle.density-baseline/99","entries":[]}"#,
        r#"{"schema":"rstyle.density-baseline/1","entries":[],"trusted":true}"#,
    ] {
        std::fs::write(&path, json).unwrap();
        assert!(load(&path).is_err(), "{json}");
    }

    std::fs::write(&path, vec![b' '; MAX_BYTES + 1]).unwrap();
    let error = load(&path).err().unwrap();
    assert!(error.contains("4 MiB"), "{error}");
}

use super::*;

fn collected(metrics: &'static str) -> Collected {
    Collected {
        metrics: Some(metrics),
        ..Collected::default()
    }
}

#[test]
fn metric_versions_are_preserved_across_units_and_mixed_inputs_reject() {
    let legacy = rstyle_core::advisory::LEGACY_METRICS;
    for version in [legacy, METRICS] {
        let inputs = [collected(version), Collected::default(), collected(version)];
        assert_eq!(collected_metrics(&inputs).unwrap(), version);
    }
    assert!(collected_metrics(&[collected(legacy), collected(METRICS)]).is_err());
    assert_eq!(collected_metrics(&[]).unwrap(), METRICS);
}

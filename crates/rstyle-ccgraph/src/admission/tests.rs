use super::imported_matching_profile;
use crate::graph::MatchingProfile;
use rstyle_pdg::extraction_profile::{DATA_ID, ExtractionProfile};

#[test]
fn current_data_policy_uses_call_aware_projection_without_extending_unknown_policies() {
    let current = ExtractionProfile::DataV3.policy();
    assert_eq!(
        imported_matching_profile(&current),
        MatchingProfile::DataCallsV1
    );
    let mut other = current;
    other.id = "unregistered.data".into();
    assert_eq!(imported_matching_profile(&other), MatchingProfile::ScalarV1);
    other.id = DATA_ID.into();
    for (version, expected) in [
        ("1.0.0", MatchingProfile::DataV1),
        ("2.0.0", MatchingProfile::DataCallsV1),
        ("3.90.0", MatchingProfile::DataCallsV1),
        ("3.91.0", MatchingProfile::DataCallsV1),
        ("3.92.0", MatchingProfile::DataCallsV1),
        ("3.93.0", MatchingProfile::DataCallsV1),
        ("3.94.0", MatchingProfile::DataCallsV1),
        ("3.28.0", MatchingProfile::DataV1),
        ("3.29.0", MatchingProfile::DataV1),
        ("999.0.0", MatchingProfile::DataV1),
    ] {
        other.version = version.into();
        assert_eq!(imported_matching_profile(&other), expected, "{version}");
    }
}

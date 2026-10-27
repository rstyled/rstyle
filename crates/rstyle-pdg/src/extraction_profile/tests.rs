use super::*;

const PROFILES: [(ExtractionProfile, usize); 12] = [
    (ExtractionProfile::R0, 0),
    (ExtractionProfile::ConstV1, 1),
    (ExtractionProfile::CastsV1, 2),
    (ExtractionProfile::MatchV1, 3),
    (ExtractionProfile::GuardsV1, 4),
    (ExtractionProfile::ConditionalV1, 5),
    (ExtractionProfile::LoopsV1, 7),
    (ExtractionProfile::CallsV1, 8),
    (ExtractionProfile::HelpersV1, 9),
    (ExtractionProfile::DataV1, 10),
    (ExtractionProfile::DataV2, 11),
    (ExtractionProfile::DataV3, 159),
];

#[test]
fn capability_ids_preserve_every_historical_prefix_and_reject_unknown_names() {
    for (profile, count) in PROFILES {
        let policy = profile.policy();
        assert_eq!(profile.capabilities().len(), count);
        for name in EXPANDED_CAPABILITIES {
            let expected = profile.capabilities().contains(name);
            assert_eq!(profile.supports(name), expected, "{profile:?}: {name}");
            assert_eq!(policy_supports(&policy, name), expected);
            for unknown in [
                format!("{name} "),
                name.to_uppercase(),
                format!("{name}-new"),
            ] {
                assert!(!profile.supports(&unknown));
                assert!(!policy_supports(&policy, &unknown));
            }
        }
        assert!(!profile.supports(""));
        assert!(!profile.supports("rust.unknown-capability"));
    }
}

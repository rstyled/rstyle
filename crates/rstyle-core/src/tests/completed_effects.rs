use super::*;
use crate::facts::{Edge, EffectEvent, EffectKind};

fn config() -> Config {
    toml::from_str(
        "exclude=['generated/**']\n[side_effects]\nmin_lines=0\nmin_statements=0\ntransitive_depth=0\n",
    )
    .unwrap()
}

fn observed(name: &str, kind: EffectKind) -> FnFacts {
    let mut facts = sample(name);
    facts.effects.push(EffectEvent {
        kind,
        detail: "recorded effect".into(),
        line: 7,
    });
    facts
}

fn provided(kind: EffectKind) -> Vec<rules::Effect> {
    vec![rules::Effect {
        kind,
        label: "completed effect".into(),
        line: 7,
        via: Some("separately resolved owner".into()),
    }]
}

#[test]
fn completed_effect_retention_is_explicit_and_default_apis_stay_unretained() {
    for retention in [Retention::Report, Retention::Owners, Retention::Scopes] {
        let default = evaluate_retained(
            vec![observed("owner", EffectKind::Io)],
            &config(),
            retention,
        )
        .unwrap();
        assert!(default.effects.is_none());
        let retained = evaluate_retaining_effects(
            vec![observed("owner", EffectKind::Io)],
            &config(),
            retention,
            None,
        )
        .unwrap();
        assert_eq!(default.report, retained.report);
        assert_eq!(default.diagnostic_owners, retained.diagnostic_owners);
        let effects = retained.effects.unwrap();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects["krate::owner"][0].kind, EffectKind::Io);
    }
}

#[test]
fn retained_supplied_effect_vectors_and_original_source_buffers_are_moved() {
    let facts = sample("owner");
    let source_buffer = facts.source.as_ptr();
    let effects = provided(EffectKind::Io);
    let effect_buffer = effects.as_ptr();
    let completed = BTreeMap::from([(facts.id.clone(), effects)]);
    let evaluated =
        evaluate_retaining_effects(vec![facts], &config(), Retention::Report, Some(completed))
            .unwrap();
    assert_eq!(evaluated.facts[0].source.as_ptr(), source_buffer);
    let retained = evaluated.effects.unwrap();
    assert_eq!(retained["krate::owner"].as_ptr(), effect_buffer);
    assert_eq!(
        retained["krate::owner"][0].via.as_deref(),
        Some("separately resolved owner")
    );
}

#[test]
fn path_exclusions_select_effect_owners_and_keep_excluded_facts_in_scopes() {
    let mut excluded = observed("excluded", EffectKind::Network);
    excluded.path = "generated/file.rs".into();
    let excluded_source = excluded.source.as_ptr();
    let evaluated = evaluate_retaining_effects(
        vec![excluded, observed("kept", EffectKind::Io)],
        &config(),
        Retention::Scopes,
        None,
    )
    .unwrap();
    assert_eq!(evaluated.facts.len(), 1);
    let effects = evaluated.effects.unwrap();
    assert_eq!(effects.len(), 1);
    assert!(effects.contains_key("krate::kept"));
    assert!(!effects.contains_key("krate::excluded"));
    let scopes = evaluated.scopes.unwrap();
    assert_eq!(scopes.excluded.len(), 1);
    assert_eq!(scopes.excluded[0].source.as_ptr(), excluded_source);
    assert_eq!(scopes.suppressions, vec![Suppression::None]);
}

#[test]
fn supplied_completed_effects_are_used_unchanged_and_current_rule_weights_apply() {
    let mut policy = config();
    policy.side_effects.weights.time = 2;
    let completed = BTreeMap::from([("krate::owner".into(), provided(EffectKind::Time))]);
    let evaluated = evaluate_retaining_effects(
        vec![observed("owner", EffectKind::Unsafe)],
        &policy,
        Retention::Owners,
        Some(completed),
    )
    .unwrap();
    assert_eq!(evaluated.report.summary.effect_counts.get("time"), Some(&1));
    assert!(
        !evaluated
            .report
            .summary
            .effect_counts
            .contains_key("unsafe")
    );
    let diagnostic = evaluated
        .report
        .diagnostics
        .iter()
        .find(|row| row.rule_id == "side_effect_density")
        .unwrap();
    assert_eq!(diagnostic.actual, 2000);
    assert_eq!(evaluated.diagnostic_owners, vec![0]);
    assert_eq!(
        evaluated.effects.unwrap()["krate::owner"],
        provided(EffectKind::Time)
    );
}

#[test]
fn missing_selected_completed_owner_is_an_error_but_excluded_owner_is_not_required() {
    let missing = evaluate_retaining_effects(
        vec![sample("missing")],
        &config(),
        Retention::Report,
        Some(BTreeMap::new()),
    );
    assert!(missing.is_err());
    let mut excluded = sample("excluded");
    excluded.path = "generated/owner.rs".into();
    let completed = BTreeMap::from([("krate::kept".into(), Vec::new())]);
    let evaluated = evaluate_retaining_effects(
        vec![excluded, sample("kept")],
        &config(),
        Retention::Report,
        Some(completed),
    )
    .unwrap();
    let retained = evaluated.effects.unwrap();
    assert_eq!(retained.len(), 1);
    assert!(retained["krate::kept"].is_empty());
}

#[test]
fn freshly_computed_retained_effects_follow_configured_propagation_depth() {
    for depth in [0, 1] {
        let mut policy = config();
        policy.side_effects.transitive_depth = depth;
        let mut caller = sample("caller");
        caller.edges.push(Edge {
            callee: "krate::leaf".into(),
            line: 9,
            dynamic: false,
            args: vec![],
        });
        let evaluated = evaluate_retaining_effects(
            vec![caller, observed("leaf", EffectKind::Io)],
            &policy,
            Retention::Report,
            None,
        )
        .unwrap();
        let effects = evaluated.effects.unwrap();
        assert_eq!(effects["krate::leaf"].len(), 1);
        assert_eq!(effects["krate::caller"].len(), depth);
        if depth == 1 {
            assert_eq!(effects["krate::caller"][0].via.as_deref(), Some("leaf"));
            assert_eq!(effects["krate::caller"][0].line, 9);
        }
    }
}

#[test]
fn suppression_scopes_and_diagnostic_owners_compose_with_effect_retention() {
    let mut allowed = observed("allowed", EffectKind::Io);
    allowed.preceding_line = "// rstyle-allow(side_effect_density): reviewed boundary".into();
    let evaluated = evaluate_retaining_effects(
        vec![allowed, observed("reported", EffectKind::Io)],
        &config(),
        Retention::Scopes,
        None,
    )
    .unwrap();
    let effects = evaluated.effects.unwrap();
    assert_eq!(effects.len(), 2);
    assert_eq!(effects["krate::allowed"].len(), 1);
    assert_eq!(effects["krate::reported"].len(), 1);
    assert_eq!(evaluated.report.diagnostics.len(), 1);
    let owner = evaluated.diagnostic_owners[0];
    assert_eq!(evaluated.facts[owner].name, "reported");
    let scopes = evaluated.scopes.unwrap();
    assert_eq!(scopes.suppressions[owner], Suppression::None);
    assert!(
        scopes
            .suppressions
            .contains(&Suppression::Allow(vec!["side_effect_density".into()]))
    );
}

#[test]
fn effect_observations_reuse_selected_core_effects_without_numeric_reports() {
    let mut excluded = observed("excluded", EffectKind::Network);
    excluded.path = "generated/file.rs".into();
    let facts = vec![excluded, observed("kept", EffectKind::Io)];
    let full =
        evaluate_retaining_effects(facts.clone(), &config(), Retention::Report, None).unwrap();
    let prepared = effect_observations(facts, &config(), None).unwrap();
    assert_eq!(prepared.facts.len(), 1);
    assert_eq!(prepared.facts[0].id, "krate::kept");
    assert_eq!(prepared.effects, full.effects.unwrap());
}

#[test]
fn effect_observations_move_supplied_buffers_and_reject_missing_owner_results() {
    let facts = sample("owner");
    let source_buffer = facts.source.as_ptr();
    let effects = provided(EffectKind::Io);
    let effect_buffer = effects.as_ptr();
    let completed = BTreeMap::from([(facts.id.clone(), effects)]);
    let prepared = effect_observations(vec![facts], &config(), Some(completed)).unwrap();
    assert_eq!(prepared.facts[0].source.as_ptr(), source_buffer);
    assert_eq!(prepared.effects["krate::owner"].as_ptr(), effect_buffer);
    assert!(effect_observations(vec![sample("owner")], &config(), Some(BTreeMap::new())).is_err());
}

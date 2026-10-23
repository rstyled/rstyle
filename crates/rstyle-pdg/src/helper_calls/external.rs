//! Scalar foreign calls require an owner-backed association, never path resolution alone.
use super::*;

pub(super) fn path(node: &HirNode, resolution: &Value, callee: &dyn EvidenceFacts) -> bool {
    let Some(compiled) = callee
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
    else {
        return false;
    };
    let Some(crate_name) = compiled["crate_name"]
        .as_str()
        .filter(|name| !name.is_empty())
    else {
        return false;
    };
    let Some(path) = resolution["path"].as_str() else {
        return false;
    };
    resolution["crate"].as_str() == Some(crate_name)
        && node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.op == path)
        && qualified_path(path, crate_name, &callee.name)
}

fn qualified_path(path: &str, crate_name: &str, name: &str) -> bool {
    path == name
        || path
            .strip_prefix(crate_name)
            .and_then(|tail| tail.strip_prefix("::"))
            == Some(name)
}

pub(super) fn signature(
    node: &HirNode,
    caller: &dyn EvidenceFacts,
    resolution: &Value,
    callee: &dyn EvidenceFacts,
) -> Result<(), String> {
    let profile = crate::extraction_profile::ExtractionProfile::from_facts(caller)?;
    let compiled = &resolution["compiled_dependency"];
    let local = callee
        .build
        .frontend_options
        .get("rstyle.rust.compiled-crate")
        .ok_or("Missing compiled callee identity")?;

    let valid = profile.supports("rust.verified-cross-unit-scalar-helpers")
        && resolution["local"] == false
        && caller.external_helper(node, callee)?
        && compiled["identity"] == "rstyle.rust.compiled-crate/1"
        && compiled["identity"] == local["identity"]
        && compiled["stable_crate_id"] == local["stable_crate_id"]
        && compiled["crate_name"] == local["crate_name"]
        && compiled["instance"]["type_arguments"]
            .as_array()
            .is_some_and(Vec::is_empty)
        && compiled["instance"]["const_arguments"]
            .as_array()
            .is_some_and(Vec::is_empty)
        && scalar_signature(callee)
        && compatible(caller, callee)?;

    if !valid {
        return Err("External helper lacks checked scalar dependency authority".into());
    }
    Ok(())
}

fn scalar_signature(facts: &dyn EvidenceFacts) -> bool {
    !facts.signature.has_self
        && !facts.signature.is_async
        && !facts.signature.is_unsafe
        && !facts.signature.is_variadic
        && (facts.signature.generics.is_empty()
            || crate::scalar_instances::activation::verify(facts).unwrap_or(false))
        && facts
            .signature
            .params
            .iter()
            .all(|param| param.ty.normalized.as_deref().is_some_and(scalar))
        && facts
            .signature
            .return_ty
            .normalized
            .as_deref()
            .is_some_and(scalar)
}

fn scalar(ty: &str) -> bool {
    matches!(
        ty,
        "unit"
            | "bool"
            | "char"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
    )
}

fn compatible(caller: &dyn EvidenceFacts, callee: &dyn EvidenceFacts) -> Result<bool, String> {
    Ok(build_compatible(&caller.build, &callee.build)
        && crate::extraction_profile::ExtractionProfile::from_facts(caller)?
            == crate::extraction_profile::ExtractionProfile::from_facts(callee)?)
}

fn build_compatible(
    left: &crate::frontend::BuildFacts,
    right: &crate::frontend::BuildFacts,
) -> bool {
    known_build(left)
        && known_build(right)
        && left.rustc_commit == right.rustc_commit
        && left.rustc_version == right.rustc_version
        && left.producer_revision == right.producer_revision
        && left.target_triple == right.target_triple
        && left.data_layout == right.data_layout
        && left.opt_level == right.opt_level
        && left.overflow_checks == right.overflow_checks
        && left.debug_assertions == right.debug_assertions
        && left.panic_strategy == right.panic_strategy
        && left.pointer_width == right.pointer_width
}

fn known_build(build: &crate::frontend::BuildFacts) -> bool {
    let strings = [
        &build.rustc_commit,
        &build.rustc_version,
        &build.producer_revision,
        &build.target_triple,
        &build.data_layout,
        &build.opt_level,
        &build.panic_strategy,
    ];
    let flags = [&build.overflow_checks, &build.debug_assertions];
    strings.iter().all(|fact| {
        fact.status == crate::FactStatus::Known
            && fact.value.as_deref().is_some_and(|value| !value.is_empty())
            && fact.reason.is_none()
    }) && flags.iter().all(|fact| {
        fact.status == crate::FactStatus::Known && fact.value.is_some() && fact.reason.is_none()
    }) && build.pointer_width.status == crate::FactStatus::Known
        && build.pointer_width.value.is_some_and(|width| width > 0)
        && build.pointer_width.reason.is_none()
}

pub(super) fn summary(summary: &Summary) -> Result<(), String> {
    if summary.mutates_arguments
        || summary.memory_effects
        || summary.memory_reads.is_some()
        || summary.reference_outputs.is_some()
    {
        return Err("External scalar helper contains memory or reference effects".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualified_paths_bind_one_exact_crate_prefix_and_complete_local_path() {
        for accepted in ["leaf", "scalar_dependency::leaf"] {
            assert!(qualified_path(accepted, "scalar_dependency", "leaf"));
        }
        assert!(qualified_path(
            "scalar_dependency::util::leaf",
            "scalar_dependency",
            "util::leaf"
        ));
        for rejected in [
            "wrong::leaf",
            "renamed::leaf",
            "scalar_dependency_extra::leaf",
            "scalar_dependency::nested::leaf",
            "scalar_dependency::leaf::tail",
            "other::scalar_dependency::leaf",
        ] {
            assert!(
                !qualified_path(rejected, "scalar_dependency", "leaf"),
                "{rejected}"
            );
        }
        assert!(!qualified_path(
            "scalar_dependency::leaf",
            "scalar_dependency",
            "util::leaf"
        ));
    }

    fn build() -> crate::frontend::BuildFacts {
        use crate::frontend::{known_bool, known_count, known_str};
        crate::frontend::BuildFacts {
            producer_revision: known_str("test-producer"),
            frontend_options: Default::default(),
            rustc_version: known_str("test-rustc"),
            rustc_commit: known_str("test-commit"),
            edition: known_str("2024"),
            target_triple: known_str("test-target"),
            pointer_width: known_count(64),
            data_layout: known_str("test-layout"),
            cfgs: Vec::new(),
            features: Vec::new(),
            overflow_checks: known_bool(true),
            debug_assertions: known_bool(true),
            panic_strategy: known_str("unwind"),
            opt_level: known_str("No"),
        }
    }

    #[test]
    fn equal_missing_build_facts_never_supply_external_compatibility() {
        let complete = build();
        assert!(build_compatible(&complete, &complete));
        let mut missing = build();
        missing.rustc_commit = crate::frontend::unknown_str("not captured");
        assert!(!build_compatible(&missing, &missing));
        let mut missing = build();
        missing.debug_assertions = crate::frontend::unknown_bool("not captured");
        assert!(!build_compatible(&missing, &missing));
        let mut missing = build();
        missing.pointer_width = crate::frontend::unknown_count("not captured");
        assert!(!build_compatible(&missing, &missing));
        let mut changed = build();
        changed.panic_strategy = crate::frontend::known_str("abort");
        assert!(!build_compatible(&complete, &changed));
    }

    fn partial() -> Summary {
        Summary {
            identity: "synthetic-partial-scalar".into(),
            argument_types: vec!["u8".into()],
            result_type: "u8".into(),
            mutates_arguments: false,
            memory_effects: false,
            may_panic: true,
            always_returns: false,
            argument_evaluation: "left-to-right".into(),
            result_dependence: "verified-callee".into(),
            bounded_loop: Some(LoopTermination {
                may_diverge: true,
                positive_max_backedges: 1,
            }),
            reference_outputs: None,
            memory_reads: None,
        }
    }

    #[test]
    fn scalar_transport_preserves_partial_flow_without_claiming_totality() {
        let evidence = partial();
        summary(&evidence).unwrap();
        assert!(evidence.may_panic);
        assert!(!evidence.always_returns);
        assert!(evidence.bounded_loop.as_ref().unwrap().may_diverge);
    }

    #[test]
    fn scalar_transport_does_not_accept_reference_or_memory_summaries() {
        let mut evidence = partial();
        evidence.memory_effects = true;
        assert!(summary(&evidence).is_err());
        let mut evidence = partial();
        evidence.mutates_arguments = true;
        assert!(summary(&evidence).is_err());
        let mut evidence = partial();
        evidence.memory_reads = Some(Vec::new());
        assert!(summary(&evidence).is_err());
        let mut evidence = partial();
        evidence.reference_outputs = Some(Vec::new());
        assert!(summary(&evidence).is_err());
    }

    #[test]
    fn external_scalar_boundary_does_not_admit_nominal_or_reference_tokens() {
        for accepted in ["unit", "bool", "u8", "i32", "usize"] {
            assert!(scalar(accepted));
        }
        for rejected in ["&u8", "[u8]", "Month", "Result<u8,u8>", "!", ""] {
            assert!(!scalar(rejected));
        }
    }
}

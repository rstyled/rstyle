use crate::{Capability, FactStatus, PdgDocument, ProfileId, Provenance};

pub(super) fn check(doc: &PdgDocument) -> Result<(), String> {
    let count = &doc.function.reference_count;
    if (count.status == FactStatus::Known) != count.policy.is_some() {
        return Err("known counts require a policy; unavailable counts must not carry one".into());
    }
    crate::model::FactShape::new(count.status, count.value.is_some(), count.reason.as_deref())
        .check("count")?;
    if !crate::validate::record_policies_agree(doc) {
        return Err("record policy disagrees with document profile".into());
    }

    let [conformance] = doc.extraction_evidence.conformance.as_slice() else {
        return Err("expected one conformance assessment".into());
    };
    if conformance.profile != doc.profile {
        return Err("conformance profile disagrees with document profile".into());
    }
    Ok(())
}

pub(super) fn inherit(doc: PdgDocument) -> PdgDocument {
    let profile = &doc.profile;
    PdgDocument {
        function: interface(doc.function, profile),
        nodes: doc
            .nodes
            .into_iter()
            .map(|value| node(value, profile))
            .collect(),
        edges: doc
            .edges
            .into_iter()
            .map(|value| edge(value, profile))
            .collect(),
        memory_locations: doc
            .memory_locations
            .into_iter()
            .map(|value| crate::MemoryLocation {
                evidence: capability(value.evidence, profile),
                ..value
            })
            .collect(),
        extraction_evidence: evidence(doc.extraction_evidence, profile),
        ..doc
    }
}

fn interface(value: crate::FunctionInfo, profile: &ProfileId) -> crate::FunctionInfo {
    crate::FunctionInfo {
        interface_completeness: capability(value.interface_completeness, profile),
        reference_count: crate::CountFact {
            policy: (value.reference_count.status == FactStatus::Known).then(|| profile.clone()),
            ..value.reference_count
        },
        ..value
    }
}

fn node(value: crate::PdgNode, profile: &ProfileId) -> crate::PdgNode {
    crate::PdgNode {
        evidence: capability(value.evidence, profile),
        provenance: provenance(value.provenance, profile),
        ..value
    }
}

fn edge(value: crate::PdgEdge, profile: &ProfileId) -> crate::PdgEdge {
    crate::PdgEdge {
        policy: profile.clone(),
        evidence: capability(value.evidence, profile),
        provenance: provenance(value.provenance, profile),
        ..value
    }
}

fn evidence(value: crate::ExtractionEvidence, profile: &ProfileId) -> crate::ExtractionEvidence {
    value.map_assessments(
        |value| capability(value, profile),
        |value| crate::Conformance {
            profile: profile.clone(),
            ..value
        },
    )
}

fn capability(value: Capability, profile: &ProfileId) -> Capability {
    Capability {
        policy: Some(profile.clone()),
        ..value
    }
}

fn provenance(value: Provenance, profile: &ProfileId) -> Provenance {
    Provenance {
        policy: Some(profile.clone()),
        ..value
    }
}

//! Receipt equality is typed and exact; failed or changed evidence never gains a hit.
use super::*;
use crate::{data_library::Callee, id_reader_constants::Layout};

fn capture() -> Arc<UnitCapture> {
    Arc::new(UnitCapture {
        identity: UNIT_IDENTITY.into(),
        outcomes: vec![],
        expansions: vec![],
        emitted: vec![],
        complete: true,
        scope: "selected-local-bang-expansion-closure".into(),
    })
}

fn owner(capture: Arc<UnitCapture>) -> Owner {
    let facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    Owner {
        definition: "reader-definition".into(),
        file: "source.rs".into(),
        span: facts.fn_span,
        source: facts.source.bytes.clone(),
        build: facts.build.settings(),
        capture,
    }
}

fn definition(name: &str) -> Definition {
    Definition {
        path: name.into(),
        definition_identity: format!("definition:{name}"),
        crate_name: "reader".into(),
        stable_crate_id: "crate-id".into(),
        local: true,
    }
}

fn observation() -> Observation {
    let site = Site {
        file: "source.rs".into(),
        span: ByteSpan { start: 0, end: 1 },
    };
    let origin = Origin {
        raw: census::expansions::FileSpan {
            path: "source.rs".into(),
            digest: "xxh3-64:0000000000000000".into(),
            span: site.span,
        },
        marks: vec![],
    };
    Observation {
        identity: crate::id_reader_constants::IDENTITY.into(),
        definition: definition("ID::SIZE"),
        implementation: definition("ID::impl"),
        origin: origin.clone(),
        implementation_origin: origin,
        defining_expansion: "expansion".into(),
        owner: definition("ID"),
        owner_source: site.clone(),
        original: definition("SmallIndex::SIZE"),
        original_source: site.clone(),
        original_implementation: definition("SmallIndex::impl"),
        original_implementation_source: site.clone(),
        size_of: Callee {
            path: "std::mem::size_of".into(),
            crate_name: "core".into(),
            crate_hash: "core".into(),
            definition_hash: "size-of".into(),
            implementation: None,
        },
        size_argument: definition("SmallIndex"),
        size_argument_source: site,
        layout: Layout { size: 4, align: 4 },
        value: 4,
    }
}

#[test]
fn exact_full_observation_is_required_for_a_successful_chain_hit() {
    let mut memo = Memo::default();
    let owner = owner(capture());
    let record = observation();
    assert!(!memo.contains(&owner, &record).unwrap());
    memo.retain(owner.clone(), record.clone());
    assert!(memo.contains(&owner, &record).unwrap());
    for field in 0..7 {
        let mut changed = record.clone();
        match field {
            0 => changed.original.definition_identity.push('x'),
            1 => changed.definition.definition_identity.push('x'),
            2 => changed.origin.raw.span.end += 1,
            3 => changed.size_of.definition_hash.push('x'),
            4 => changed.layout.align = 8,
            5 => changed.value = 8,
            _ => changed.size_argument_source.file.push('x'),
        }
        assert!(!memo.contains(&owner, &changed).unwrap());
    }
}

#[test]
fn owner_build_source_and_immutable_unit_capture_cannot_be_substituted() {
    let mut memo = Memo::default();
    let owner = owner(capture());
    let record = observation();
    memo.retain(owner.clone(), record.clone());
    for field in 0..6 {
        let mut changed = owner.clone();
        match field {
            0 => changed.definition.push('x'),
            1 => changed.file.push('x'),
            2 => changed.span.end += 1,
            3 => changed.source = "fn other() {}".into(),
            4 => changed.build.cfgs.push("changed".into()),
            _ => changed.capture = capture(),
        }
        assert!(memo.contains(&changed, &record).is_err());
    }
}

#[test]
fn lookup_cannot_publish_a_failed_chain_and_successful_storage_is_bounded() {
    let mut memo = Memo::default();
    let owner = owner(capture());
    let record = observation();
    for _ in 0..3 {
        assert!(!memo.contains(&owner, &record).unwrap());
    }
    assert!(memo.owner.is_none());
    for index in 0..MAX_CHAINS + 1 {
        let mut changed = record.clone();
        changed.definition.definition_identity = index.to_string();
        memo.retain(owner.clone(), changed);
    }
    assert_eq!(memo.records.len(), MAX_CHAINS);
}

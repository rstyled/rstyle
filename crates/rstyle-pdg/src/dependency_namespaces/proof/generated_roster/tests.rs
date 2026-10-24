//! Independent identity/graph mutations; real Cargo replay supplies the full roster oracle.
use super::*;
use crate::generated_sources::capture::{ExpansionKind, Origin, UnitCapture};

fn definition(path: &str, identity: &str, local: bool) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: identity.into(),
        crate_name: if local { "time_core" } else { "core" }.into(),
        stable_crate_id: if local { "producer" } else { "core-build" }.into(),
        local,
    }
}

fn fixture() -> (Projection, UnitCapture, Definition) {
    let projection = super::super::generated_units::tests::projection();
    let root = Expansion {
        expansion: "root".into(),
        parent: None,
        kind: ExpansionKind::Bang,
        name: projection.macro_name().into(),
        macro_definition: Some(definition("convert::impl_per", "macro", true)),
        definition: Some(projection.definition().clone()),
        invocation: Origin {
            raw: projection.invocation().clone(),
            marks: vec![],
        },
        edition: "2024".into(),
        local_inner_macros: false,
    };
    let mut expansions = vec![root.clone()];
    for index in 0..8 {
        expansions.push(child(&projection, &root, index, "derive"));
        for name in ["Debug", "Clone", "Copy"] {
            expansions.push(child(&projection, &root, index, name));
        }
    }
    let mut module = definition("time_core::convert", "module", true);
    module.local = false;
    let generated = UnitCapture {
        identity: capture::UNIT_IDENTITY.into(),
        scope: "selected-local-bang-expansion-closure".into(),
        expansions,
        emitted: vec![],
        outcomes: vec![],
        complete: true,
    };
    (projection, generated, module)
}

fn child(projection: &Projection, root: &Expansion, index: usize, name: &str) -> Expansion {
    let (kind, path, role) = match name {
        "derive" => (
            ExpansionKind::Attr,
            "core::derive",
            TemplateRole::DeriveAttribute,
        ),
        "Debug" => (
            ExpansionKind::Derive,
            "core::fmt::Debug",
            TemplateRole::DeriveDebug,
        ),
        "Clone" => (
            ExpansionKind::Derive,
            "core::clone::Clone",
            TemplateRole::DeriveClone,
        ),
        _ => (
            ExpansionKind::Derive,
            "core::marker::Copy",
            TemplateRole::DeriveCopy,
        ),
    };
    Expansion {
        expansion: format!("{index}/{name}"),
        parent: Some(if name == "derive" {
            "root".into()
        } else {
            format!("{index}/derive")
        }),
        kind,
        name: name.into(),
        macro_definition: Some(definition(path, name, false)),
        definition: None,
        invocation: Origin {
            raw: projection.template(role).clone(),
            marks: vec![mark(root, Transparency::SemiOpaque)],
        },
        edition: "2024".into(),
        local_inner_macros: false,
    }
}

#[test]
fn eight_distinct_attribute_groups_have_all_twenty_four_exact_derive_children() {
    let (projection, generated, _) = fixture();
    let graph = derives::Graph::new(&projection, &generated, &generated.expansions[0]).unwrap();
    assert!(graph.contains("root"));
    for index in 0..8 {
        for name in ["derive", "Debug", "Clone", "Copy"] {
            assert!(graph.contains(&format!("{index}/{name}")));
        }
    }
}

#[test]
fn derive_identity_source_hygiene_ancestry_and_complete_roster_are_required() {
    for attack in [
        "duplicate",
        "missing",
        "core",
        "hash",
        "parent",
        "marks",
        "source",
        "detached",
    ] {
        let (projection, mut generated, _) = fixture();
        mutate_expansions(&mut generated, attack);
        assert!(
            derives::Graph::new(&projection, &generated, &generated.expansions[0]).is_err(),
            "{attack}"
        );
    }
}

fn mutate_expansions(generated: &mut UnitCapture, attack: &str) {
    match attack {
        "duplicate" => generated.expansions.push(generated.expansions[2].clone()),
        "missing" => {
            generated.expansions.remove(2);
        }
        "core" => {
            generated.expansions[2]
                .macro_definition
                .as_mut()
                .unwrap()
                .crate_name = "other".into()
        }
        "hash" => {
            generated.expansions[2]
                .macro_definition
                .as_mut()
                .unwrap()
                .definition_identity = "forged".into()
        }
        "parent" => generated.expansions[2].parent = Some("1/derive".into()),
        "marks" => {
            generated.expansions[2].invocation.marks[0].transparency = Transparency::Transparent
        }
        "source" => generated.expansions[2].invocation.raw.span.start += 1,
        _ => {
            let mut detached = generated.expansions[2].clone();
            detached.expansion = "detached".into();
            detached.parent = None;
            generated.expansions.push(detached);
        }
    }
}

fn structure(projection: &Projection, root: &Expansion, module: &Definition) -> Emitted {
    Emitted {
        definition: definition("convert::Nanosecond", "structure", true),
        parent: module.clone(),
        kind: "Struct".into(),
        origin: Origin {
            raw: projection.template(TemplateRole::Struct).clone(),
            marks: vec![mark(root, Transparency::SemiOpaque)],
        },
    }
}

#[test]
fn exact_record_consumption_requires_parent_identity_and_no_surplus() {
    let (projection, mut generated, module) = fixture();
    let module = local_module(&projection, &module).unwrap();
    generated
        .emitted
        .push(structure(&projection, &generated.expansions[0], &module));
    let graph = derives::Graph::new(&projection, &generated, &generated.expansions[0]).unwrap();
    let mut inventory = Inventory::new(&projection, &generated, &graph, &module).unwrap();
    inventory
        .take(Expected {
            kind: "Struct",
            path: "convert::Nanosecond".into(),
            parent: &module,
            source: projection.template(TemplateRole::Struct),
            mark: &mark(&generated.expansions[0], Transparency::SemiOpaque),
        })
        .unwrap();
    inventory.finish().unwrap();
    assert!(
        Inventory::new(&projection, &generated, &graph, &module)
            .unwrap()
            .finish()
            .is_err()
    );
    let mut inventory = Inventory::new(&projection, &generated, &graph, &module).unwrap();
    let mut wrong_parent = module.clone();
    wrong_parent.definition_identity = "wrong-parent".into();
    assert!(
        inventory
            .take(Expected {
                kind: "Struct",
                path: "convert::Nanosecond".into(),
                parent: &wrong_parent,
                source: projection.template(TemplateRole::Struct),
                mark: &mark(&generated.expansions[0], Transparency::SemiOpaque),
            })
            .is_err()
    );
}

#[test]
fn aliases_unknown_marks_wrong_source_and_foreign_compiler_owners_fail_closed() {
    for attack in ["identity", "marks", "source", "crate", "local", "expansion"] {
        let (projection, mut generated, module) = fixture();
        let module = local_module(&projection, &module).unwrap();
        let mut record = structure(&projection, &generated.expansions[0], &module);
        match attack {
            "identity" => generated.emitted.push(record.clone()),
            "marks" => record.origin.marks.clear(),
            "source" => record.origin.raw.digest = "unrelated-source".into(),
            "crate" => record.definition.stable_crate_id = "other-build".into(),
            "local" => record.definition.local = false,
            _ => record.origin.marks[0].expansion = "unknown-expansion".into(),
        }
        generated.emitted.push(record);
        let graph = derives::Graph::new(&projection, &generated, &generated.expansions[0]).unwrap();
        assert!(
            Inventory::new(&projection, &generated, &graph, &module).is_err(),
            "{attack}"
        );
    }
}

#[test]
fn incomplete_roster_cannot_gain_authority_from_complete_flag() {
    let (projection, generated, module) = fixture();
    assert!(generated.complete);
    assert!(verify(&projection, &generated, &generated.expansions[0], &module).is_err());
}

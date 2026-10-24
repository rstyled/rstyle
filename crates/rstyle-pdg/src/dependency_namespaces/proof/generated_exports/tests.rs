use super::super::generated_units::tests::projection;
use super::*;
use crate::generated_sources::capture::{ExpansionKind, Mark, Origin};

pub(in crate::dependency_namespaces::proof) struct Fixture {
    pub(in crate::dependency_namespaces::proof) generated: capture::UnitCapture,
    pub(in crate::dependency_namespaces::proof) import: Import,
}

fn definition(path: &str, identity: &str) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: identity.into(),
        crate_name: "time_core".into(),
        stable_crate_id: "producer".into(),
        local: true,
    }
}

fn foreign(local: &Definition) -> Definition {
    Definition {
        path: format!("time_core::{}", local.path),
        local: false,
        ..local.clone()
    }
}

pub(in crate::dependency_namespaces::proof) fn fixture() -> Fixture {
    let projection = projection();
    let module = definition("convert", "module");
    let emitted = emitted_units(&projection, &module);
    let exports = emitted
        .iter()
        .map(|item| super::super::capture::Export {
            binding: Binding::Named(item.definition.path.rsplit("::").next().unwrap().into()),
            target: foreign(&item.definition),
            target_kind: item.kind.clone(),
            public: true,
            route: vec![],
        })
        .collect();

    let import = Import {
        site: site(&projection, &module),
        compiled_dependency: serde_json::Value::Null,
        exports,
        complete: false,
    };

    Fixture {
        generated: capture::UnitCapture {
            identity: capture::UNIT_IDENTITY.into(),
            complete: true,
            scope: "selected-local-bang-expansion-closure".into(),
            emitted,
            expansions: vec![expansion(&projection)],
            outcomes: vec![],
        },
        import,
    }
}

fn emitted_units(projection: &Projection, module: &Definition) -> Vec<Emitted> {
    let mut emitted = Vec::new();
    for name in [
        "Nanosecond",
        "Microsecond",
        "Millisecond",
        "Second",
        "Minute",
        "Hour",
        "Day",
        "Week",
    ] {
        let path = format!("convert::{name}");
        let structure = definition(&path, &format!("type:{name}"));
        emitted.push(declaration(
            projection,
            structure.clone(),
            module.clone(),
            "Struct",
        ));
        emitted.push(declaration(
            projection,
            definition(&path, &format!("value:{name}")),
            structure,
            "Ctor(Struct, Const)",
        ));
    }

    emitted
}

fn declaration(
    projection: &Projection,
    definition: Definition,
    parent: Definition,
    kind: &str,
) -> Emitted {
    Emitted {
        definition,
        parent,
        kind: kind.into(),
        origin: Origin {
            raw: projection.template(TemplateRole::Struct).clone(),
            marks: vec![Mark {
                expansion: "unit-expansion".into(),
                transparency: Transparency::SemiOpaque,
            }],
        },
    }
}

fn expansion(projection: &Projection) -> Expansion {
    Expansion {
        expansion: "unit-expansion".into(),
        parent: None,
        kind: ExpansionKind::Bang,
        name: "impl_per".into(),
        macro_definition: Some(definition("convert::impl_per", "macro")),
        definition: Some(projection.definition().clone()),
        invocation: Origin {
            raw: projection.invocation().clone(),
            marks: vec![],
        },
        edition: "2024".into(),
        local_inner_macros: false,
    }
}

fn site(projection: &Projection, module: &Definition) -> Site {
    Site {
        definition: definition("{use#0}", "consumer-import"),
        owner: definition("", "consumer-root"),
        declaration: projection.invocation().clone(),
        leaf: projection.invocation().clone(),
        scope: vec![],
        path: vec!["time_core".into(), "convert".into()],
        binding: Binding::Glob,
        public: false,
        target: foreign(module),
        target_kind: "Mod".into(),
    }
}

#[test]
fn type_and_constructor_exports_have_distinct_definitions_and_retain_all_names() {
    let fixture = fixture();
    let receipt = verify(projection(), &fixture.generated, &fixture.import).unwrap();
    assert_eq!(
        receipt.names().collect::<Vec<_>>(),
        [
            "Nanosecond",
            "Microsecond",
            "Millisecond",
            "Second",
            "Minute",
            "Hour",
            "Day",
            "Week"
        ]
    );
    assert_eq!(receipt.import(), &fixture.import.site);
    assert!(!fixture.import.complete);
}

#[test]
fn source_export_identity_parent_hygiene_and_roster_changes_never_supply_permission() {
    let changes: &[fn(&mut Fixture)] = &[
        |f| {
            f.import.exports.pop();
        },
        |f| f.import.exports.push(f.import.exports[0].clone()),
        |f| f.import.exports[0].target_kind = "Ctor(Struct, Const)".into(),
        |f| f.import.exports[0].target.definition_identity.push('x'),
        |f| f.import.exports[0].target.stable_crate_id.push('x'),
        |f| f.import.exports[0].target.local = true,
        |f| f.import.exports[0].public = false,
        |f| f.import.exports[0].binding = Binding::Named("Hidden".into()),
        |f| f.generated.emitted[1].parent.definition_identity.push('x'),
        |f| f.generated.emitted[0].origin.marks[0].transparency = Transparency::Opaque,
        |f| f.generated.emitted[0].origin.raw.span.start += 1,
        |f| f.generated.emitted.push(f.generated.emitted[0].clone()),
        |f| {
            f.generated.expansions[0]
                .macro_definition
                .as_mut()
                .unwrap()
                .path = "other::impl_per".into()
        },
        |f| {
            f.generated.expansions[0]
                .macro_definition
                .as_mut()
                .unwrap()
                .stable_crate_id
                .push('x')
        },
        |f| f.generated.expansions[0].invocation.raw.span.end -= 1,
        |f| f.generated.complete = false,
    ];
    for change in changes {
        let mut fixture = fixture();
        change(&mut fixture);
        assert!(verify(projection(), &fixture.generated, &fixture.import).is_err());
    }
}

use super::*;
const SOURCE: &str = include_str!("source.rs");
const INVOCATION: &str =
    "index_type_impls!(PatternID, PatternIDError, PatternIDIter, WithPatternIDIter)";
const NEW: &str = "pub fn new(value: usize) -> Result<$name, $err>";
const BODY: &str = "{\n                        SmallIndex::new(value).map($name).map_err($err)\n                    }";
const CALL: &str = "SmallIndex::new(value)";
const MAP: &str = "SmallIndex::new(value).map($name)";
const MAP_ERR: &str = "SmallIndex::new(value).map($name).map_err($err)";

pub(crate) struct Fixture {
    pub facts: HirFnFacts,
    pub census: Census,
    pub capture: UnitCapture,
}

impl Fixture {
    pub fn new() -> Self {
        let census = Census {
            identity: CENSUS_IDENTITY.into(),
            context: crate::nominal_boundary::tests::context(),
            files: vec![UnitFile {
                path: "/sample/src/lib.rs".into(),
                bytes: SOURCE.into(),
                digest: crate::source_identity::checksum(SOURCE.as_bytes()),
            }],
            expansions: Vec::new(),
            documentation: Vec::new(),
            external_imports: Vec::new(),
        };
        let mut fixture = Self {
            census,
            facts: facts(),
            capture: UnitCapture {
                identity: UNIT_IDENTITY.into(),
                outcomes: Vec::new(),
                expansions: Vec::new(),
                emitted: Vec::new(),
                complete: true,
                scope: "selected-local-bang-expansion-closure".into(),
            },
        };
        fixture.capture.expansions.push(fixture.root());
        fixture.facts.fn_span = fixture.span(INVOCATION).span;
        fixture.capture.emitted = fixture.emitted();
        let observation = fixture.observation();
        fixture.capture.outcomes.push(Outcome::Observed {
            observation: Box::new(observation),
        });
        fixture
    }

    pub fn verify(&self) -> Result<CheckedMember, String> {
        verify(Input {
            facts: &self.facts,
            census: &self.census,
            capture: &self.capture,
        })
    }

    pub fn observation_mut(&mut self) -> &mut Observation {
        let Outcome::Observed { observation } = &mut self.capture.outcomes[0] else {
            panic!("Expected observation")
        };
        observation
    }

    pub fn span(&self, text: &str) -> FileSpan {
        let file = &self.census.files[0];
        let start = file.bytes.find(text).unwrap();
        FileSpan {
            path: file.path.clone(),
            digest: file.digest.clone(),
            span: crate::ByteSpan {
                start: start.try_into().unwrap(),
                end: (start + text.len()).try_into().unwrap(),
            },
        }
    }

    pub fn span_in(&self, text: &str, prefix: &str, value: &str) -> FileSpan {
        let mut span = self.span(text);
        span.span.start += u32::try_from(prefix.len()).unwrap();
        span.span.end = span.span.start + u32::try_from(value.len()).unwrap();
        span
    }

    fn root(&self) -> Expansion {
        Expansion {
            expansion: "PatternID-root".into(),
            parent: None,
            kind: ExpansionKind::Bang,
            name: "index_type_impls".into(),
            macro_definition: Some(definition("util::primitives::index_type_impls")),
            definition: Some(self.span("macro_rules! index_type_impls")),
            invocation: Origin {
                raw: self.span(INVOCATION),
                marks: Vec::new(),
            },
            edition: "2021".into(),
            local_inner_macros: false,
        }
    }

    fn emitted(&self) -> Vec<Emitted> {
        let implementation = definition("util::primitives::PatternID");
        let mut emitted = vec![Emitted {
            definition: implementation.clone(),
            parent: definition("util::primitives"),
            kind: "Impl { of_trait: false }".into(),
            origin: marked(self.span("impl $name")),
        }];
        for (name, header, kind) in [
            ("MAX", "pub const MAX: $name", "AssocConst"),
            ("LIMIT", "pub const LIMIT: usize", "AssocConst"),
            ("ZERO", "pub const ZERO: $name", "AssocConst"),
            ("SIZE", "pub const SIZE: usize", "AssocConst"),
            ("new", NEW, "AssocFn"),
            (
                "new_unchecked",
                "pub fn new_unchecked() -> usize",
                "AssocFn",
            ),
            ("must", "pub fn must() -> usize", "AssocFn"),
            ("as_usize", "pub fn as_usize() -> usize", "AssocFn"),
            ("as_u64", "pub fn as_u64() -> usize", "AssocFn"),
            ("as_u32", "pub fn as_u32() -> usize", "AssocFn"),
            ("as_i32", "pub fn as_i32() -> usize", "AssocFn"),
            ("one_more", "pub fn one_more() -> usize", "AssocFn"),
            (
                "from_ne_bytes",
                "pub fn from_ne_bytes() -> usize",
                "AssocFn",
            ),
            (
                "from_ne_bytes_unchecked",
                "pub fn from_ne_bytes_unchecked() -> usize",
                "AssocFn",
            ),
            ("to_ne_bytes", "pub fn to_ne_bytes() -> usize", "AssocFn"),
            ("iter", "pub fn iter() -> usize", "AssocFn"),
        ] {
            emitted.push(Emitted {
                definition: definition(&format!("util::primitives::PatternID::{name}")),
                parent: implementation.clone(),
                kind: kind.into(),
                origin: marked(self.span(header)),
            });
        }
        emitted
    }

    fn observation(&self) -> Observation {
        Observation {
            identity: IDENTITY.into(),
            owner: definition("util::primitives::PatternID::new"),
            implementation: definition("util::primitives::PatternID"),
            defining_expansion: "PatternID-root".into(),
            signature: marked(self.span(NEW)),
            body: marked(self.span(BODY)),
            nodes: self.observation_nodes(),
        }
    }

    fn observation_nodes(&self) -> Vec<NodeOrigin> {
        [
            (
                vec![0, 0],
                2,
                "pat_binding",
                self.span_in(NEW, "pub fn new(", "value"),
            ),
            (vec![1], 19, "expr_block", self.span(BODY)),
            (vec![1, 0], 4, "expr_method_call", self.span(MAP_ERR)),
            (vec![1, 0, 0], 6, "expr_method_call", self.span(MAP)),
            (vec![1, 0, 0, 0], 8, "expr_call", self.span(CALL)),
            (
                vec![1, 0, 0, 0, 0],
                13,
                "expr_path",
                self.span_in(CALL, "SmallIndex::new(", "value"),
            ),
            (
                vec![1, 0, 0, 1],
                15,
                "expr_path",
                self.span_in("map($name)", "map(", "$name"),
            ),
            (
                vec![1, 0, 1],
                17,
                "expr_path",
                self.span_in("map_err($err)", "map_err(", "$err"),
            ),
        ]
        .into_iter()
        .map(|(path, local, kind, span)| NodeOrigin {
            path,
            hir: HirRef {
                krate: 0,
                owner: 10,
                local,
            },
            kind: kind.into(),
            origin: marked(span),
        })
        .collect()
    }

    pub fn change_source(&mut self, before: &str, after: &str) {
        let file = &mut self.census.files[0];
        let start = u32::try_from(file.bytes.find(before).unwrap()).unwrap();
        let end = start + u32::try_from(before.len()).unwrap();
        let delta = i64::try_from(after.len()).unwrap() - i64::try_from(before.len()).unwrap();
        file.bytes = file.bytes.replacen(before, after, 1).into();
        file.digest = crate::source_identity::checksum(file.bytes.as_bytes());
        let update = |span: &mut FileSpan| {
            span.digest = file.digest.clone();
            if span.span.start >= end {
                span.span.start = (i64::from(span.span.start) + delta).try_into().unwrap();
            }
            if span.span.end >= end {
                span.span.end = (i64::from(span.span.end) + delta).try_into().unwrap();
            }
        };
        for expansion in &mut self.capture.expansions {
            update(expansion.definition.as_mut().unwrap());
            update(&mut expansion.invocation.raw);
        }
        for emitted in &mut self.capture.emitted {
            update(&mut emitted.origin.raw);
        }
        for outcome in &mut self.capture.outcomes {
            if let Outcome::Observed { observation } = outcome {
                update(&mut observation.signature.raw);
                update(&mut observation.body.raw);
                for node in &mut observation.nodes {
                    update(&mut node.origin.raw);
                }
            }
        }
        self.facts.fn_span = self.capture.expansions[0].invocation.raw.span;
    }
}

fn definition(path: &str) -> Definition {
    Definition {
        path: path.into(),
        definition_identity: format!("test-definition:{path}"),
        crate_name: "sample".into(),
        stable_crate_id: "test-sample-crate".into(),
        local: true,
    }
}

fn marked(raw: FileSpan) -> Origin {
    Origin {
        raw,
        marks: vec![Mark {
            expansion: "PatternID-root".into(),
            transparency: Transparency::SemiOpaque,
        }],
    }
}

fn node(local: u32, kind: &str, children: serde_json::Value) -> serde_json::Value {
    json!({ "hir":{"krate":0,"owner":10,"local":local}, "kind":kind,
        "span":{"start":0,"end":INVOCATION.len()}, "snippet":INVOCATION,
        "ty":null,"res":null,"dispatch":null,"expansion":null,"binding":null,
        "binding_mut":false,"lit":null,"param_index":null,"is_unsafe_block":false,"children":children })
}

fn facts() -> HirFnFacts {
    let mut facts: serde_json::Value = serde_json::from_str(include_str!(
        "../../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    facts["name"] = json!("util::primitives::PatternID::new");
    facts["source"] = json!({"bytes":INVOCATION,"digest":crate::source_identity::checksum(INVOCATION.as_bytes())});
    facts["body"] = node(
        19,
        "expr_block",
        json!([node(
            4,
            "expr_method_call",
            json!([
                node(
                    6,
                    "expr_method_call",
                    json!([
                        node(8, "expr_call", json!([node(13, "expr_path", json!([]))])),
                        node(15, "expr_path", json!([]))
                    ])
                ),
                node(17, "expr_path", json!([]))
            ])
        )]),
    );
    facts["signature"]["params"] = json!([{"symbol":"s:10:2","position":0,
        "name_span":{"start":0,"end":INVOCATION.len()},"ty_span":{"start":0,"end":INVOCATION.len()},
        "ty":{"normalized":"usize","observed":"usize","spelling":INVOCATION},"pat":node(2,"pat_binding",json!([]))}]);
    facts["signature"]["receiver"] = json!(null);
    facts["signature"]["has_self"] = json!(false);
    facts["signature"]["generics"] = json!([]);
    for flag in ["is_variadic", "is_async", "is_unsafe", "is_const"] {
        facts["signature"][flag] = json!(false);
    }
    facts["signature"]["abi"] = serde_json::to_value(crate::known_str("Rust")).unwrap();
    facts["build"]["frontend_options"]["rstyle.rust.compiled-crate"] = json!({"identity":"rstyle.rust.compiled-crate/1","crate_name":"sample","stable_crate_id":"test-sample-crate"});
    facts["build"]["frontend_options"]["definition_identity"] =
        json!("test-definition:util::primitives::PatternID::new");
    facts["build"]["frontend_options"]["edition"] = json!("2021");
    facts["build"]["frontend_options"]["source_file"] = json!("/sample/src/lib.rs");
    serde_json::from_value(facts).unwrap()
}

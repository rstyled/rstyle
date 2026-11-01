use crate::{HirFnFacts, unit_store::EvidenceFacts};

const SAVED_SOURCE: &str = "fn saved<'a>(a: &'a [u8], b: &'a [u8]) -> &'a [u8] { let mut selected = a; let saved = selected; selected = b; saved }";

struct ReferenceFlow {
    template: crate::HirNode,
    ty: crate::ResolvedTy,
}

impl ReferenceFlow {
    fn node(&self, local: u32, kind: &str, anchor: &str) -> crate::HirNode {
        let start = SAVED_SOURCE.find(anchor).unwrap() as u32;
        let text = if matches!(kind, "expr_path" | "pat_binding") {
            anchor
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .next()
                .unwrap()
        } else {
            anchor
        };
        crate::HirNode {
            hir: crate::HirRef {
                local,
                ..self.template.hir.clone()
            },
            kind: kind.into(),
            span: crate::ByteSpan {
                start,
                end: start + text.len() as u32,
            },
            snippet: text.into(),
            ty: Some(self.ty.clone()),
            children: Vec::new(),
            ..self.template.clone()
        }
    }

    fn read(&self, local: u32, target: u32, anchor: &str) -> crate::HirNode {
        let value = self.node(local, "expr_path", anchor);
        let res = crate::frontend::PathRes {
            kind: "local".into(),
            target: Some(crate::HirRef {
                local: target,
                ..value.hir.clone()
            }),
            description: String::new(),
        };
        crate::HirNode {
            res: Some(res),
            ..value
        }
    }

    fn parameters(&self) -> Vec<crate::ParamFact> {
        [(1, "a"), (2, "b")]
            .into_iter()
            .enumerate()
            .map(|(position, (local, name))| {
                let pat = self.node(local, "pat_binding", &format!("{name}:"));
                crate::ParamFact {
                    symbol: name.into(),
                    position: position as u32,
                    name_span: pat.span,
                    ty_span: pat.span,
                    ty: self.ty.clone(),
                    pat,
                }
            })
            .collect()
    }

    fn body(&self) -> crate::HirNode {
        let selected = crate::HirNode {
            children: vec![
                self.node(4, "pat_binding", "selected"),
                self.read(5, 1, "a;"),
            ],
            ..self.node(3, "stmt_let", "let mut selected = a;")
        };
        let saved = crate::HirNode {
            children: vec![
                self.node(7, "pat_binding", "saved ="),
                self.read(8, 4, "selected;"),
            ],
            ..self.node(6, "stmt_let", "let saved = selected;")
        };
        let reassigned = crate::HirNode {
            children: vec![self.read(10, 4, "selected = b"), self.read(11, 2, "b;")],
            ..self.node(9, "expr_assign", "selected = b")
        };

        let source = &SAVED_SOURCE[SAVED_SOURCE.find('{').unwrap()..];
        crate::HirNode {
            children: vec![selected, saved, reassigned, self.read(12, 7, "saved }")],
            ..self.node(0, "expr_block", source)
        }
    }
}

// Typed source flow needs no compiler capture or graph-admission observations.
fn saved_reference_facts() -> HirFnFacts {
    use crate::data_types::TypeToken;
    let facts: HirFnFacts = serde_json::from_str(include_str!(
        "../../../nominal_boundary/resolution/selected/function_cleanup_tests/facts.json"
    ))
    .unwrap();
    let flow = ReferenceFlow {
        template: facts.body.clone(),
        ty: crate::ResolvedTy {
            normalized: Some(
                TypeToken::Shared(Box::new(TypeToken::Slice(Box::new(TypeToken::Scalar(
                    "u8".into(),
                )))))
                .text()
                .unwrap(),
            ),
            observed: "&[u8]".into(),
            spelling: "&[u8]".into(),
        },
    };

    let bytes = crate::SourceText::from(SAVED_SOURCE);
    HirFnFacts {
        build: crate::frontend::BuildFacts {
            frontend_options: Default::default(),
            ..facts.build
        },
        source: crate::frontend::SourceFacts {
            digest: bytes.identity().into(),
            bytes,
        },
        name: "saved".into(),
        fn_span: crate::ByteSpan {
            start: 0,
            end: SAVED_SOURCE.len() as u32,
        },
        signature: crate::frontend::SignatureFacts {
            params: flow.parameters(),
            return_ty: flow.ty.clone(),
            ..facts.signature
        },
        body: flow.body(),
        ..facts
    }
}

#[test]
fn repeated_origin_queries_preserve_saved_sources_after_reassignment() {
    use crate::{data_origins::Origin, field_locations::Projection};
    let store = crate::unit_store::Builder::new(vec![saved_reference_facts()])
        .unwrap()
        .store;
    let unit_analysis = store.analysis();
    let analysis = unit_analysis.function(0).unwrap();
    let saved = analysis.body.children.last().unwrap();
    let expected = vec![Origin {
        parameter: 0,
        path: vec![Projection::SharedReferent],
    }];
    let mut edited = saved.clone();
    edited.kind = "expr_lit".into();

    let check = || {
        // The assignment RHS is b, while saved still resolves through the earlier a read.
        let replacement = &analysis.body.children[2].children[1];
        assert_eq!(
            analysis.reference_origins(replacement, &[]).unwrap(),
            vec![Origin {
                parameter: 1,
                path: vec![Projection::SharedReferent],
            }]
        );
        for _ in 0..2 {
            assert_eq!(analysis.reference_origins(saved, &[]).unwrap(), expected);
        }
        assert!(analysis.reference_origins(&edited, &[]).is_err());
        assert!(analysis.reference_origins(saved, &[0]).is_err());
        assert_eq!(analysis.reference_origins(saved, &[]).unwrap(), expected);
    };
    crate::verification_policy::strict(check);
    crate::verification_policy::generated(check);
    crate::verification_policy::strict(check);
}

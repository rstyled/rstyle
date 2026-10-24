//! Contradictions target source, types, identities and lexical inventories separately.
use super::*;

#[test]
fn source_ascii_endpoints_are_checked_after_all_recorded_snippets_are_refreshed() {
    for original in originals() {
        for (before, after) in [("b'0'", "b'1'"), ("b'9'", "b'8'")] {
            rejected(&original, |facts| {
                facts.source.bytes = facts.source.bytes.replace(before, after).into();
                refresh(&mut facts.body, &facts.source.bytes);
            });
        }
    }
}

fn refresh(node: &mut HirNode, source: &str) {
    node.snippet = source[node.span.start as usize..node.span.end as usize].into();
    for child in &mut node.children {
        refresh(child, source);
    }
}

#[test]
fn mutable_or_overloaded_matching_does_not_gain_shared_pattern_permission() {
    for original in originals() {
        rejected(&original, |facts| {
            edit(facts, "pat_slice_prefix", |observed| {
                observed.adjustments[0].kind = AdjustmentKind::OverloadedDeref;
            })
        });
        rejected(&original, |facts| {
            edit(facts, "pat_slice_prefix", |observed| {
                observed.adjustments[0].source =
                    TypeShape::Mutable(Box::new(TypeShape::Slice(Box::new(TypeShape::Byte))));
            })
        });
        rejected(&original, |facts| {
            edit(facts, "pat_binding_at", |observed| {
                let Details::Binding { binding, .. } = &mut observed.details else {
                    unreachable!();
                };
                binding.effective_mode = Some(BindingMode::Mutable);
            })
        });
    }
}

#[test]
fn changed_canonical_symbols_child_rosters_and_endpoint_values_fail() {
    let original = originals().remove(2);
    rejected(&original, |facts| {
        edit(facts, "pat_binding_at", |observed| {
            let Details::Binding { binding, .. } = &mut observed.details else {
                unreachable!();
            };
            binding.canonical.local += 1;
        })
    });
    rejected(&original, |facts| {
        edit(facts, "pat_slice_prefix", |observed| {
            let Details::Slice { prefix, .. } = &mut observed.details else {
                unreachable!();
            };
            prefix.swap(0, 1);
        })
    });
    rejected(&original, |facts| {
        edit(facts, "pat_slice_bound", |observed| {
            observed.details = Details::ByteLiteral {
                value: b'1',
                negated: false,
            };
        })
    });
}

#[test]
fn duplicated_actual_hir_identity_is_rejected_even_with_updated_receipt_roster() {
    let original = originals().remove(2);
    rejected(&original, |facts| {
        let root = find_mut(&mut facts.body, "pat_slice_prefix");
        root.children[1] = root.children[0].clone();
        let mut receipt = Observation::decode(root).unwrap();
        let Details::Slice { prefix, .. } = &mut receipt.details else {
            unreachable!();
        };
        prefix[1] = prefix[0].clone();
        root.res = Some(receipt.resolution().unwrap());
    });
}

#[test]
fn lexical_declaration_spans_scopes_duplicates_and_types_are_independently_required() {
    let original = originals().remove(0);
    rejected(&original, |facts| {
        let node = find(&facts.body, "pat_binding_at");
        let hir = node.hir.clone();
        facts
            .bindings
            .iter_mut()
            .find(|entry| entry.hir == hir)
            .unwrap()
            .scope = "scope:source:wrong-arm:0:1".into();
    });
    rejected(&original, |facts| {
        edit(facts, "pat_binding_at", |observed| {
            let Details::Binding { binding, .. } = &mut observed.details else {
                unreachable!();
            };
            binding.identifier_span.end += 1;
        })
    });
    rejected(&original, |facts| {
        let hir = find(&facts.body, "pat_binding_at").hir.clone();
        let entry = facts
            .bindings
            .iter()
            .find(|entry| entry.hir == hir)
            .unwrap()
            .clone();
        facts.bindings.push(entry);
    });
    rejected(&original, |facts| {
        find_mut(&mut facts.body, "pat_binding_at")
            .ty
            .as_mut()
            .unwrap()
            .normalized = Some("i32".into());
    });
}

#[test]
fn duplicate_original_binding_names_fail_even_with_matching_raw_inventory() {
    let original = originals().remove(2);
    let error = rejected(&original, |facts| {
        let root = find_mut(&mut facts.body, "pat_slice_prefix");
        let binder = &mut root.children[1];
        let mut receipt = Observation::decode(binder).unwrap();
        let Details::Binding { binding, .. } = &mut receipt.details else {
            unreachable!();
        };
        let span = binding.identifier_span;
        binding.name = "a".into();
        let hir = binder.hir.clone();
        binder.res = Some(receipt.resolution().unwrap());
        facts
            .bindings
            .iter_mut()
            .find(|entry| entry.hir == hir)
            .unwrap()
            .name = "a".into();
        let mut bytes = facts.source.bytes.to_string();
        bytes.replace_range(span.start as usize..span.end as usize, "a");
        facts.source.bytes = bytes.into();
        refresh(&mut facts.body, &facts.source.bytes);
    });
    assert!(error.contains("Duplicate source bindings"));
}

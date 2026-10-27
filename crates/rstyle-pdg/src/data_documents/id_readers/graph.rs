//! Independently inventory every read, saved value and conditional payload.
use super::*;
use crate::CfgEdgeKind;
use serde_json::{Value, json};

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<BTreeMap<String, Value>, String> {
    super::super::id_error_maps::verify(pair, facts)?;
    super::super::named_error_result_propagation::verify(pair, facts)?;
    crate::helper_calls::verify_documents(pair, facts)?;
    let mut graph = Graph {
        pair,
        facts,
        roles: BTreeMap::new(),
    };
    let inputs = graph.inputs(facts)?;
    let map = graph.prefix(checked, inputs)?;
    graph.continuation(facts, checked, &map)?;
    graph.effects(checked)?;

    graph.complete()
}

struct Graph<'a> {
    pair: PairView<'a>,
    facts: &'a dyn EvidenceFacts,
    roles: BTreeMap<String, Value>,
}

impl<'a> Graph<'a> {
    fn inputs(&mut self, facts: &dyn EvidenceFacts) -> Result<[&'a PdgNode; 2], String> {
        Ok([self.input(facts, 0)?, self.input(facts, 1)?])
    }

    fn prefix(
        &mut self,
        checked: &source::Checked<'_>,
        inputs: [&PdgNode; 2],
    ) -> Result<super::super::id_error_maps::Operations<'a>, String> {
        let bytes = self.bytes(checked, inputs[0])?;
        let map = super::super::id_error_maps::operations(self.pair, &checked.map)?;
        self.decoder(checked, bytes, map.receiver)?;
        self.map(checked, &map, inputs[1])?;

        Ok(map)
    }
    fn role(&mut self, node: &PdgNode, role: impl serde::Serialize) -> Result<(), String> {
        super::super::guarded_sources::insert_role(&mut self.roles, node, role)
    }
    fn empty(&mut self, node: &PdgNode) -> Result<(), String> {
        self.role(
            node,
            json!({"role":"checked-reference-free-value","origins":[]}),
        )
    }
    fn input(&mut self, facts: &dyn EvidenceFacts, position: usize) -> Result<&'a PdgNode, String> {
        let formal = &facts.signature.params[position];
        let node = serialized_operation(self.pair, &formal.pat.span, |node| {
            node.category == NodeCategory::FormalInput
        })?;
        if node.attributes.position != Some(position as u64) {
            return Err("ID reader changes a formal position".into());
        }
        self.role(
            node,
            json!({"role":"formal-shared-root","parameter":position,"binding":formal.pat.hir}),
        )?;

        Ok(node)
    }
    fn value(&self, source: &HirNode) -> Result<&'a PdgNode, String> {
        value_operation(self.pair, source)
    }
    fn saved(&self, node: &PdgNode, operands: &[&PdgNode]) -> Result<(), String> {
        verify_saved_operands(self.pair, node, operands)
    }
    fn consumes(&self, value: &PdgNode, consumers: &[&PdgNode]) -> Result<(), String> {
        super::super::value_consumers::verify(self.pair, value, consumers, "value_operand")
    }
    fn definition(&self, definition: &PdgNode, reads: &[&PdgNode]) -> Result<(), String> {
        for read in reads {
            super::super::copy_tagged_values::origins::verify_definition(
                self.pair, definition, read,
            )?;
        }
        super::super::value_consumers::verify(self.pair, definition, reads, "reaching_definition")
    }
    fn declaration(
        &mut self,
        source: &HirNode,
        binding: &HirNode,
        value: &PdgNode,
    ) -> Result<&'a PdgNode, String> {
        let node = serialized_operation(self.pair, &source.span, |node| {
            node.category == NodeCategory::Declaration
        })?;
        super::super::verify_binding_initialization(node, binding)?;
        self.saved(node, &[value])?;
        self.consumes(value, &[node])?;
        self.empty(node)?;

        Ok(node)
    }
    fn bytes(
        &mut self,
        checked: &source::Checked<'_>,
        input: &PdgNode,
    ) -> Result<&'a PdgNode, String> {
        let base = self.value(checked.view.base)?;
        let end = self.value(checked.view.end.ok_or("Missing ID prefix end")?)?;
        let window = checked.copy.children.first().ok_or("Missing ID prefix")?;
        let view = self.value(window)?;
        let copy = self.value(checked.copy)?;
        let unwrap = self.value(checked.unwrap)?;
        self.definition(input, &[base])?;
        self.saved(view, &[base, end])?;
        self.saved(copy, &[view])?;
        self.saved(unwrap, &[copy])?;
        for (value, target) in [(base, view), (end, view), (view, copy), (copy, unwrap)] {
            self.consumes(value, &[target])?;
        }
        for node in [base, view] {
            self.role(
                node,
                json!({"role":"saved-byte-prefix","root":input.id,"length":4}),
            )?;
        }
        for node in [end, copy, unwrap] {
            self.empty(node)?;
        }
        super::super::callbacks::order::sequence(self.pair, &[base, end, view, copy, unwrap])?;

        self.declaration(checked.bytes_local, checked.bytes_binding, unwrap)
    }
    fn decoder(
        &mut self,
        checked: &source::Checked<'_>,
        bytes: &PdgNode,
        decoder: &PdgNode,
    ) -> Result<(), String> {
        let read = self.value(checked.bytes_read)?;
        self.definition(bytes, &[read])?;
        self.saved(decoder, &[read])?;
        self.consumes(read, &[decoder])?;
        self.empty(read)?;
        self.empty(decoder)?;

        super::super::callbacks::order::sequence(self.pair, &[bytes, read, decoder])
    }
    fn map(
        &mut self,
        checked: &source::Checked<'_>,
        ops: &super::super::id_error_maps::Operations<'_>,
        message: &PdgNode,
    ) -> Result<(), String> {
        self.definition(message, &[ops.environment])?;
        for node in [ops.environment, ops.message_read] {
            self.role(node, json!({"role":"saved-static-message","root":message.id,"capture":ops.environment.id}))?;
        }
        for node in [
            ops.selector,
            ops.payload,
            ops.declaration,
            ops.error_read,
            ops.success_payload,
            ops.success,
        ] {
            self.empty(node)?;
        }
        for node in [ops.constructor, ops.error, ops.merge] {
            self.role(
                node,
                MappedError {
                    role: "conditional-owned-error",
                    scrutinee: &checked.decoder.hir,
                    selector: &ops.selector.id,
                    static_root: &message.id,
                    error_route: origins::message_route(self.facts, checked)?,
                    ok_origins: [],
                    error_guard: "err",
                },
            )?;
        }

        Ok(())
    }
    fn continuation(
        &mut self,
        facts: &dyn EvidenceFacts,
        checked: &source::Checked<'_>,
        map: &super::super::id_error_maps::Operations<'_>,
    ) -> Result<(), String> {
        let [selector, success, early] = self.propagation(facts, checked, map)?;
        let declaration = self.declaration(checked.id_local, checked.id_binding, success)?;
        let tail = self.tail(checked, declaration)?;
        let guards = super::super::conditional_payloads::Guards::new(self.pair)?;
        for node in [declaration, tail[0], tail[1], tail[2], tail[3], tail[4]] {
            guards.guarded(selector, node, CfgEdgeKind::False)?;
        }
        super::super::callbacks::order::sequence(
            self.pair,
            &[
                map.merge,
                selector,
                success,
                declaration,
                tail[0],
                tail[1],
                tail[2],
                tail[3],
                tail[4],
            ],
        )?;
        super::super::result_propagation::return_to_exit(self.pair, early)?;

        super::super::result_propagation::return_to_exit(self.pair, tail[4])
    }
    fn propagation(
        &mut self,
        facts: &dyn EvidenceFacts,
        checked: &source::Checked<'_>,
        map: &super::super::id_error_maps::Operations<'_>,
    ) -> Result<[&'a PdgNode; 3], String> {
        let constructs = checked.propagation.mode.constructs();
        let selector = typed_operation(self.pair, checked.question, constructs[0], "bool")?;
        let error = typed_operation(
            self.pair,
            checked.question,
            constructs[1],
            &checked.propagation.output_token,
        )?;
        let success = typed_operation(
            self.pair,
            checked.question,
            constructs[2],
            &checked.propagation.payload_token,
        )?;
        let early =
            serialized_operation(self.pair, &checked.propagation.early_return.span, |node| {
                node.category == NodeCategory::Return
            })?;
        self.empty(selector)?;
        self.empty(success)?;
        for node in [error, early] {
            self.role(
                node,
                PropagatedError {
                    role: "propagated-owned-error",
                    saved_result: &map.merge.id,
                    selector: &selector.id,
                    requires: "err",
                    static_parameter: 1,
                    error_route: origins::message_route(facts, checked)?,
                },
            )?;
        }

        Ok([selector, success, early])
    }
    fn tail(
        &mut self,
        checked: &source::Checked<'_>,
        declaration: &PdgNode,
    ) -> Result<[&'a PdgNode; 5], String> {
        let read = self.value(checked.id_read)?;
        let size = self.value(checked.size)?;
        let tuple = self.value(checked.tuple)?;
        let success = self.value(checked.success)?;
        let returned = serialized_operation(self.pair, &checked.success.span, |node| {
            node.category == NodeCategory::Return
        })?;
        self.definition(declaration, &[read])?;
        self.saved(tuple, &[read, size])?;
        self.saved(success, &[tuple])?;
        self.saved(returned, &[success])?;
        for (value, target) in [
            (read, tuple),
            (size, tuple),
            (tuple, success),
            (success, returned),
        ] {
            self.consumes(value, &[target])?;
        }
        self.consumes(returned, &[])?;
        for node in [read, size, tuple, success, returned] {
            self.empty(node)?;
        }

        Ok([read, size, tuple, success, returned])
    }
    fn effects(&self, checked: &source::Checked<'_>) -> Result<(), String> {
        let declarations = [
            checked.bytes_local.span,
            checked.id_local.span,
            checked.map.binding.span,
        ];
        for node in &self.pair.pdg.nodes {
            let declared = node.category == NodeCategory::Declaration
                && node.source_span.as_ref().is_some_and(|span| {
                    declarations.iter().any(|expected| {
                        span.start == u64::from(expected.start)
                            && span.end == u64::from(expected.end)
                    })
                });
            if node.annotations.contains_key(crate::data_places::KEY)
                || !declared
                    && (!node.attributes.writes.is_empty() || node.attributes.write_form.is_some())
                || node
                    .attributes
                    .reads
                    .iter()
                    .any(|read| read.location_id.is_some())
                    && node.source_span.as_ref().is_none_or(|span| {
                        span.start != u64::from(checked.copy.span.start)
                            || span.end != u64::from(checked.copy.span.end)
                    })
            {
                return Err("ID reader invents storage writes or additional memory reads".into());
            }
        }
        if self.pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Partial {
            return Err("ID reader must preserve prefix and unwrap panic behavior".into());
        }
        Ok(())
    }
    fn complete(self) -> Result<BTreeMap<String, Value>, String> {
        super::super::guarded_sources::complete_roles(
            self.pair,
            self.roles,
            "ID reader contains omitted, duplicate or unaccounted operations",
        )
    }
}

#[derive(serde::Serialize)]
struct MappedError<'a> {
    role: &'static str,
    scrutinee: &'a crate::HirRef,
    selector: &'a str,
    static_root: &'a str,
    error_route: Vec<crate::copy_tagged_values::origins::Component>,
    ok_origins: [Value; 0],
    error_guard: &'static str,
}

#[derive(serde::Serialize)]
struct PropagatedError<'a> {
    role: &'static str,
    saved_result: &'a str,
    selector: &'a str,
    requires: &'static str,
    static_parameter: u32,
    error_route: Vec<crate::copy_tagged_values::origins::Component>,
}

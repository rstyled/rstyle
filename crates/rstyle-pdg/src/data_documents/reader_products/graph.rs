//! Complete saved-value graph, consumers, scalar computations and disjoint exits.
use super::super::wire_validators::{
    graph as shared, origins as shared_origins, source as shared_source,
};
use super::*;

pub(super) fn verify(
    pair: PairView<'_>,
    facts: &dyn EvidenceFacts,
    checked: &source::Checked<'_>,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    super::super::named_error_result_propagation::verify(pair, facts)?;
    super::super::scalar_tuple_bindings::verify(pair, facts)?;
    crate::helper_calls::verify_documents(pair, facts)?;

    let input = serialized_operation(pair, &facts.signature.params[0].pat.span, |node| {
        node.category == NodeCategory::FormalInput
    })?;
    if input.attributes.position != Some(0) {
        return Err("Product reader input changes parameter position".into());
    }
    let reader = shared::reader_operations(pair, &checked.reader)?;
    shared::literal(reader.message, &checked.reader.literal)?;

    let mut context = Expressions {
        pair,
        facts,
        roles: initial_roles(facts, &checked.reader, &reader, input)?,
        sequence: vec![reader.tuple],
        reads: Vec::new(),
    };
    let bindings = context.tuple_bindings(checked)?;
    let product = context.product(checked)?;
    let tail = context.tail(checked)?;

    let ops = Operations {
        input,
        reader,
        bindings,
        product,
        tail,
    };
    context.dependencies(&ops)?;
    context.control(&ops)?;
    effects(
        pair,
        &[
            ops.bindings.declarations[0],
            ops.bindings.declarations[1],
            ops.product.local,
        ],
        ops.reader.call,
    )?;

    context.complete()
}

fn initial_roles(
    facts: &dyn EvidenceFacts,
    source: &shared_source::Reader<'_>,
    reader: &shared::Reader<'_>,
    input: &PdgNode,
) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let mut roles = shared_origins::reader_roles(facts, source, reader, input)?;
    origins::empty(&mut roles, reader.selector)?;
    origins::empty(&mut roles, reader.tuple)?;
    Ok(roles)
}

struct Bindings<'a> {
    components: Vec<&'a PdgNode>,
    declarations: Vec<&'a PdgNode>,
}
struct Product<'a> {
    local: &'a PdgNode,
    reads: Vec<&'a PdgNode>,
}
struct Tail<'a> {
    returned: &'a PdgNode,
    product_read: &'a PdgNode,
    count_read: &'a PdgNode,
}
struct Operations<'a> {
    input: &'a PdgNode,
    reader: shared::Reader<'a>,
    bindings: Bindings<'a>,
    product: Product<'a>,
    tail: Tail<'a>,
}

struct Expressions<'a, 'b> {
    pair: PairView<'a>,
    facts: &'b dyn EvidenceFacts,
    roles: BTreeMap<String, serde_json::Value>,
    sequence: Vec<&'a PdgNode>,
    reads: Vec<&'a PdgNode>,
}
impl<'a> Expressions<'a, '_> {
    fn record(&mut self, node: &'a PdgNode) -> Result<(), String> {
        self.sequence.push(node);
        origins::empty(&mut self.roles, node)
    }

    fn tuple_bindings(&mut self, checked: &source::Checked<'_>) -> Result<Bindings<'a>, String> {
        let pairs = checked
            .binding
            .entries
            .iter()
            .map(|entry| {
                let selected = typed_operation(
                    self.pair,
                    entry.pattern,
                    crate::scalar_tuple_bindings::SELECTOR,
                    entry.token,
                )?;
                let declared = typed_operation(
                    self.pair,
                    entry.pattern,
                    crate::scalar_tuple_bindings::DECLARATION,
                    "unit",
                )?;
                super::super::value_consumers::verify(
                    self.pair,
                    selected,
                    &[declared],
                    "value_operand",
                )?;

                self.record(selected)?;
                self.record(declared)?;
                Ok((selected, declared))
            })
            .collect::<Result<Vec<_>, String>>()?;
        let (components, declarations) = pairs.into_iter().unzip();
        Ok(Bindings {
            components,
            declarations,
        })
    }

    fn product(&mut self, checked: &source::Checked<'_>) -> Result<Product<'a>, String> {
        let product = self.expression(checked.product)?;
        let local = serialized_operation(self.pair, &checked.local.span, |node| {
            node.category == NodeCategory::Declaration
        })?;
        super::super::verify_binding_initialization(local, checked.pattern)?;

        verify_saved_operands(self.pair, local, &[product])?;
        super::super::value_consumers::verify(self.pair, product, &[local], "value_operand")?;
        self.record(local)?;
        Ok(Product {
            local,
            reads: std::mem::take(&mut self.reads),
        })
    }

    fn tail(&mut self, checked: &source::Checked<'_>) -> Result<Tail<'a>, String> {
        let success = self.expression(checked.success)?;
        let returned = serialized_operation(self.pair, &checked.success.span, |node| {
            node.category == NodeCategory::Return
        })?;
        verify_saved_operands(self.pair, returned, &[success])?;

        super::super::value_consumers::verify(self.pair, success, &[returned], "value_operand")?;
        super::super::value_consumers::verify(self.pair, returned, &[], "value_operand")?;
        self.record(returned)?;

        let product_read = value_operation(self.pair, checked.product_read)?;
        let count_read = value_operation(self.pair, checked.count_read)?;
        if self.reads != [product_read, count_read] {
            return Err("Product reader changes complete saved local reads".into());
        }
        Ok(Tail {
            returned,
            product_read,
            count_read,
        })
    }

    fn dependencies(&self, ops: &Operations<'_>) -> Result<(), String> {
        if ops.product.reads.len() != 3 {
            return Err("Product reader changes complete saved local reads".into());
        }
        for (definition, reads) in [
            (ops.input, vec![ops.reader.slice]),
            (ops.bindings.declarations[0], ops.product.reads.clone()),
            (ops.bindings.declarations[1], vec![ops.tail.count_read]),
            (ops.product.local, vec![ops.tail.product_read]),
        ] {
            for read in &reads {
                super::super::copy_tagged_values::origins::verify_definition(
                    self.pair, definition, read,
                )?;
            }
            super::super::value_consumers::verify(
                self.pair,
                definition,
                &reads,
                "reaching_definition",
            )?;
        }

        super::super::value_consumers::verify(
            self.pair,
            ops.reader.tuple,
            &ops.bindings.components,
            "value_operand",
        )?;
        verify_saved_operands(
            self.pair,
            ops.reader.call,
            &[ops.reader.slice, ops.reader.message],
        )?;
        for actual in [ops.reader.slice, ops.reader.message] {
            super::super::value_consumers::verify(
                self.pair,
                actual,
                &[ops.reader.call],
                "value_operand",
            )?;
        }
        Ok(())
    }

    fn control(&self, ops: &Operations<'_>) -> Result<(), String> {
        let guards = super::super::conditional_payloads::Guards::new(self.pair)?;
        for node in self.sequence.iter().skip(1) {
            guards.guarded(ops.reader.selector, node, crate::CfgEdgeKind::False)?;
        }

        super::super::callbacks::order::sequence(
            self.pair,
            &[
                ops.reader.slice,
                ops.reader.message,
                ops.reader.call,
                ops.reader.selector,
            ],
        )?;
        super::super::callbacks::order::sequence(self.pair, &self.sequence)?;
        for node in [ops.reader.returned, ops.tail.returned] {
            super::super::result_propagation::return_to_exit(self.pair, node)?;
        }
        Ok(())
    }

    fn complete(self) -> Result<BTreeMap<String, serde_json::Value>, String> {
        super::super::guarded_sources::complete_roles(
            self.pair,
            self.roles,
            "Reader product contains omitted, duplicate or unaccounted operations",
        )
    }

    fn expression(&mut self, source: &HirNode) -> Result<&'a PdgNode, String> {
        let mut children = Vec::new();
        for child in &source.children {
            children.push(self.expression(child)?);
        }

        let node = value_operation(self.pair, source)?;
        if node.original_kind.known_value() != Some(source.kind.as_str())
            || node.attributes.write_form.is_some()
            || node.attributes.callee.is_some()
            || node.attributes.call_form.is_some()
            || !node.attributes.writes.is_empty()
            || !node.attributes.defines.is_empty()
        {
            return Err("Reader product expression changes source kind or invents effects".into());
        }

        if source.kind == "expr_path" {
            if node.category != NodeCategory::Read || !children.is_empty() {
                return Err("Product local read changes its operation".into());
            }
            self.reads.push(node);
        } else {
            if !node.attributes.reads.is_empty() || !node.attributes.symbol_ids.is_empty() {
                return Err("Product computation invents local or memory reads".into());
            }
            self.semantics(node, source)?;
        }

        self.operands(node, source, &children)?;
        self.record(node)?;
        Ok(node)
    }

    fn operands(
        &self,
        node: &PdgNode,
        source: &HirNode,
        children: &[&PdgNode],
    ) -> Result<(), String> {
        if source.kind == "expr_path" {
            if self
                .pair
                .pdg
                .edges
                .iter()
                .any(|edge| edge.target == node.id && edge.subkind == "value_operand")
            {
                return Err("Product saved local read invents value operands".into());
            }
        } else {
            verify_saved_operands(self.pair, node, children)?;
        }
        for child in children {
            super::super::value_consumers::verify(self.pair, child, &[node], "value_operand")?;
        }
        Ok(())
    }

    fn semantics(&self, node: &PdgNode, source: &HirNode) -> Result<(), String> {
        if node.category != NodeCategory::Computation {
            return Err("Product computation changes category".into());
        }

        let operator = node
            .attributes
            .operator
            .as_ref()
            .and_then(|value| value.known_value());
        let literal = node
            .attributes
            .literal
            .as_ref()
            .and_then(|value| value.known_value());
        let construct = node.attributes.construct.as_deref();

        let valid = match source.kind.as_str() {
            "expr_binary" => {
                operator
                    == source
                        .dispatch
                        .as_ref()
                        .map(|dispatch| dispatch.op.as_str())
                    && literal.is_none()
                    && construct.is_none()
            }
            "expr_lit" => {
                literal == source.lit.as_deref() && operator.is_none() && construct.is_none()
            }
            "expr_struct" => {
                construct == Some("data.struct")
                    && operator.is_none()
                    && literal
                        == Some(crate::data_structs::descriptor(source, self.facts)?.as_str())
            }
            "expr_tup" => construct.is_none() && operator.is_none() && literal.is_none(),
            "expr_call" => {
                let checked = crate::concrete_results::verify_constructor(source, self.facts)?;
                construct == Some(checked.construct) && operator.is_none() && literal.is_none()
            }
            _ => false,
        };
        if !valid {
            return Err("Product graph changes exact builtin or aggregate semantics".into());
        }
        Ok(())
    }
}

fn effects(pair: PairView<'_>, declarations: &[&PdgNode], reader: &PdgNode) -> Result<(), String> {
    if pair.cfg.extraction_evidence.status != crate::ExtractionStatus::Partial {
        return Err("Product reader must preserve its actual native read/panic boundary".into());
    }
    for node in &pair.pdg.nodes {
        if node.annotations.contains_key(crate::data_places::KEY)
            || !declarations
                .iter()
                .any(|declaration| declaration.id == node.id)
                && (!node.attributes.writes.is_empty() || node.attributes.write_form.is_some())
            || node.id != reader.id
                && node
                    .attributes
                    .reads
                    .iter()
                    .any(|read| read.location_id.is_some())
        {
            return Err("Product reader invents places, writes or extra memory reads".into());
        }
    }
    Ok(())
}

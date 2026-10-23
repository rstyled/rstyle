//! Resolve formal origins by following source reaching RHS operations. A visited
//! (operation, component) cuts cycles; all terminal formal paths are still visited.

use super::flow::{OwnedIndex, Value};
use crate::{HirNode, data_origins::Origin, data_types::TypeToken, unit_store::EvidenceFacts};
use std::collections::BTreeSet;

pub(super) fn selected(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    let index = OwnedIndex::new(facts)?;
    selected_with(node, selectors, facts, &index)
}

pub(super) fn selected_with(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
    index: &OwnedIndex,
) -> Result<Vec<Origin>, String> {
    let resolver = Resolver { index, facts };
    let roots = resolver.source(node, selectors, &mut BTreeSet::new())?;
    Ok(roots
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

struct Resolver<'a> {
    index: &'a OwnedIndex,
    facts: &'a dyn EvidenceFacts,
}

type Visiting = BTreeSet<(u32, Vec<usize>)>;

impl Resolver<'_> {
    fn source(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        if crate::extraction_profile::facts_support(self.facts, "rust.checked-borrowed-result-flow")
            && selectors.is_empty()
            && !crate::data_origins::shared(&crate::data_types::parse(
                crate::data_aggregates::token(node)?,
            )?)
        {
            return Ok(Vec::new());
        }
        let key = (node.hir.local, selectors.to_vec());
        if !visiting.insert(key.clone()) {
            return Ok(Vec::new());
        }

        // Each query unions terminal origins. A previously explored component
        // need not be expanded again, even when several paths reach it.
        self.operation(node, selectors, visiting)
    }

    fn operation(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        if crate::borrowed_result_propagation::candidate(node, self.facts)
            || crate::option_propagation::candidate(node, self.facts)
        {
            return self.propagated(node, selectors, visiting);
        }
        if crate::concrete_results::constructor_candidate(node, self.facts) {
            return self.result_constructor(node, selectors, visiting);
        }

        let handler: Handler<'_> = match node.kind.as_str() {
            "expr_path" => Self::read,
            "expr_tup" | "expr_struct" => Self::constructed,
            "expr_field" if crate::data_value_fields::candidate(node, self.facts) => {
                Self::projection
            }
            "expr_field" | "expr_unary" | "expr_addr_of" | "pat_binding" => Self::terminal,
            "expr_block" | "expr_ret" => Self::tail,
            "expr_if" => Self::alternatives,
            "expr_call" | "expr_method_call"
                if selectors.is_empty()
                    || crate::helper_component_outputs::call_used(node, self.facts) =>
            {
                Self::call
            }
            _ => return Err("Unsupported reference rebinding source operation".into()),
        };
        handler(self, node, selectors, visiting)
    }

    fn propagated(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        if !selectors.is_empty() {
            return Err("Propagation cannot select unrelated payload components".into());
        }
        let operand = if crate::borrowed_result_propagation::candidate(node, self.facts) {
            crate::borrowed_result_propagation::verify(node, self.facts)?.operand
        } else {
            crate::option_propagation::verify(node, self.facts)?.operand
        };

        crate::data_origins::propagation_origins(self.source(operand, &[], visiting)?)
    }

    fn result_constructor(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        let checked = crate::concrete_results::verify_constructor(node, self.facts)?;
        match checked.construct {
            "data.result-ok" => self.source(checked.payload, selectors, visiting),
            "data.result-err" if selectors.is_empty() => Ok(Vec::new()),
            _ => Err("Result failure cannot select a successful payload component".into()),
        }
    }

    fn projection(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        let path: Vec<_> = [crate::data_value_fields::index(node, self.facts)?]
            .into_iter()
            .chain(selectors.iter().copied())
            .collect();
        self.source(&node.children[0], &path, visiting)
    }

    fn terminal(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        match node.kind.as_str() {
            "expr_field" | "expr_unary"
                if crate::field_locations::derived::candidate(node, self.facts) =>
            {
                crate::field_locations::derived::selected_with(
                    node,
                    selectors,
                    self.facts,
                    &mut |base| self.source(base, &[], visiting),
                )
            }
            "expr_field" => {
                crate::data_component_origins::select_origins(node, selectors, self.facts)
            }
            "expr_addr_of"
                if selectors.is_empty() && crate::data_subslices::borrowed(node, self.facts) =>
            {
                let checked = crate::data_subslices::verify_borrow(node, self.facts)?;
                if checked.array_length.is_some() {
                    Ok(vec![crate::data_places::field_origin(
                        checked.base,
                        self.facts,
                    )?])
                } else {
                    self.source(checked.base, selectors, visiting)
                }
            }
            "expr_addr_of" if selectors.is_empty() => {
                Ok(vec![crate::data_places::borrow_origin(node, self.facts)?])
            }
            "pat_binding" => Ok(Vec::new()),
            _ => Err("Unsupported reference flow terminal".into()),
        }
    }

    fn tail(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        self.source(
            node.children.last().ok_or("Missing reference flow value")?,
            selectors,
            visiting,
        )
    }

    fn alternatives(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        let [_, left, right] = node.children.as_slice() else {
            return Err("Reference flow value needs both alternatives".into());
        };
        let mut roots = self.source(left, selectors, visiting)?;
        roots.extend(self.source(right, selectors, visiting)?);
        Ok(roots)
    }

    fn read(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        let ty = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
        if crate::borrowed_result_types::uses_result(&ty) {
            return Err(
                "Result formal/storage reference flow requires guarded move evidence".into(),
            );
        }

        let definitions = self
            .index
            .reads
            .get(&node.hir.local)
            .filter(|definitions| !definitions.is_empty())
            .ok_or("Reference read has no source reaching definition")?;

        let mut roots = Vec::new();
        for definition in definitions {
            match self.index.value(definition, self.facts)? {
                Value::Parameter(parameter) => {
                    roots.extend(formal_origins(parameter, selectors)?);
                }
                Value::Expression(value) => {
                    if crate::data_aggregates::token(value)? != crate::data_aggregates::token(node)?
                    {
                        return Err("Reference source definition/read type mismatch".into());
                    }
                    roots.extend(self.source(value, selectors, visiting)?);
                }
            }
        }
        Ok(roots)
    }

    fn constructed(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        if !selectors.is_empty() {
            let (operand, rest) =
                crate::data_component_origins::operand(node, selectors, self.facts)?;
            return self.source(
                node.children
                    .get(operand)
                    .ok_or("Missing reference component operand")?,
                rest,
                visiting,
            );
        }

        let ty = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
        let fields: Vec<_> = match &ty {
            TypeToken::Tuple(elements) => elements.iter().collect(),
            TypeToken::Struct { fields, .. } => fields.iter().map(|(_, ty)| ty).collect(),
            _ => return Err("Reference construction requires tuple or struct components".into()),
        };
        let mut roots = Vec::new();
        for (position, _) in fields
            .iter()
            .enumerate()
            .filter(|(_, ty)| crate::data_origins::shared(ty))
        {
            roots.extend(self.constructed(node, &[position], visiting)?);
        }
        Ok(roots)
    }

    fn call(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        if !crate::helper_component_outputs::call_used(node, self.facts)
            && let Some(origins) = self.library_call(node, visiting)
        {
            return origins;
        }
        self.helper_call(node, selectors, visiting)
    }

    fn library_call(
        &self,
        node: &HirNode,
        visiting: &mut Visiting,
    ) -> Option<Result<Vec<Origin>, String>> {
        match crate::reference_instances::activation::operation(node, self.facts) {
            Ok(Some(operation)) => {
                return Some((|| {
                    let roots = self.source(operation.receiver, &[], visiting)?;
                    if roots != [operation.origin] {
                        return Err(
                            "Checked AsRef flow operand loses its exact shared formal origin"
                                .into(),
                        );
                    }
                    Ok(roots)
                })());
            }
            Err(error) => return Some(Err(error)),
            Ok(None) => {}
        }
        if let Some(receiver) = checked_receiver(node, self.facts) {
            return Some((|| {
                let (receiver, element) = receiver?;
                let roots = self.source(receiver, &[], visiting)?;
                Ok(if element {
                    crate::checked_options::project(roots)
                } else {
                    roots
                })
            })());
        }
        crate::data_bytes::source_origins(node, self.facts, |operand| {
            self.source(operand, &[], visiting)
        })
    }

    fn helper_call(
        &self,
        node: &HirNode,
        selectors: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        if crate::helper_component_outputs::call_used(node, self.facts) {
            return crate::helper_component_outputs::map(node, selectors, self.facts, |position| {
                let actual = node
                    .children
                    .get(position as usize)
                    .ok_or("Missing selected helper actual")?;
                self.source(actual, &[], visiting)
            });
        }
        crate::helper_references::map_outputs(node, self.facts, |position| {
            let actual = node
                .children
                .get(position as usize)
                .ok_or("Missing returned reference argument")?;
            self.source(actual, &[], visiting)
        })
    }
}

fn formal_origins(
    parameter: &crate::ParamFact,
    selectors: &[usize],
) -> Result<Vec<Origin>, String> {
    let ty = crate::data_types::parse(
        parameter
            .ty
            .normalized
            .as_deref()
            .ok_or("Missing reference formal type")?,
    )?;
    crate::data_component_origins::selected(&ty, selectors, parameter.position, Vec::new())
}

fn checked_receiver<'a>(
    node: &'a HirNode,
    facts: &dyn EvidenceFacts,
) -> Option<Result<(&'a HirNode, bool), String>> {
    if crate::checked_slice_views::candidate(node, facts) {
        return Some(
            crate::checked_slice_views::verify(node, facts)
                .map(|checked| (checked.receiver, false)),
        );
    }

    if crate::borrowed_result_errors::candidate(node, facts) {
        return Some(
            crate::borrowed_result_errors::verify(node, facts)
                .map(|checked| (checked.receiver, false)),
        );
    }

    if crate::checked_options::get_candidate(node, facts) {
        return Some(
            crate::checked_options::verify_get(node, facts).map(|checked| (checked.receiver, true)),
        );
    }
    None
}

type Handler<'a> =
    fn(&Resolver<'a>, &HirNode, &[usize], &mut Visiting) -> Result<Vec<Origin>, String>;

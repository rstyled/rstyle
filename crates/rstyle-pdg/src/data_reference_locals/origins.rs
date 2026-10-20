//! Resolve formal origins by following source reaching RHS operations. A visited
//! (operation, component) cuts cycles; all terminal formal paths are still visited.

use super::flow::{Index, Value};
use crate::{HirNode, data_origins::Origin, data_types::TypeToken, unit_store::EvidenceFacts};
use std::collections::BTreeSet;

pub(super) fn selected(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    let index = Index::new(facts)?;
    let resolver = Resolver { index, facts };
    let roots = resolver.source(node, selectors, &mut BTreeSet::new())?;
    Ok(roots
        .into_iter()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}

struct Resolver<'a> {
    index: Index<'a>,
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
        let handler: Handler<'_> = match node.kind.as_str() {
            "expr_path" => Self::read,
            "expr_tup" | "expr_struct" => Self::constructed,
            "expr_field" if crate::data_value_fields::candidate(node, self.facts) => {
                Self::projection
            }
            "expr_field" | "expr_unary" | "expr_addr_of" | "pat_binding" => Self::terminal,
            "expr_block" | "expr_ret" => Self::tail,
            "expr_if" => Self::alternatives,
            "expr_call" | "expr_method_call" if selectors.is_empty() => Self::call,
            _ => return Err("Unsupported reference rebinding source operation".into()),
        };
        handler(self, node, selectors, visiting)
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
                crate::data_subslices::verify_borrow(node, self.facts)?;
                let checked = crate::data_subslices::verify(&node.children[0], self.facts)?;
                self.source(checked.base, selectors, visiting)
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
        let definitions = self
            .index
            .reads
            .get(&node.hir.local)
            .filter(|definitions| !definitions.is_empty())
            .ok_or("Reference read has no source reaching definition")?;
        let mut roots = Vec::new();
        for definition in definitions {
            match self
                .index
                .values
                .get(definition)
                .ok_or("Missing reference source definition")?
            {
                Value::Parameter(parameter) => {
                    let ty = crate::data_types::parse(
                        parameter
                            .ty
                            .normalized
                            .as_deref()
                            .ok_or("Missing reference formal type")?,
                    )?;
                    roots.extend(crate::data_component_origins::selected(
                        &ty,
                        selectors,
                        parameter.position,
                        Vec::new(),
                    )?);
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
        _: &[usize],
        visiting: &mut Visiting,
    ) -> Result<Vec<Origin>, String> {
        let mut roots = Vec::new();
        if crate::checked_options::get_candidate(node, self.facts) {
            let checked = crate::checked_options::verify_get(node, self.facts)?;
            return Ok(crate::checked_options::project(self.source(
                checked.receiver,
                &[],
                visiting,
            )?));
        }
        if let Some(origins) = crate::data_bytes::source_origins(node, self.facts, |operand| {
            self.source(operand, &[], visiting)
        }) {
            return origins;
        }

        for position in crate::helper_references::parameters(node, self.facts)? {
            let actual = node
                .children
                .get(position as usize)
                .ok_or("Missing returned reference argument")?;
            roots.extend(self.source(actual, &[], visiting)?);
        }
        Ok(roots)
    }
}

type Handler<'a> =
    fn(&Resolver<'a>, &HirNode, &[usize], &mut Visiting) -> Result<Vec<Origin>, String>;

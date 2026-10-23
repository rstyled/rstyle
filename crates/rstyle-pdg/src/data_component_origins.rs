//! Reconstruct selected component origins from existing value/source slots.

use crate::{
    HirNode, data_origins::Origin, data_types::TypeToken, field_locations::Projection,
    unit_store::EvidenceFacts,
};

pub(crate) fn source(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    if crate::data_reference_locals::flow_enabled(facts) {
        return crate::data_reference_locals::selected(node, selectors, facts);
    }
    if crate::field_locations::derived::candidate(node, facts) {
        return crate::field_locations::derived::selected(node, selectors, facts);
    }
    if selectors.is_empty() {
        return crate::data_origins::origins(node, facts);
    }
    source_operation(node, selectors, facts)
}

fn source_operation(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    match node.kind.as_str() {
        "expr_call" | "expr_method_call"
            if crate::helper_component_outputs::call_used(node, facts) =>
        {
            call(node, selectors, facts)
        }
        "expr_tup" | "expr_struct" => {
            let (operand, rest) = operand(node, selectors, facts)?;
            source(
                node.children
                    .get(operand)
                    .ok_or("Missing selected constructor operand")?,
                rest,
                facts,
            )
        }
        "expr_path" => local_or_formal(node, selectors, facts),
        "expr_field" if crate::data_value_fields::candidate(node, facts) => {
            let path: Vec<_> = [crate::data_value_fields::index(node, facts)?]
                .into_iter()
                .chain(selectors.iter().copied())
                .collect();
            source(&node.children[0], &path, facts)
        }
        "expr_field" => select_origins(node, selectors, facts),
        "expr_block" | "expr_ret" => source(
            node.children
                .last()
                .ok_or("Missing selected aggregate value")?,
            selectors,
            facts,
        ),
        "expr_if" if node.children.len() == 3 => {
            let mut roots = source(&node.children[1], selectors, facts)?;
            roots.extend(source(&node.children[2], selectors, facts)?);
            roots.sort();
            roots.dedup();
            Ok(roots)
        }
        _ => Err("Aggregate component lacks supported source correspondence".into()),
    }
}

fn call(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    crate::helper_component_outputs::map(node, selectors, facts, |position| {
        let actual = node
            .children
            .get(position as usize)
            .ok_or("Missing selected helper actual")?;
        source(actual, &[], facts)
    })
}

fn local_or_formal(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    let ty = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
    let target = node
        .res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref())
        .ok_or("Missing selected aggregate binding")?;
    if let Some(parameter) = facts
        .signature
        .params
        .iter()
        .find(|parameter| parameter.pat.hir == *target)
    {
        return selected(&ty, selectors, parameter.position, Vec::new());
    }
    let initializer = crate::data_reference_locals::initializer(node, facts, &ty)?;
    if crate::data_aggregates::token(initializer)? != crate::data_aggregates::token(node)? {
        return Err("Selected aggregate local and initializer types disagree".into());
    }

    source(initializer, selectors, facts)
}

pub(crate) fn select_origins(
    node: &HirNode,
    selectors: &[usize],
    facts: &dyn EvidenceFacts,
) -> Result<Vec<Origin>, String> {
    let ty = crate::data_types::parse(crate::data_aggregates::token(node)?)?;
    let fields = facts
        .build
        .frontend_options
        .data::<crate::field_locations::FieldObservations>()?;
    let field = fields
        .operations
        .iter()
        .find(|field| field.hir == node.hir)
        .ok_or("Missing selected aggregate field location")?;
    let crate::field_locations::Location::Captured {
        parameter, path, ..
    } = &field.location
    else {
        return Err("Selected aggregate field lacks a captured location".into());
    };

    selected(&ty, selectors, *parameter, path.clone())
}

pub(crate) fn selected(
    ty: &TypeToken,
    selectors: &[usize],
    parameter: u32,
    prefix: Vec<Projection>,
) -> Result<Vec<Origin>, String> {
    let (selection, component) = selection(ty, selectors)?;
    let prefix = prefix.into_iter().chain(selection).collect();
    Ok(crate::data_origins::paths(component, prefix)?
        .into_iter()
        .map(|path| Origin { parameter, path })
        .collect())
}

fn selection<'a>(
    ty: &'a TypeToken,
    selectors: &[usize],
) -> Result<(Vec<Projection>, &'a TypeToken), String> {
    let Some((&position, rest)) = selectors.split_first() else {
        return Ok((Vec::new(), ty));
    };
    let (projection, component) = match ty {
        TypeToken::Tuple(elements) => (
            Projection::TupleField { position },
            elements
                .get(position)
                .ok_or("Selected tuple component is absent")?,
        ),
        TypeToken::Struct { nominal, fields } => {
            let (name, ty) = fields
                .get(position)
                .ok_or("Selected struct component is absent")?;
            (
                Projection::StructField {
                    nominal: nominal.clone(),
                    position,
                    name: name.clone(),
                },
                ty,
            )
        }
        _ => return Err("Selected value is not a tuple or struct".into()),
    };
    let (path, component) = selection(component, rest)?;
    Ok(([projection].into_iter().chain(path).collect(), component))
}

pub(crate) fn operand<'a>(
    node: &HirNode,
    selectors: &'a [usize],
    facts: &dyn EvidenceFacts,
) -> Result<(usize, &'a [usize]), String> {
    let (&position, rest) = selectors
        .split_first()
        .ok_or("Missing component selector")?;
    match node.kind.as_str() {
        "expr_tup" if position < node.children.len() => Ok((position, rest)),
        "expr_struct" => slot(&crate::data_structs::slots(node, facts)?, selectors),
        _ => Err("Selected value lacks a supported constructor".into()),
    }
}

pub(crate) fn slot<'a>(
    slots: &[Option<usize>],
    selectors: &'a [usize],
) -> Result<(usize, &'a [usize]), String> {
    let (&position, rest) = selectors.split_first().ok_or("Missing struct selector")?;
    if let Some(operand) = slots.iter().position(|slot| *slot == Some(position)) {
        return Ok((operand, rest));
    }
    slots
        .iter()
        .position(Option::is_none)
        .map(|operand| (operand, selectors))
        .ok_or("Selected constructor slot is absent".into())
}

pub(crate) fn updated(node: &HirNode, facts: &dyn EvidenceFacts) -> Result<Vec<Origin>, String> {
    let slots = crate::data_structs::slots(node, facts)?;
    let TypeToken::Struct { fields, .. } =
        crate::data_types::parse(crate::data_aggregates::token(node)?)?
    else {
        return Err("Update requires an exact struct type".into());
    };

    let mut roots = Vec::new();
    for (position, _) in fields
        .iter()
        .enumerate()
        .filter(|(_, (_, ty))| crate::data_origins::shared(ty))
    {
        let selectors = [position];
        let (operand, remaining) = slot(&slots, &selectors)?;
        roots.extend(source(
            node.children
                .get(operand)
                .ok_or("Missing update component operand")?,
            remaining,
            facts,
        )?);
    }
    Ok(roots)
}

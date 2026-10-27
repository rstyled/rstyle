//! Independent typed source roster, including all synthetic branch operations.
use super::*;
use crate::{HirNode, concrete_types::TypeTree};

pub(super) struct Spec<'a> {
    pub name: &'static str,
    pub source: &'a HirNode,
    pub token: String,
    pub category: NodeCategory,
}

pub(super) fn specs<'a>(checked: &contract::Checked<'a>) -> Result<Vec<Spec<'a>>, String> {
    let mut specs = prefix(checked)?;
    specs.extend(mapping(checked)?);
    specs.extend(failure(checked)?);

    let r = &checked.roles;
    let p = &checked.proof;
    for (source, ty) in [
        (r.map, &p.intermediate),
        (r.map_err, &p.output),
        (r.source, &p.output),
    ] {
        specs.push(spec("result", source, ty, NodeCategory::Merge)?);
    }
    Ok(specs)
}

fn spec<'a>(
    name: &'static str,
    source: &'a HirNode,
    ty: &TypeTree,
    category: NodeCategory,
) -> Result<Spec<'a>, String> {
    Ok(Spec {
        name,
        source,
        token: crate::data_types::token(ty)?,
        category,
    })
}

fn scalar(name: &str) -> TypeTree {
    TypeTree::Scalar(name.into())
}

fn tuple(name: &str) -> TypeTree {
    TypeTree::Tuple(vec![scalar(name), scalar("usize")])
}

fn prefix<'a>(checked: &contract::Checked<'a>) -> Result<Vec<Spec<'a>>, String> {
    let r = &checked.roles;
    let p = &checked.proof;
    use NodeCategory::{Computation as Value, Control, Declaration, Read};
    Ok(vec![
        spec("outer-environment", r.continuation, &p.storage[0], Read)?,
        spec("outer-is-ok", r.source, &scalar("bool"), Control)?,
        spec(
            "reader-ok-payload",
            r.payload,
            &tuple(r.width.scalar()),
            Value,
        )?,
        spec(
            "decoded-payload",
            r.decoded,
            &scalar(r.width.scalar()),
            Value,
        )?,
        spec("count-payload", r.count, &scalar("usize"), Value)?,
        spec("binding", r.decoded, &scalar("unit"), Declaration)?,
        spec("binding", r.count, &scalar("unit"), Declaration)?,
        spec("reader-error-forward", r.source, &p.output, Value)?,
    ])
}

fn mapping<'a>(checked: &contract::Checked<'a>) -> Result<Vec<Spec<'a>>, String> {
    let r = &checked.roles;
    let p = &checked.proof;
    use NodeCategory::{Computation as Value, Control, Declaration, Read};
    Ok(vec![
        spec("map-environment", r.success, &p.storage[1], Read)?,
        spec("map-is-ok", r.map, &scalar("bool"), Control)?,
        spec(
            "conversion-ok-payload",
            r.converted,
            &scalar("usize"),
            Value,
        )?,
        spec("binding", r.converted, &scalar("unit"), Declaration)?,
        spec("count-capture", r.count_read, &scalar("usize"), Value)?,
        spec("success-tuple", r.tuple, &tuple("usize"), Value)?,
        spec("map-ok", r.map, &p.intermediate, Value)?,
        spec("map-error-forward", r.map, &p.intermediate, Value)?,
    ])
}

fn failure<'a>(checked: &contract::Checked<'a>) -> Result<Vec<Spec<'a>>, String> {
    let r = &checked.roles;
    let p = &checked.proof;
    use NodeCategory::{Computation as Value, Control};
    Ok(vec![
        spec("failure-environment", r.failure, &p.storage[2], Value)?,
        spec("failure-is-err", r.map_err, &scalar("bool"), Control)?,
        spec(
            "conversion-error-discard",
            r.wildcard,
            &scalar("unit"),
            Value,
        )?,
        spec(
            "message-capture",
            r.message_read,
            &TypeTree::Shared(Box::new(TypeTree::Str)),
            Value,
        )?,
        spec("final-error", r.map_err, &p.output, Value)?,
        spec("final-success-payload", r.map_err, &tuple("usize"), Value)?,
        spec("final-ok", r.map_err, &p.output, Value)?,
    ])
}

pub(super) fn construct(name: &str) -> String {
    format!("data.widening-reader.{name}")
}

pub(super) struct Operations<'a> {
    pub nodes: std::collections::BTreeMap<&'static str, &'a PdgNode>,
    pub bindings: [&'a PdgNode; 3],
    pub inputs: [&'a PdgNode; 2],
}

impl<'a> Operations<'a> {
    pub fn get(&self, name: &str) -> Result<&'a PdgNode, String> {
        self.nodes
            .get(name)
            .copied()
            .ok_or_else(|| format!("Missing widening operation {name}"))
    }
}

pub(super) fn load<'a>(
    pair: PairView<'a>,
    checked: &contract::Checked<'_>,
) -> Result<Operations<'a>, String> {
    let mut nodes = std::collections::BTreeMap::new();
    let mut bindings = Vec::new();
    for spec in specs(checked)? {
        let node = operation(pair, &spec)?;
        let kind = if spec.category == NodeCategory::Merge {
            "compat.rust.hir-r0.expression_merge"
        } else {
            &spec.source.kind
        };
        if node.category != spec.category || node.original_kind.known_value() != Some(kind) {
            return Err("Widening operation changes its exact original kind/category".into());
        }
        if spec.name == "result" {
            continue;
        }
        if spec.name == "binding" {
            bindings.push(node);
        } else if nodes.insert(spec.name, node).is_some() {
            return Err("Widening operation roster repeats an operation".into());
        }
    }

    ordinary(pair, checked, &mut nodes)?;
    let inputs = [checked.roles.slice, checked.roles.message].map(|formal| {
        super::super::serialized_formal_input(pair, &formal.pat.span, u64::from(formal.position))
    });
    let [slice, message] = inputs;
    let inputs = [slice?, message?];
    Ok(Operations {
        nodes,
        bindings: bindings
            .try_into()
            .map_err(|_| "Missing widening bindings")?,
        inputs,
    })
}

fn ordinary<'a>(
    pair: PairView<'a>,
    checked: &contract::Checked<'_>,
    nodes: &mut std::collections::BTreeMap<&'static str, &'a PdgNode>,
) -> Result<(), String> {
    let r = &checked.roles;
    let [slice, message] = r.reader.children.as_slice() else {
        return Err("Missing widening reader actuals".into());
    };
    for (name, source) in [
        ("slice", slice),
        ("message", message),
        ("reader", r.reader),
        ("decoded-read", r.decoded_read),
        ("conversion", r.conversion),
        ("converted-read", r.value_read),
        ("constructor", r.constructor),
        ("map-merge", r.map),
        ("failure-merge", r.map_err),
        ("outer-merge", r.source),
    ] {
        nodes.insert(name, super::super::value_operation(pair, source)?);
    }
    let returned = super::super::serialized_operation(pair, &r.source.span, |node| {
        node.category == NodeCategory::Return
    })?;

    nodes.insert("returned", returned);
    Ok(())
}

pub(super) fn operation<'a>(pair: PairView<'a>, spec: &Spec<'_>) -> Result<&'a PdgNode, String> {
    if spec.category != NodeCategory::Merge {
        return super::super::typed_operation(
            pair,
            spec.source,
            &construct(spec.name),
            &spec.token,
        );
    }
    let node = super::super::value_operation(pair, spec.source)?;
    if node.category != NodeCategory::Merge
        || node.attributes.construct.as_deref() != Some(contract::RESULT)
        || super::super::wire_type(node)? != spec.token
    {
        return Err("Widening reader loses its exact complete merge value".into());
    }
    Ok(node)
}

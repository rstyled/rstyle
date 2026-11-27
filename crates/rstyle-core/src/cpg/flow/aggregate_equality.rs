//! Derive the prefix compared during struct or array equality. The individual
//! relations are static frontend facts; sorting and early exit are language semantics.
use super::Derived;
use crate::cpg::model::{FileSpan, Function, Node, Type, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
    pub left: FileSpan,
    pub right: FileSpan,
    pub relation: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Comparison {
    pub evaluated: Vec<Member>,
    pub outcome: String,
}

fn shape(ty: &Type) -> Option<Vec<(usize, Option<&str>)>> {
    match ty.kind {
        TypeKind::Sequence if ty.fixed == Some(true) && ty.length? <= 64 => {
            Some((0..ty.length? as usize).map(|i| (i, None)).collect())
        }
        TypeKind::Record => {
            let fields = ty.fields.as_deref()?;
            (fields.len() <= 64).then(|| {
                fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.name != "_")
                    .map(|(i, f)| (i, Some(f.name.as_str())))
                    .collect()
            })
        }
        _ => None,
    }
}

fn within(root: &Node, s: &FileSpan) -> bool {
    root.span.as_ref().is_some_and(|r| {
        r.file == s.file && r.start <= s.start && s.end <= r.end && s.start < s.end
    })
}

fn comparison(
    tables: &Tables<'_>,
    body: &Body<'_>,
    node: &Node,
    raw: &Value,
) -> Option<Comparison> {
    let invert = match node.operator.as_deref()? {
        "eq" => false,
        "ne" => true,
        _ => return None,
    };
    let x = body.child(node, 0).ok()?;
    let y = body.child(node, 1).ok()?;
    if x.ty != y.ty {
        return None;
    }
    let members = shape(tables.ty(x.ty.as_deref()?).ok()?)?;
    if raw.as_array()?.len() != members.len() {
        return None;
    }
    let mut steps: Vec<Member> = serde_json::from_value(raw.clone()).ok()?;
    steps.sort_by_key(|v| v.index);
    let mut compared = vec![];
    let mut result = (!invert).to_string();
    for (s, (index, name)) in steps.into_iter().zip(members) {
        if s.index != index
            || s.field.as_deref() != name
            || !within(x, &s.left)
            || !within(y, &s.right)
        {
            return None;
        }
        let stop = match s.relation.as_str() {
            "equal" => None,
            "unequal" => Some(invert.to_string()),
            "incomparable" => Some("runtime_panic".into()),
            "unknown" => Some("unknown".into()),
            _ => return None,
        };
        compared.push(s);
        if let Some(outcome) = stop {
            result = outcome;
            break;
        }
    }
    Some(Comparison {
        evaluated: compared,
        outcome: result,
    })
}

pub(crate) fn attach(tables: &Tables<'_>, f: &Function, derived: &mut Derived) {
    let Some(cases) = f
        .extensions
        .as_ref()
        .and_then(|e| e.get("go:aggregate-comparisons"))
        .and_then(Value::as_object)
    else {
        return;
    };
    let Ok(body) = Body::new(f) else { return };
    super::each_reached(f, derived, |_, reached, own| {
        for (site, source) in cases {
            if reached.contains(site.as_str())
                && let Ok(node) = body.node(site)
                && let Some(row) = comparison(tables, &body, node, source)
            {
                own.aggregate_comparisons.insert(site.clone(), row);
            }
        }
    });
}

//! Bounded value transport from a fresh Go range operand to its iteration
//! positions. The items describe what a continuing loop can consume, independently
//! of an early break. Mutable or unknown backing storage has no concrete listing.

use super::Derived;
use crate::cpg::model::{Function, Literal, Node, NodeKind, Type, TypeKind};
use crate::cpg::tree::{Body, Tables};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

const ITEM_LIMIT: usize = 256;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RangeItems {
    pub order: String,
    /// Whether array/slice iteration needs to read the stored element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_element: Option<bool>,
    pub components: Vec<String>,
    /// None means that the component types are known but the items are not.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<Vec<String>>>,
}

struct Range<'a> {
    tables: &'a Tables<'a>,
    body: &'a Body<'a>,
    values: BTreeMap<&'a str, &'a Literal>,
}

impl Range<'_> {
    fn literal(&self, n: &Node) -> Option<&Literal> {
        self.values.get(n.id.as_str()).copied()
    }

    fn nil(&self, n: &Node) -> bool {
        let operand = if n.kind == NodeKind::Cast && n.children.len() == 1 {
            self.body.node(&n.children[0]).ok()
        } else {
            None
        };
        let operand = operand.unwrap_or(n);
        let target = self
            .body
            .reference(&operand.id)
            .and_then(|r| r.target.as_ref())
            .and_then(|t| t.id.as_deref());
        self.tables
            .named_definition(target)
            .is_some_and(|d| d.standard && d.name == "nil")
    }

    fn integer(&self, n: &Node) -> Option<Vec<Vec<String>>> {
        let value = self
            .literal(n)?
            .value
            .as_deref()?
            .parse::<i128>()
            .ok()?
            .max(0);
        let count = usize::try_from(value).ok().filter(|&n| n <= ITEM_LIMIT)?;
        Some((0..count).map(|i| vec![i.to_string()]).collect())
    }

    fn bytes(&self, n: &Node) -> Option<Vec<u8>> {
        let literal = self.literal(n)?;
        if literal.kind != "string" {
            return None;
        }
        if let Some(s) = &literal.value {
            return (s.len() <= 4 * ITEM_LIMIT).then(|| s.as_bytes().to_vec());
        }
        let hex = self
            .tables
            .extension("go:exact-constants")?
            .get(&n.id)?
            .get("bytes")?
            .as_str()?;
        if hex.len() > 8 * ITEM_LIMIT || hex.len() % 2 != 0 || !hex.is_ascii() {
            return None;
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).ok())
            .collect()
    }

    fn strings(&self, n: &Node) -> Option<Vec<Vec<String>>> {
        let data = self.bytes(n)?;
        let mut at = 0;
        let mut out = Vec::new();
        while at < data.len() && out.len() < ITEM_LIMIT {
            let (ch, width) = rune(&data[at..]);
            out.push(vec![at.to_string(), u32::from(ch).to_string()]);
            at += width;
        }
        (at == data.len()).then_some(out)
    }

    fn positions(&self, n: &Node, ty: &Type) -> Option<Vec<Vec<String>>> {
        if ty.fixed == Some(false) && self.nil(n) {
            return Some(Vec::new());
        }
        let owner = if n.kind == NodeKind::Reference {
            self.body.node(n.children.first()?).ok()?
        } else {
            n
        };
        if owner.kind != NodeKind::Array {
            return None;
        }
        // Keyed or partially populated literals need index and zero-fill handling.
        let len = ty
            .length
            .map_or(Some(owner.children.len()), |n| usize::try_from(n).ok())?;
        if len != owner.children.len() || len > ITEM_LIMIT {
            return None;
        }
        owner
            .children
            .iter()
            .enumerate()
            .map(|(i, id)| {
                let elem = self.body.node(id).ok()?;
                Some(vec![i.to_string(), self.literal(elem)?.value.clone()?])
            })
            .collect()
    }

    fn entries(&self, n: &Node) -> Option<Vec<Vec<String>>> {
        if self.nil(n) {
            return Some(Vec::new());
        }
        if n.kind != NodeKind::Array || n.children.len() > ITEM_LIMIT {
            return None;
        }
        let mut keys = BTreeSet::new();
        let mut rows = Vec::new();
        for child in &n.children {
            let pair = self.body.node(child).ok()?;
            if pair.kind != NodeKind::Tuple || pair.children.len() != 2 {
                return None;
            }
            let key = self
                .literal(self.body.node(&pair.children[0]).ok()?)?
                .value
                .clone()?;
            if !keys.insert(key.clone()) {
                return None;
            }
            let value = self
                .literal(self.body.node(&pair.children[1]).ok()?)?
                .value
                .clone()?;
            rows.push(vec![key, value]);
        }
        rows.sort();
        Some(rows)
    }

    fn result(&self, head: &Node) -> Option<RangeItems> {
        let n = self.body.node(head.children.get(1)?).ok()?;
        let mut ty = self.tables.ty(n.ty.as_deref()?).ok()?;
        if ty.kind == TypeKind::Reference {
            ty = self.tables.ty(ty.target.as_deref()?).ok()?;
            if ty.kind != TypeKind::Sequence || ty.fixed != Some(true) {
                return None;
            }
        }
        let (order, components, items) = match ty.kind {
            TypeKind::Integer => ("ascending", vec![ty.spelling.clone()], self.integer(n)),
            TypeKind::String => (
                "byte_offset",
                vec!["int".into(), "rune".into()],
                self.strings(n),
            ),
            TypeKind::Sequence => (
                "ascending",
                vec![
                    "int".into(),
                    self.tables
                        .ty(ty.element.as_deref()?)
                        .ok()?
                        .spelling
                        .clone(),
                ],
                self.positions(n, ty),
            ),
            TypeKind::Map => (
                "unspecified",
                vec![
                    self.tables
                        .ty(ty.key_type.as_deref()?)
                        .ok()?
                        .spelling
                        .clone(),
                    self.tables
                        .ty(ty.value_type.as_deref()?)
                        .ok()?
                        .spelling
                        .clone(),
                ],
                self.entries(n),
            ),
            _ => {
                let ch = self.tables.extension("go:channel-types")?.get(&ty.id)?;
                if ch["direction"].as_str() == Some("send") {
                    return None;
                }
                (
                    "receive",
                    vec![
                        self.tables
                            .ty(ch["element"].as_str()?)
                            .ok()?
                            .spelling
                            .clone(),
                    ],
                    None,
                )
            }
        };
        let load_element = if ty.kind == TypeKind::Sequence {
            let binding = self.body.node(head.children.first()?).ok()?;
            let second = binding
                .children
                .get(1)
                .and_then(|id| self.body.node(id).ok());
            Some(
                binding.kind == NodeKind::PatternTuple
                    && second.is_some_and(|n| {
                        n.kind != NodeKind::PatternWildcard && n.original_kind != "Ident.blank"
                    }),
            )
        } else {
            None
        };
        Some(RangeItems {
            load_element,
            order: order.into(),
            components,
            items,
        })
    }
}

fn rune(data: &[u8]) -> (char, usize) {
    let width = match data[0] {
        0..=127 => 1,
        0xc2..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf4 => 4,
        _ => 0,
    };
    data.get(..width)
        .and_then(|s| std::str::from_utf8(s).ok())
        .and_then(|s| s.chars().next())
        .map_or(('\u{fffd}', 1), |ch| (ch, width))
}

pub(crate) fn attach(tables: &Tables<'_>, function: &Function, derived: &mut Derived) {
    if tables.language() != "go"
        || !function
            .nodes
            .iter()
            .any(|n| n.original_kind == "RangeStmt")
    {
        return;
    }
    let Ok(body) = Body::new(function) else {
        return;
    };
    let mut values: BTreeMap<_, _> = function
        .nodes
        .iter()
        .filter_map(|n| Some((n.id.as_str(), n.value.as_ref()?)))
        .collect();
    values.extend(
        function
            .values
            .iter()
            .filter_map(|v| Some((v.node.as_deref()?, &v.value))),
    );
    let ranges = Range {
        tables,
        body: &body,
        values,
    };
    super::each_reached(function, derived, |reached, own| {
        for head in function.nodes.iter().filter(|n| {
            n.kind == NodeKind::For
                && n.original_kind == "RangeStmt"
                && reached.contains(n.id.as_str())
        }) {
            if let Some(result) = ranges.result(head) {
                own.range_items.insert(head.id.clone(), result);
            }
        }
    });
}

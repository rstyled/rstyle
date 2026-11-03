//! Path and loop-exit resolutions, as the frontend's facts record them.

use super::{Context, hir_ref};
use crate::frontend::{HirRef, PathRes};
use rstyle_core::cpg::model::{Node, NodeKind, TargetKind};
use rstyle_core::cpg::tree::Body;
use std::collections::BTreeMap;

/// Each loop exit's destination: the loop it leaves or continues, with its label.
pub(super) fn loop_exits(body: &Body<'_>) -> Result<BTreeMap<String, (bool, HirRef)>, String> {
    let mut exits = BTreeMap::new();
    let mut loops: Vec<(Option<String>, HirRef)> = Vec::new();
    visit(body, body.root()?, &mut loops, &mut exits)?;
    Ok(exits)
}

fn visit<'a>(
    body: &Body<'a>,
    node: &'a Node,
    loops: &mut Vec<(Option<String>, HirRef)>,
    exits: &mut BTreeMap<String, (bool, HirRef)>,
) -> Result<(), String> {
    if matches!(node.kind, NodeKind::Break | NodeKind::Continue) {
        let destination = match &node.label {
            Some(label) => loops
                .iter()
                .rev()
                .find(|(own, _)| own.as_ref() == Some(label)),
            None => loops.last(),
        };
        if let Some((_, target)) = destination {
            exits.insert(node.id.clone(), (node.label.is_some(), target.clone()));
        }
    }
    let entered = looped(body, node)?;
    if let Some(target) = entered.clone() {
        loops.push((node.label.clone(), target));
    }
    for child in body.children(node)? {
        visit(body, child, loops, exits)?;
    }
    if entered.is_some() {
        loops.pop();
    }
    Ok(())
}

/// The compiler's loop a construct is: itself, or for a `for` its lowering's `loop`.
fn looped(body: &Body<'_>, node: &Node) -> Result<Option<HirRef>, String> {
    Ok(match node.kind {
        NodeKind::Loop | NodeKind::While => Some(hir_ref(&node.id)),
        NodeKind::For => body
            .children(node)?
            .into_iter()
            .find(|child| child.original_kind == "expr_for")
            .map(|iteration| hir_ref(&iteration.id)),
        _ => None,
    })
}

impl Context<'_, '_> {
    pub(super) fn resolution(&self, node: &Node, tag: &str) -> Option<PathRes> {
        match tag {
            "expr_break" | "expr_continue" => self.loop_exit(node),
            "expr_path" => self.path(node),
            "expr_method_call" => self.method_callee(node),
            _ => None,
        }
    }

    fn loop_exit(&self, node: &Node) -> Option<PathRes> {
        let (labelled, target) = self.exits.get(&node.id)?;
        Some(PathRes {
            kind: if *labelled {
                "labelled_loop"
            } else {
                "innermost_loop"
            }
            .into(),
            target: Some(target.clone()),
            description: "Compiler-resolved loop exit destination".into(),
        })
    }

    fn path(&self, node: &Node) -> Option<PathRes> {
        let reference = self.body.reference(&node.id)?;
        let target = reference.target.as_ref()?;
        match target.kind {
            TargetKind::Symbol => {
                let symbol = target.id.as_deref()?;
                let binding = hir_ref(symbol);
                Some(PathRes {
                    kind: "local".into(),
                    description: format!(
                        "Local(HirId({}.{}))",
                        self.owner_debug(binding.owner)?,
                        binding.local
                    ),
                    target: Some(binding),
                })
            }
            TargetKind::Definition | TargetKind::Constructor => {
                let definition = self.reader.tables.definition(target.id.as_deref()?).ok()?;
                let rust = |key: &str| definition.extensions.get(key);
                Some(PathRes {
                    kind: "other".into(),
                    target: None,
                    description: format!(
                        "Def({}, {})",
                        rust("rust:def-kind")?,
                        rust("rust:def-id")?
                    ),
                })
            }
            _ => None,
        }
    }

    /// A method call's declared callee, as a library callee record.
    fn method_callee(&self, node: &Node) -> Option<PathRes> {
        let reference = self.body.reference(&node.id)?;
        let declared = match &reference.target {
            Some(target) => target.id.as_deref()?,
            None => reference.via.as_deref()?,
        };
        let definition = self.reader.tables.definition(declared).ok()?;
        let rust = |key: &str| definition.extensions.get(key).cloned();
        let callee = crate::data_library::Callee {
            path: definition.name.clone(),
            crate_name: rust("rust:crate-name")?,
            crate_hash: rust("rust:crate-hash").unwrap_or_else(|| "local-unencoded".into()),
            definition_hash: rust("rust:def-path-hash")?,
            implementation: None,
        };
        Some(PathRes {
            kind: "resolved_callee".into(),
            target: None,
            description: serde_json::to_value(callee).ok()?.to_string(),
        })
    }

    /// The function's definition as the compiler prints a local `DefId`.
    fn owner_debug(&self, owner: u32) -> Option<String> {
        let definition = self
            .reader
            .tables
            .definition(&self.body.function.definition)
            .ok()?;
        let identity = &definition.stable_identity;
        let (krate, path) = identity.split_at(identity.find("::").unwrap_or(identity.len()));
        let compilation = definition.compilation.as_deref()?;
        Some(format!(
            "DefId(0:{owner} ~ {krate}[{}]{path})",
            compilation.get(..4)?
        ))
    }
}

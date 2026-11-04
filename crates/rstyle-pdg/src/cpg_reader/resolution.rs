//! Path and loop-exit resolutions, as the frontend's facts record them.

use super::{Context, hir_ref};
use crate::frontend::{HirRef, PathRes};
use rstyle_core::cpg::model::{Node, NodeKind, Origin, TargetKind, TypeKind};
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
        let observed = self
            .float_literal(node, tag)
            .or_else(|| self.option_expression(node))
            .or_else(|| self.result_expression(node));
        if let Some(observed) = observed {
            return Some(observed);
        }
        match tag {
            "expr_break" | "expr_continue" => self.loop_exit(node),
            "expr_path" => self.path(node),
            "expr_method_call" => self.method_callee(node),
            "expr_index" if self.reader.profile.is_data() => self.method_callee(node),
            "expr_struct" if self.range_desugaring(node) => {
                let ty = self.reader.ty(node.ty.as_deref()?).ok()?;
                let definition = self
                    .reader
                    .tables
                    .definition(ty.definition.as_deref()?)
                    .ok()?;
                let mut resolution = self.library_callee(definition)?;
                resolution.kind = "resolved_range_type".into();
                Some(resolution)
            }
            "expr_call" | "expr_struct" if node.original_kind == "expr_call" => {
                self.direct_callee(node)
            }
            "expr_field" if self.reader.profile.supports("rust.verified-helper-calls") => {
                self.field_resolution(node)
            }
            _ => None,
        }
    }

    /// A range expression the data profile observes as the library range it builds.
    fn range_desugaring(&self, node: &Node) -> bool {
        self.reader.profile.is_data()
            && node.original_kind == "expr_struct"
            && node.provenance.construct.as_deref() == Some("rust:range")
    }

    /// A source float literal's value as the compiler evaluates it, by its bits.
    fn float_literal(&self, node: &Node, tag: &str) -> Option<PathRes> {
        if tag != "expr_lit"
            || !self.reader.profile.supports("rust.float-kernel-scalars")
            || node.provenance.origin != Origin::Source
        {
            return None;
        }
        let ty = self.reader.ty(node.ty.as_deref()?).ok()?;
        if ty.kind != TypeKind::Float {
            return None;
        }
        let text = self
            .snippet(self.span(node.span.as_ref()?))
            .replace('_', "");
        let digits = text.trim_end_matches("f32").trim_end_matches("f64");
        let bits = match ty.spelling.as_str() {
            "f32" => format!("0x{:08x}", digits.parse::<f32>().ok()?.to_bits()),
            "f64" => format!("0x{:016x}", digits.parse::<f64>().ok()?.to_bits()),
            _ => return None,
        };
        Some(PathRes {
            kind: "resolved_float_literal".into(),
            target: None,
            description: serde_json::json!({"ty": ty.spelling, "bits": bits}).to_string(),
        })
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
        let target = self.body.reference(&node.id)?.target.as_ref()?;
        if matches!(
            target.kind,
            TargetKind::Definition | TargetKind::Constructor | TargetKind::Variant
        ) {
            let definition = self.reader.tables.definition(target.id.as_deref()?).ok()?;
            if let Some((resolution, _)) = self.named_option_function(node) {
                return Some(resolution);
            }
            if let Some(resolution) = self
                .fieldless_variant(definition)
                .or_else(|| self.constant_resolution(definition))
            {
                return Some(resolution);
            }
        }
        self.path_resolution(node)
    }

    /// What a path resolves to, as the compiler prints the resolution.
    pub(super) fn path_resolution(&self, node: &Node) -> Option<PathRes> {
        let target = self.body.reference(&node.id)?.target.as_ref()?;
        match target.kind {
            TargetKind::Symbol => {
                let binding = hir_ref(target.id.as_deref()?);
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
            TargetKind::Definition | TargetKind::Constructor | TargetKind::Variant => {
                let definition = self.reader.tables.definition(target.id.as_deref()?).ok()?;
                let rust = |key: &str| identity(definition, key);
                Some(PathRes {
                    kind: "other".into(),
                    target: None,
                    description: format!("Def({}, {})", rust("def_kind")?, rust("def_id")?),
                })
            }
            _ => None,
        }
    }

    /// A field access's position in its owner, as the frontend records helper fields.
    fn field_resolution(&self, node: &Node) -> Option<PathRes> {
        let base = self.body.child(node, 0).ok()?;
        let adjusted = base
            .adjustments
            .as_ref()
            .and_then(|adjustments| adjustments.last())
            .and_then(|adjustment| adjustment.target.as_deref())
            .or(base.ty.as_deref())?;
        let ty = self.reader.ty(adjusted).ok()?;
        let owner_path = match ty.kind {
            TypeKind::Record | TypeKind::Sum => {
                let definition = self
                    .reader
                    .tables
                    .definition(ty.definition.as_deref()?)
                    .ok()?;
                Some(definition.name.clone())
            }
            TypeKind::Tuple => None,
            _ => return None,
        };
        let resolution = crate::field_locations::FieldResolution {
            position: node.member_index? as usize,
            owner_path,
        };
        Some(PathRes {
            kind: "resolved_field".into(),
            target: None,
            description: serde_json::to_string(&resolution).ok()?,
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

/// A part of the compiler's own identity of a definition, as the compiler prints it.
fn identity<'d>(definition: &'d rstyle_core::cpg::model::Definition, key: &str) -> Option<&'d str> {
    definition
        .extensions
        .get("rust:compiler-identity")?
        .get(key)?
        .as_str()
}

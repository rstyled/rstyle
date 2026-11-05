//! The data profile's Option and local enum variant observations, on expressions and
//! patterns, as the frontend records them.

use super::{Context, identity};
use crate::extraction_profile::ExtractionProfile;
use crate::frontend::{HirNode, PathRes};
use rstyle_core::cpg::model::{Definition, Node, TargetKind, Type, TypeKind};

type Built = Result<Option<HirNode>, String>;

impl<'a> Context<'_, 'a> {
    fn data_v3(&self) -> bool {
        self.reader.profile == ExtractionProfile::DataV3
    }

    /// `Some(value)` or `None` of an Option the data profile observes.
    pub(super) fn option_expression(&self, node: &Node) -> Option<PathRes> {
        if !self.data_v3() {
            return None;
        }
        let resolution = match node.original_kind.as_str() {
            "expr_call" if node.children.len() == 2 => {
                let callee = self.body.child(node, 0).ok()?;
                (callee.original_kind == "expr_path")
                    .then(|| self.option_variant(callee, "Some"))??
            }
            "expr_path" => self.option_variant(node, "None")?,
            _ => return None,
        };
        let ty = self.reader.ty(node.ty.as_deref()?).ok()?;
        (self.scalar_payload(ty) || self.product_payload(ty)).then_some(resolution)
    }

    /// `Ok(value)` or `Err(error)` of a Result, where the profile observes fresh results.
    pub(super) fn result_expression(&self, node: &Node) -> Option<PathRes> {
        if !self.reader.profile.supports("rust.fresh-concrete-results")
            || node.original_kind != "expr_call"
            || node.children.len() != 2
        {
            return None;
        }
        let ty = self.reader.ty(node.ty.as_deref()?).ok()?;
        let definition = self
            .reader
            .tables
            .definition(ty.definition.as_deref()?)
            .ok()?;
        if identity(definition, "diagnostic") != Some("Result") || ty.arguments.as_ref()?.len() != 2
        {
            return None;
        }
        let callee = self.body.child(node, 0).ok()?;
        if callee.original_kind != "expr_path" {
            return None;
        }
        let constructor = self.constructor(callee)?;
        let owner = self
            .reader
            .tables
            .definition(identity(constructor, "enum")?)
            .ok()?;
        if identity(owner, "diagnostic") != Some("Result")
            || !matches!(identity(constructor, "variant"), Some("Ok" | "Err"))
        {
            return None;
        }

        self.variant_record(constructor, "resolved_result_variant")
    }

    /// A variant constructor as the library record the frontend serializes for it.
    fn variant_record(&self, definition: &Definition, kind: &str) -> Option<PathRes> {
        let callee = crate::data_library::Callee {
            path: definition.name.clone(),
            crate_name: identity(definition, "crate_name")?.to_owned(),
            crate_hash: identity(definition, "crate_hash")?.to_owned(),
            definition_hash: identity(definition, "def_path_hash")?.to_owned(),
            implementation: None,
        };
        Some(PathRes {
            kind: kind.into(),
            target: None,
            description: serde_json::to_string(&callee).ok()?,
        })
    }

    /// A path to a local enum's variant, where the profile observes fieldless enums.
    pub(super) fn fieldless_variant(&self, definition: &'a Definition) -> Option<PathRes> {
        if !self.reader.profile.supports("rust.fieldless-enum-values") {
            return None;
        }
        let variant = match identity(definition, "def_kind")? {
            "Variant" => definition,
            "Ctor(Variant, Const)" => self
                .reader
                .tables
                .definition(identity(definition, "variant_definition")?)
                .ok()?,
            _ => return None,
        };
        if variant.scope != rstyle_core::cpg::model::Scope::Unit {
            return None;
        }
        let mut resolution = self.library_callee(variant)?;
        resolution.kind = "resolved_fieldless_variant".into();

        Some(resolution)
    }

    /// A path to a local nongeneric scalar constant, with its declaration and value.
    pub(super) fn constant_resolution(&self, definition: &'a Definition) -> Option<PathRes> {
        let kind = identity(definition, "def_kind")?;
        let inherent = identity(definition, "inherent_impl").is_some();
        let generic = definition
            .extensions
            .get("rust:compiler-identity")?
            .get("generics")
            .and_then(|generics| generics.get("count")?.as_u64())
            .unwrap_or(0);
        if !self
            .reader
            .profile
            .supports("rust.resolved-scalar-constants")
            || definition.scope != rstyle_core::cpg::model::Scope::Unit
            || generic != 0
            || !(kind == "Const" || kind == "AssocConst" && inherent)
        {
            return None;
        }
        let value = self
            .reader
            .unit
            .values
            .iter()
            .find(|value| value.definition.as_deref() == Some(definition.id.as_str()))?;
        let ty = self
            .reader
            .resolved(value.ty.as_deref()?, String::new())
            .ok()?
            .normalized?;
        let declaration = self.reader.source(definition.span.as_ref()?).ok()?;
        if value.value.kind == "float" && !self.float_constant(definition, declaration.as_str()) {
            return None;
        }
        let constant = crate::scalar_constants::ScalarConstant {
            definition: definition.name.clone(),
            declaration: declaration.as_str().to_owned(),
            ty,
            value: value.value.value.clone()?,
        };

        Some(PathRes {
            kind: crate::scalar_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&constant).ok()?,
        })
    }

    /// A float constant the profile observes exactly: a float literal, possibly negated,
    /// declared beside the function reading it.
    fn float_constant(&self, definition: &Definition, declaration: &str) -> bool {
        let function = self
            .reader
            .tables
            .definition(&self.body.function.definition)
            .ok();
        let parent = |definition: &Definition| identity(definition, "parent").map(str::to_owned);
        if !self.reader.profile.supports("rust.float-kernel-scalars")
            || function
                .and_then(parent)
                .is_none_or(|own| parent(definition) != Some(own))
        {
            return false;
        }
        let expression = match syn::parse_str::<syn::ImplItemConst>(declaration) {
            Ok(constant) => constant.expr,
            Err(_) => return false,
        };
        let expression = match expression {
            syn::Expr::Unary(syn::ExprUnary {
                op: syn::UnOp::Neg(_),
                expr,
                ..
            }) => *expr,
            expression => expression,
        };

        matches!(
            expression,
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Float(_),
                ..
            })
        )
    }

    /// The data profile's own pattern records: shared scalars, Option and local variants.
    pub(super) fn observed_pattern(&self, pattern: &'a Node) -> Built {
        if self.data_v3() {
            if let Some(observed) = self.option_pattern(pattern)? {
                return Ok(Some(observed));
            }
            if let Some(observed) = self.local_variant_pattern(pattern)? {
                return Ok(Some(observed));
            }
        }
        self.slice_prefix_pattern(pattern)
    }

    fn option_pattern(&self, pattern: &'a Node) -> Built {
        let Some(value) = pattern.ty.as_deref().and_then(|ty| self.reader.ty(ty).ok()) else {
            return Ok(None);
        };
        let children = self.parts(pattern)?;
        let (kind, resolution, children) = match pattern.original_kind.as_str() {
            "pat_ref" if self.shared_scalar(value) => {
                let inner = children
                    .iter()
                    .map(|child| self.pattern(child))
                    .collect::<Result<_, _>>()?;
                ("pat_shared", None, inner)
            }
            "pat_tuple_struct" if children.len() == 1 && !rest(self, pattern) => {
                if !self.scalar_payload(value) {
                    return Ok(None);
                }
                let Some(resolution) = self.option_variant(pattern, "Some") else {
                    return Ok(None);
                };
                let inner = children
                    .iter()
                    .map(|child| self.pattern(child))
                    .collect::<Result<_, _>>()?;
                ("pat_option_some", Some(resolution), inner)
            }
            "pat_path" => {
                if !self.scalar_payload(value) {
                    return Ok(None);
                }
                let Some(resolution) = self.option_variant(pattern, "None") else {
                    return Ok(None);
                };
                ("pat_option_none", Some(resolution), Vec::new())
            }
            _ => return Ok(None),
        };
        self.observed(pattern, kind, (resolution, children))
            .map(Some)
    }

    fn local_variant_pattern(&self, pattern: &'a Node) -> Built {
        if pattern.original_kind != "pat_tuple_struct" || rest(self, pattern) {
            return Ok(None);
        }
        let Some(value) = pattern.ty.as_deref().and_then(|ty| self.reader.ty(ty).ok()) else {
            return Ok(None);
        };
        let Some(owner) = value.definition.as_deref() else {
            return Ok(None);
        };
        let local_enum = value.kind == TypeKind::Sum
            && self
                .reader
                .tables
                .definition(owner)
                .is_ok_and(|owner| owner.scope == rstyle_core::cpg::model::Scope::Unit);
        let Some(constructor) = self.constructor(pattern) else {
            return Ok(None);
        };
        if !local_enum || identity(constructor, "enum") != Some(owner) {
            return Ok(None);
        }
        let Some(mut resolution) = self.library_callee(constructor) else {
            return Ok(None);
        };
        resolution.kind = "resolved_local_variant".into();
        let children = self.parts(pattern)?;
        let children = children
            .iter()
            .map(|child| self.pattern(child))
            .collect::<Result<_, _>>()?;

        self.observed(pattern, "pat_local_variant", (Some(resolution), children))
            .map(Some)
    }

    fn observed(
        &self,
        pattern: &'a Node,
        kind: &str,
        (res, children): (Option<PathRes>, Vec<HirNode>),
    ) -> Result<HirNode, String> {
        let node = self.pattern_node(pattern, kind)?;
        Ok(HirNode {
            res,
            children,
            ..node
        })
    }

    /// The constructor a path names.
    fn constructor(&self, path: &Node) -> Option<&'a Definition> {
        self.target_definition(path, TargetKind::Constructor)
    }

    /// The Option variant `expected` a path names, as a library variant record.
    fn option_variant(&self, path: &Node, expected: &str) -> Option<PathRes> {
        let target = self.body.reference(&path.id)?.target.as_ref()?;
        let definition = self.reader.tables.definition(target.id.as_deref()?).ok()?;
        let owner = self
            .reader
            .tables
            .definition(identity(definition, "enum")?)
            .ok()?;
        if identity(owner, "diagnostic") != Some("Option")
            || identity(definition, "variant") != Some(expected)
        {
            return None;
        }

        self.variant_record(definition, "resolved_option_variant")
    }

    fn option_argument(&self, ty: &Type) -> Option<&'a Type> {
        let [payload] = self.reader.diagnostic_arguments(ty, "Option")? else {
            return None;
        };
        self.reader.ty(payload).ok()
    }

    /// An Option of a bool or integer, or a shared reference to one.
    fn scalar_payload(&self, ty: &Type) -> bool {
        self.option_argument(ty)
            .is_some_and(|payload| scalar(payload) || self.shared_scalar(payload))
    }

    /// An Option of a tuple or nominal value, where the profile propagates them.
    fn product_payload(&self, ty: &Type) -> bool {
        self.reader
            .profile
            .supports("rust.concrete-option-propagation")
            && self.option_argument(ty).is_some_and(|payload| {
                matches!(
                    payload.kind,
                    TypeKind::Tuple | TypeKind::Record | TypeKind::Sum
                ) || payload.kind == TypeKind::Pointer && payload.mode.as_deref() == Some("owned")
            })
    }

    fn shared_scalar(&self, ty: &Type) -> bool {
        ty.kind == TypeKind::Reference
            && ty.mode.as_deref() == Some("shared")
            && ty
                .target
                .as_deref()
                .and_then(|target| self.reader.ty(target).ok())
                .is_some_and(scalar)
    }
}

fn scalar(ty: &Type) -> bool {
    matches!(ty.kind, TypeKind::Bool | TypeKind::Integer)
}

/// Whether a tuple struct pattern elides fields with `..`.
pub(super) fn rest(context: &Context<'_, '_>, pattern: &Node) -> bool {
    let Some(span) = pattern.span.as_ref() else {
        return false;
    };
    let text = context.snippet(context.span(span));
    let inner = text.split_once('(').map_or("", |(_, inner)| inner);
    inner
        .split(',')
        .any(|part| part.trim().trim_end_matches(')').trim() == "..")
}

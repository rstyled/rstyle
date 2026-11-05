//! Local items a body declares, and library and local constants the data profile
//! observes by their evaluated values: bool tables, native sizes and `u32::MAX`.

use super::{Context, identity};
use crate::bool_constants::{self, BoolArray, ConstFunction};
use crate::data_types::TypeToken;
use crate::frontend::{HirNode, PathRes};
use rstyle_core::cpg::model::{Definition, Node, Scope, TypeKind, Value};

impl<'a> Context<'_, 'a> {
    /// An item statement, refined where it declares a bool table or a native size.
    pub(super) fn item_statement(&self, node: &'a Node) -> Result<HirNode, String> {
        let base = self.node(node, "stmt_item")?;
        let Some(definition) = self.declared_item(node) else {
            return Ok(base);
        };
        let declaration = self.const_array_item(definition).or_else(|| {
            self.native_size_constant(definition)
                .map(|res| ("stmt_size_const", res))
        });
        Ok(match declaration {
            Some((kind, res)) => HirNode {
                kind: kind.into(),
                res: Some(res),
                ..base
            },
            None => base,
        })
    }

    /// The local item a statement declares: the definition written at its span.
    fn declared_item(&self, node: &Node) -> Option<&'a Definition> {
        let span = node.span.as_ref()?;
        self.reader.unit.definitions.iter().find(|definition| {
            definition.scope == Scope::Unit
                && definition.span.as_ref().is_some_and(|own| {
                    own.file == span.file && own.start == span.start && own.end == span.end
                })
        })
    }

    fn const_array_item(&self, definition: &'a Definition) -> Option<(&'static str, PathRes)> {
        if !self.reader.profile.supports(bool_constants::CAPABILITY) {
            return None;
        }
        match identity(definition, "def_kind")? {
            "Const" => Some(("stmt_bool_array", self.bool_array_resolution(definition)?)),
            "Fn" => {
                let declaration = self.reader.source(definition.span.as_ref()?).ok()?;
                bool_constants::function_declaration(&declaration).ok()?;
                let result = self.reader.ty(&definition.callable.as_ref()?.result).ok()?;
                let ty = bool_array(self, result.element.as_deref()?, result)?;
                let record = ConstFunction {
                    definition: definition.name.clone(),
                    declaration: declaration.as_str().to_owned(),
                    ty,
                };
                Some((
                    "stmt_const_array_fn",
                    PathRes::observation(bool_constants::FUNCTION, &record).ok()?,
                ))
            }
            _ => None,
        }
    }

    /// A local bool table constant, with its evaluated elements and initializer.
    pub(super) fn bool_array_resolution(&self, definition: &'a Definition) -> Option<PathRes> {
        if !self.reader.profile.supports(bool_constants::CAPABILITY)
            || !self.local_constant(definition)
        {
            return None;
        }
        let value = self.constant_value(definition)?;
        let ty = self.reader.ty(value.ty.as_deref()?).ok()?;
        let length = ty.length?;
        if !(1..=bool_constants::MAX_LENGTH).contains(&length) {
            return None;
        }
        let token = bool_array(self, ty.element.as_deref()?, ty)?;
        let declaration = self.reader.source(definition.span.as_ref()?).ok()?;
        bool_constants::array_declaration(&declaration).ok()?;
        let values = value
            .value
            .items
            .as_deref()?
            .iter()
            .map(|item| match item.value.as_deref() {
                Some("true") => Some(true),
                Some("false") => Some(false),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?;
        let initializer = self.array_initializer(definition)?;

        PathRes::observation(
            bool_constants::RESOLUTION,
            &BoolArray {
                definition: definition.name.clone(),
                declaration: declaration.as_str().to_owned(),
                ty: token,
                values,
                initializer,
            },
        )
        .ok()
    }

    /// The sibling local function a constant's body calls.
    fn array_initializer(&self, definition: &Definition) -> Option<String> {
        if identity(definition, "def_kind")? != "Const" {
            return None;
        }
        let initializer = definition.extensions.get("rust:constant-initializer")?;
        let callee = self
            .reader
            .tables
            .definition(initializer.get("callee")?.as_str()?)
            .ok()?;
        if callee.scope != Scope::Unit
            || identity(callee, "def_kind")? != "Fn"
            || identity(callee, "parent")? != identity(definition, "parent")?
        {
            return None;
        }

        Some(callee.name.clone())
    }

    /// A local constant initialized by `size_of::<usize>()`, or another library call
    /// instantiated at `usize` alone.
    fn native_size_constant(&self, definition: &'a Definition) -> Option<PathRes> {
        if !self
            .reader
            .profile
            .supports("rust.native-borrowed-byte-readers")
            || identity(definition, "def_kind")? != "Const"
        {
            return None;
        }
        let initializer = definition.extensions.get("rust:constant-initializer")?;
        let [argument] = initializer.get("arguments")?.as_array()?.as_slice() else {
            return None;
        };
        if self.reader.ty(argument.as_str()?).ok()?.spelling != "usize" {
            return None;
        }
        let callee = self
            .reader
            .tables
            .definition(initializer.get("callee")?.as_str()?)
            .ok()?;
        let resolution = self.constant_resolution(definition)?;
        let constant = crate::scalar_constants::ScalarConstant::decode(&resolution).ok()?;
        crate::size_constants::declaration(&constant.declaration).ok()?;
        let record = crate::size_constants::NativeSize {
            constant,
            initializer: self.library_record(callee)?,
            argument: "usize".into(),
        };

        Some(PathRes {
            kind: crate::size_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&record).ok()?,
        })
    }

    /// `u32::MAX`, with its evaluated value.
    pub(super) fn primitive_constant_resolution(&self, definition: &Definition) -> Option<PathRes> {
        if !self
            .reader
            .profile
            .supports(crate::primitive_constants::CAPABILITY)
            || identity(definition, "def_kind")? != "AssocConst"
            || definition.scope == Scope::Unit
            || identity(definition, "crate_name")? != "core"
            || definition.name.rsplit("::").next() != Some("MAX")
        {
            return None;
        }
        // The unit records a library constant's value only for a primitive's own constants.
        let value = self.constant_value(definition)?;
        let ty = self.reader.ty(value.ty.as_deref()?).ok()?;
        if ty.kind != TypeKind::Integer || ty.spelling != "u32" {
            return None;
        }
        let constant = crate::primitive_constants::PrimitiveConstant {
            definition: self.library_record(definition)?,
            primitive: "u32".into(),
            member: "MAX".into(),
            value: value.value.value.clone()?,
        };

        Some(PathRes {
            kind: crate::primitive_constants::RESOLUTION.into(),
            target: None,
            description: serde_json::to_string(&constant).ok()?,
        })
    }

    /// A local nongeneric constant, or an inherent associated one.
    pub(super) fn local_constant(&self, definition: &Definition) -> bool {
        let kind = identity(definition, "def_kind");
        let generic = definition
            .extensions
            .get("rust:compiler-identity")
            .and_then(|identity| identity.get("generics")?.get("count")?.as_u64())
            .unwrap_or(0);
        definition.scope == Scope::Unit
            && generic == 0
            && (kind == Some("Const")
                || kind == Some("AssocConst") && identity(definition, "inherent_impl").is_some())
    }

    pub(super) fn constant_value(&self, definition: &Definition) -> Option<&'a Value> {
        self.reader
            .unit
            .values
            .iter()
            .find(|value| value.definition.as_deref() == Some(definition.id.as_str()))
    }
}

/// The token of a fixed-length array of bools.
fn bool_array(
    context: &Context<'_, '_>,
    element: &str,
    ty: &rstyle_core::cpg::model::Type,
) -> Option<String> {
    let element = context.reader.ty(element).ok()?;
    if ty.kind != TypeKind::Sequence || ty.fixed != Some(true) || element.kind != TypeKind::Bool {
        return None;
    }
    TypeToken::Array {
        element: Box::new(TypeToken::Scalar("bool".into())),
        length: ty.length?,
    }
    .text()
    .ok()
}

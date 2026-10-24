//! Required externally established source, definition, build and nominal authority.
use super::{CoreConstant, ProductConstant, SourceSite};
use crate::{HirNode, data_library::Callee};

pub struct BuildContext<'a> {
    /// Implementor must validate compiler/target authority before returning this context.
    pub pointer_width: u64,
    pub cfgs: &'a [String],
}

pub struct SourceUnit<'a> {
    pub file: &'a str,
    /// Complete immutable original file, whose offsets belong to the borrowed AST.
    pub bytes: &'a str,
    pub implementation: &'a syn::ItemImpl,
}

pub struct SourceSelection<'a> {
    pub constant_unit: SourceUnit<'a>,
    pub constant: &'a syn::ImplItemConst,
    pub constructor_unit: SourceUnit<'a>,
    pub constructor: &'a syn::ImplItemFn,
}

/// No method has an accepting default. Wire observations cannot instantiate this proof.
pub trait Authority {
    /// Bind the actual original caller use, selected local constant and compiled context.
    fn actual(&self, node: &HirNode, record: &ProductConstant) -> Result<(), String>;
    fn build(&self) -> Result<BuildContext<'_>, String>;
    fn sources(&self, record: &ProductConstant) -> Result<SourceSelection<'_>, String>;
    /// Bind exact original source span to its independently captured local definition/build.
    fn local(&self, definition: &Callee, source: &SourceSite) -> Result<(), String>;
    /// Resolve owner/core paths and the selected constructor/tuple constructor identities.
    fn bindings(
        &self,
        record: &ProductConstant,
        source: &SourceSelection<'_>,
    ) -> Result<(), String>;
    /// Establish complete nominal identity, scalar fields, Copy and no-drop source closure.
    fn owned_copy(&self, record: &ProductConstant) -> Result<(), String>;
    /// Check a reviewed core legacy i32 MAX definition, compiler and target authority.
    fn core_constant(&self, constant: &CoreConstant) -> Result<(), String>;
}

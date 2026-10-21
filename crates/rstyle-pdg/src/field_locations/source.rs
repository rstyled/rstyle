//! Source-first verification, separate from the compiler-field capture walk.

use super::*;
use crate::source_evidence::byte_offset;
use syn::{spanned::Spanned, visit::Visit};

#[derive(Default)]
struct Inventory<'a> {
    fields: Vec<&'a syn::ExprField>,
    bindings: Vec<String>,
    has_macros: bool,
    facts: Option<&'a dyn crate::unit_store::EvidenceFacts>,
    configuration: Option<crate::conditional_source::Configuration<'a>>,
}

impl<'a> Visit<'a> for Inventory<'a> {
    fn visit_expr(&mut self, expression: &'a syn::Expr) {
        if !self.inactive(crate::conditional_source::expression_attributes(expression)) {
            syn::visit::visit_expr(self, expression);
        }
    }

    fn visit_local(&mut self, local: &'a syn::Local) {
        if !self.inactive(&local.attrs) {
            syn::visit::visit_local(self, local);
        }
    }

    fn visit_stmt(&mut self, statement: &'a syn::Stmt) {
        if let syn::Stmt::Macro(statement) = statement
            && self.inactive(&statement.attrs)
        {
            return;
        }
        syn::visit::visit_stmt(self, statement);
    }

    fn visit_arm(&mut self, arm: &'a syn::Arm) {
        if !self.inactive(&arm.attrs) {
            syn::visit::visit_arm(self, arm);
        }
    }
    fn visit_expr_field(&mut self, field: &'a syn::ExprField) {
        syn::visit::visit_expr_field(self, field);
        self.fields.push(field);
    }

    fn visit_pat_ident(&mut self, pattern: &'a syn::PatIdent) {
        self.bindings.push(pattern.ident.to_string());
        syn::visit::visit_pat_ident(self, pattern);
    }

    fn visit_expr_assign(&mut self, assignment: &'a syn::ExprAssign) {
        self.bindings.extend(written_roots(&assignment.left));
        syn::visit::visit_expr_assign(self, assignment);
    }

    fn visit_expr_reference(&mut self, reference: &'a syn::ExprReference) {
        if reference.mutability.is_some() {
            self.bindings.extend(written_roots(&reference.expr));
        }
        syn::visit::visit_expr_reference(self, reference);
    }

    fn visit_expr_binary(&mut self, binary: &'a syn::ExprBinary) {
        if matches!(
            binary.op,
            syn::BinOp::AddAssign(_)
                | syn::BinOp::SubAssign(_)
                | syn::BinOp::MulAssign(_)
                | syn::BinOp::DivAssign(_)
                | syn::BinOp::RemAssign(_)
                | syn::BinOp::BitXorAssign(_)
                | syn::BinOp::BitAndAssign(_)
                | syn::BinOp::BitOrAssign(_)
                | syn::BinOp::ShlAssign(_)
                | syn::BinOp::ShrAssign(_)
        ) {
            self.bindings.extend(written_roots(&binary.left));
        }
        syn::visit::visit_expr_binary(self, binary);
    }

    fn visit_macro(&mut self, mac: &'a syn::Macro) {
        if !self.simple_assertion(mac) {
            self.has_macros = true;
        }
    }

    fn visit_item(&mut self, _: &'a syn::Item) {
        // Nested items own their own field inventory.
    }
}

impl Inventory<'_> {
    fn simple_assertion(&self, mac: &syn::Macro) -> bool {
        let Some(facts) = self.facts else {
            return false;
        };
        if !crate::extraction_profile::facts_support(facts, "rust.accelerator-packing") {
            return false;
        }

        let span = ByteSpan {
            start: byte_offset(&facts.source.bytes, mac.span().start()) as u32,
            end: byte_offset(&facts.source.bytes, mac.span().end()) as u32,
        };
        let sources = crate::field_locations::operations(&facts.body, &["expr_assert"]);
        sources.into_iter().any(|node| {
            node.span == span
                && node
                    .children
                    .iter()
                    .all(|child| matches!(child.kind.as_str(), "expr_path" | "expr_lit"))
                && crate::assertions::verify(
                    node,
                    &node.snippet,
                    facts.build.debug_assertions.value,
                    crate::extraction_profile::ExtractionProfile::from_facts(facts).unwrap(),
                )
                .is_ok()
        })
    }

    fn inactive(&self, attributes: &[syn::Attribute]) -> bool {
        !attributes.is_empty()
            && self
                .configuration
                .as_ref()
                .is_some_and(|configuration| configuration.active(attributes) == Ok(false))
    }
}

fn written_roots(expression: &syn::Expr) -> Vec<String> {
    match expression {
        syn::Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            vec![path.path.segments[0].ident.to_string()]
        }
        syn::Expr::Field(field) => written_roots(&field.base),
        syn::Expr::Index(index) => written_roots(&index.expr),
        syn::Expr::Unary(unary) => written_roots(&unary.expr),
        syn::Expr::Paren(paren) => written_roots(&paren.expr),
        syn::Expr::Group(group) => written_roots(&group.expr),
        syn::Expr::Tuple(tuple) => tuple.elems.iter().flat_map(written_roots).collect(),
        syn::Expr::Array(array) => array.elems.iter().flat_map(written_roots).collect(),
        _ => Vec::new(),
    }
}

pub(super) fn verify(
    facts: &dyn crate::unit_store::EvidenceFacts,
    observations: &FieldObservations,
    interface: &InterfaceObservations,
) -> Result<(), String> {
    let function: syn::ItemFn =
        syn::parse_str(&facts.source.bytes).map_err(|error| error.to_string())?;
    let mut inventory = Inventory {
        facts: Some(facts),
        configuration: crate::extraction_profile::ExtractionProfile::from_facts(facts)?
            .supports("rust.cfg-source-removal")
            .then(|| crate::conditional_source::Configuration::new(&facts.build.cfgs)),
        ..Inventory::default()
    };
    inventory.visit_block(&function.block);
    if inventory.has_macros && !inventory.fields.is_empty() {
        return Err("Macro effects prevent the formal-only field origin proof".into());
    }
    let hir = fields(&facts.body);
    if inventory.fields.len() != observations.operations.len()
        || hir.len() != observations.operations.len()
    {
        return Err("Owned source/HIR/field observation inventories disagree".into());
    }
    let verification = Verification {
        facts,
        observations,
        interface,
        bindings: &inventory.bindings,
    };
    for (index, field) in inventory.fields.into_iter().enumerate() {
        let (operation, _) = verification.correspondence(field)?;
        if observations.operations[index] != *operation {
            return Err("Field observations contradict source base-before-projection order".into());
        }
        verification.field(field)?;
    }

    Ok(())
}

struct Verification<'a> {
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    observations: &'a FieldObservations,
    interface: &'a InterfaceObservations,
    bindings: &'a [String],
}

impl Verification<'_> {
    fn field(&self, field: &syn::ExprField) -> Result<(), String> {
        let (operation, node) = self.correspondence(field)?;
        if crate::data_value_fields::candidate(node, self.facts) {
            if !matches!(operation.location, Location::Unsupported { .. }) {
                return Err("Aggregate value projection invents a formal memory location".into());
            }
            return crate::data_value_fields::index(node, self.facts).map(|_| ());
        }
        if derived::candidate(node, self.facts) {
            if !matches!(operation.location, Location::Unsupported { .. }) {
                return Err("Indirect field invents one captured origin".into());
            }
            return derived::locations(node, self.facts).map(|_| ());
        }
        let source = source_path(field)?;
        if self.bindings.iter().any(|binding| binding == &source.root) {
            return self.alias(node, operation, &source);
        }
        let (parameter, ty) = formal(&source.root, self.facts, self.interface)?;
        let (path, value) = source_projection(ty, &source.members)?;
        let expected = Location::Captured {
            parameter: parameter.position,
            path,
            value,
        };
        if operation.location != expected {
            return Err(
                "Field location, nominal identity or result contradicts source projection".into(),
            );
        }

        check_resolution(node, parameter, &expected)
    }

    fn alias(
        &self,
        node: &HirNode,
        operation: &FieldOperation,
        source: &SourcePath,
    ) -> Result<(), String> {
        if !crate::extraction_profile::ExtractionProfile::from_facts(self.facts)?
            .supports("rust.shared-local-field-reads")
        {
            return Err("Local field roots require their registered capability".into());
        }

        let root = hir_root(node)?;
        let origin = self.alias_origin(root, &source.root)?;
        let Some(Observation::Captured { ty }) =
            self.interface.parameters.get(origin.parameter as usize)
        else {
            return Err("Shared field alias lacks its concrete formal root".into());
        };
        let referent = origin_value(ty, &origin.path)?;
        let expected = crate::data_types::TypeToken::Shared(Box::new(
            crate::data_types::TypeToken::from_tree(&referent),
        ));
        if crate::data_types::parse(crate::data_aggregates::token(root)?)? != expected {
            return Err("Shared field alias disagrees with its original referent type".into());
        }

        let (suffix, value) = source_projection(&referent, &source.members)?;
        let expected = Location::Captured {
            parameter: origin.parameter,
            path: origin.path.iter().cloned().chain(suffix).collect(),
            value,
        };
        if operation.location != expected {
            return Err(
                "Shared local field location contradicts its source origin and members".into(),
            );
        }
        check_field_resolution(node, &expected)
    }

    fn alias_origin(
        &self,
        root: &HirNode,
        name: &str,
    ) -> Result<crate::data_origins::Origin, String> {
        let target = root
            .res
            .as_ref()
            .and_then(|resolution| resolution.target.as_ref());
        if !self.facts.bindings.iter().any(|binding| {
            Some(&binding.hir) == target
                && !binding.is_param
                && !binding.is_mut
                && binding.name == name
        }) {
            return Err("Source field alias lacks its immutable local binding identity".into());
        }

        let origins = crate::data_origins::origins(root, self.facts)?;
        let [origin] = origins.as_slice() else {
            return Err("Shared field alias needs one exact formal origin".into());
        };
        Ok(origin.clone())
    }

    fn correspondence(
        &self,
        field: &syn::ExprField,
    ) -> Result<(&FieldOperation, &HirNode), String> {
        let span = field.span();
        let span = ByteSpan {
            start: byte_offset(&self.facts.source.bytes, span.start()) as u32,
            end: byte_offset(&self.facts.source.bytes, span.end()) as u32,
        };
        let operations: Vec<_> = self
            .observations
            .operations
            .iter()
            .filter(|operation| operation.span == span)
            .collect();
        let [operation] = operations.as_slice() else {
            return Err("Source field has missing or duplicate observation correspondence".into());
        };
        let candidates: Vec<_> = fields(&self.facts.body)
            .into_iter()
            .filter(|node| node.span == span && node.hir == operation.hir)
            .collect();
        let [node] = candidates.as_slice() else {
            return Err("Source field has no unique exact HIR correspondence".into());
        };
        self.navigation(node, operation)?;

        Ok((operation, node))
    }

    fn navigation(&self, node: &HirNode, operation: &FieldOperation) -> Result<(), String> {
        if node.expansion.is_some()
            || operation.snippet != node.snippet
            || self
                .facts
                .source
                .bytes
                .get(operation.span.start as usize..operation.span.end as usize)
                != Some(&operation.snippet)
        {
            return Err("Field source navigation or expansion provenance disagrees".into());
        }

        Ok(())
    }
}

fn formal<'a>(
    name: &str,
    facts: &'a HirFnFacts,
    interface: &'a InterfaceObservations,
) -> Result<(&'a crate::ParamFact, &'a TypeTree), String> {
    let candidates: Vec<_> = facts
        .signature
        .params
        .iter()
        .enumerate()
        .filter(|(_, parameter)| parameter.pat.snippet == name)
        .collect();
    let [(index, parameter)] = candidates.as_slice() else {
        return Err("Source field root lacks unique named formal correspondence".into());
    };
    let Some(Observation::Captured { ty }) = interface.parameters.get(*index) else {
        return Err("Source field root lacks captured concrete type".into());
    };

    Ok((parameter, ty))
}

struct SourcePath {
    root: String,
    members: Vec<syn::Member>,
}

fn source_path(field: &syn::ExprField) -> Result<SourcePath, String> {
    let mut members = vec![field.member.clone()];
    let mut base = crate::data_aggregates::ungroup(&field.base);
    while let syn::Expr::Field(field) = base {
        members.push(field.member.clone());
        base = crate::data_aggregates::ungroup(&field.base);
    }
    let syn::Expr::Path(root) = base else {
        return Err("Source field path is outside the formal projection grammar".into());
    };
    if root.qself.is_some() || root.path.leading_colon.is_some() || root.path.segments.len() != 1 {
        return Err("Source field root is not a simple formal binding".into());
    }
    members.reverse();

    Ok(SourcePath {
        root: root.path.segments[0].ident.to_string(),
        members,
    })
}

fn source_projection(
    root: &TypeTree,
    members: &[syn::Member],
) -> Result<(Vec<Projection>, TypeTree), String> {
    let mut path = Vec::new();
    let mut value = root;
    for member in members {
        while let TypeTree::Shared(referent) = value {
            path.push(Projection::SharedReferent);
            value = referent;
        }
        let (projection, next) = source_member(value, member)?;
        path.push(projection);
        value = next;
    }

    Ok((path, value.clone()))
}

fn source_member<'a>(
    owner: &'a TypeTree,
    member: &syn::Member,
) -> Result<(Projection, &'a TypeTree), String> {
    match (owner, member) {
        (
            TypeTree::Struct {
                nominal, fields, ..
            },
            member,
        ) => {
            let name = match member {
                syn::Member::Named(name) => name.to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            let matches: Vec<_> = fields.iter().filter(|field| field.name == name).collect();
            let [field] = matches.as_slice() else {
                return Err("Source member lacks unique declared field correspondence".into());
            };
            Ok((
                Projection::StructField {
                    nominal: nominal.clone(),
                    position: field.position,
                    name,
                },
                &field.ty,
            ))
        }
        (TypeTree::Tuple(elements), syn::Member::Unnamed(index)) => {
            let position = index.index as usize;
            Ok((
                Projection::TupleField { position },
                elements
                    .get(position)
                    .ok_or("Source tuple member is out of range")?,
            ))
        }
        _ => Err("Source member contradicts supported concrete owner".into()),
    }
}

fn check_resolution(
    node: &HirNode,
    parameter: &crate::ParamFact,
    location: &Location,
) -> Result<(), String> {
    check_root(node, parameter)?;
    check_field_resolution(node, location)
}

fn check_field_resolution(node: &HirNode, location: &Location) -> Result<(), String> {
    let Location::Captured { path, value, .. } = location else {
        unreachable!()
    };
    let (position, owner_path) = match path.last() {
        Some(Projection::StructField {
            nominal, position, ..
        }) => (*position, Some(nominal.definition_path.clone())),
        Some(Projection::TupleField { position }) => (*position, None),
        _ => return Err("Missing source-backed terminal field projection".into()),
    };
    let res = node
        .res
        .as_ref()
        .filter(|res| res.kind == "resolved_field")
        .ok_or("Missing resolved HIR field evidence")?;
    let resolution: FieldResolution =
        serde_json::from_str(&res.description).map_err(|error| error.to_string())?;
    if resolution
        != (FieldResolution {
            position,
            owner_path,
        })
    {
        return Err("Resolved HIR field position/owner contradicts source projection".into());
    }
    if let TypeTree::Scalar(token) = value
        && node.ty.as_ref().and_then(|ty| ty.normalized.as_ref()) != Some(token)
    {
        return Err("HIR scalar field type contradicts independently checked result".into());
    }

    Ok(())
}

pub(super) fn origin_value<'a>(
    value: &'a TypeTree,
    path: &[Projection],
) -> Result<std::borrow::Cow<'a, TypeTree>, String> {
    if let Some((
        Projection::ByteWindow {
            operand_position: 0,
            length: 8,
        },
        prefix,
    )) = path.split_last()
    {
        let referent = base_value(value, prefix)?;
        if *referent != TypeTree::Slice(Box::new(TypeTree::Scalar("u8".into()))) {
            return Err("Borrowed byte window requires a shared byte-slice origin".into());
        }
        return Ok(std::borrow::Cow::Owned(TypeTree::Array {
            element: Box::new(TypeTree::Scalar("u8".into())),
            length: 8,
        }));
    }
    base_value(value, path).map(std::borrow::Cow::Borrowed)
}

fn base_value<'a>(mut value: &'a TypeTree, path: &[Projection]) -> Result<&'a TypeTree, String> {
    for step in path {
        value = match (value, step) {
            (TypeTree::Shared(referent), Projection::SharedReferent) => referent,
            (TypeTree::Slice(element), Projection::CheckedElement) => element,
            (TypeTree::Tuple(elements), Projection::TupleField { position }) => elements
                .get(*position)
                .ok_or("Shared alias tuple origin is out of range")?,
            (
                TypeTree::Struct {
                    nominal: owner,
                    fields,
                    ..
                },
                Projection::StructField {
                    nominal,
                    position,
                    name,
                },
            ) if owner == nominal => {
                let field = fields
                    .get(*position)
                    .filter(|field| field.position == *position && field.name == *name)
                    .ok_or("Shared alias nominal origin contradicts its declaration")?;
                &field.ty
            }
            _ => return Err("Shared alias origin contradicts its formal concrete type".into()),
        };
    }
    Ok(value)
}

fn hir_root(mut node: &HirNode) -> Result<&HirNode, String> {
    while node.kind == "expr_field" {
        let [base] = node.children.as_slice() else {
            return Err("Shared alias field lacks its exact base".into());
        };
        node = base;
    }
    if node.kind != "expr_path" {
        return Err("Shared alias field root must be a local binding".into());
    }
    Ok(node)
}

fn check_root(node: &HirNode, parameter: &crate::ParamFact) -> Result<(), String> {
    let mut base = node;
    while base.kind == "expr_field" {
        let [child] = base.children.as_slice() else {
            return Err("HIR field base correspondence is incomplete".into());
        };
        base = child;
    }
    if base.kind != "expr_path"
        || base
            .res
            .as_ref()
            .filter(|res| res.kind == "local")
            .and_then(|res| res.target.as_ref())
            != Some(&parameter.pat.hir)
    {
        return Err("Resolved HIR field root contradicts source formal identity".into());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independent_shared_tuple_projection_retains_both_referents() {
        let root = TypeTree::Shared(Box::new(TypeTree::Tuple(vec![
            TypeTree::Scalar("u16".into()),
            TypeTree::Shared(Box::new(TypeTree::Tuple(vec![TypeTree::Scalar(
                "i32".into(),
            )]))),
        ])));
        let expression: syn::ExprField = syn::parse_str("input.1.0").unwrap();
        let source = source_path(&expression).unwrap();
        assert_eq!(source.root, "input");
        let (path, result) = source_projection(&root, &source.members).unwrap();
        assert_eq!(
            path,
            [
                Projection::SharedReferent,
                Projection::TupleField { position: 1 },
                Projection::SharedReferent,
                Projection::TupleField { position: 0 },
            ]
        );
        assert_eq!(result, TypeTree::Scalar("i32".into()));
        let expression: syn::ExprField = syn::parse_str("input.2").unwrap();
        let source = source_path(&expression).unwrap();
        assert!(source_projection(&root, &source.members).is_err());
    }

    #[test]
    fn nested_field_inventory_is_independently_base_first() {
        let source = "{ input.1.0; }";
        let block: syn::Block = syn::parse_str(source).unwrap();
        let mut inventory = Inventory::default();
        inventory.visit_block(&block);
        let snippets: Vec<_> = inventory
            .fields
            .iter()
            .map(|field| {
                let span = field.span();
                &source[byte_offset(source, span.start())..byte_offset(source, span.end())]
            })
            .collect();
        assert_eq!(snippets, ["input.1", "input.1.0"]);
    }

    #[test]
    fn source_formal_mutations_and_macros_cannot_prove_stable_origins() {
        let source = "{ x = y; x.value += 1; let _ = &mut other; alter!(); }";
        let block: syn::Block = syn::parse_str(source).unwrap();
        let mut inventory = Inventory::default();
        inventory.visit_block(&block);
        assert_eq!(inventory.bindings, ["x", "x", "other"]);
        assert!(inventory.has_macros);
    }

    #[test]
    fn source_inventory_owns_nested_items_separately_and_retains_cfg_source() {
        let function: syn::ItemFn = syn::parse_str(
            "fn outer(x: &T) { fn inner(y: &T) { y.value; } #[cfg(any())] { x.value; } x.other; }",
        )
        .unwrap();
        let mut inventory = Inventory::default();
        inventory.visit_block(&function.block);
        assert_eq!(inventory.fields.len(), 2);
        assert_eq!(source_path(inventory.fields[0]).unwrap().root, "x");
        assert_eq!(source_path(inventory.fields[1]).unwrap().root, "x");
    }

    #[test]
    fn source_roots_with_aliases_or_executable_bases_are_unavailable() {
        for expression in ["get().value", "(*x).value", "x[0].value", "module::x.value"] {
            let field: syn::ExprField = syn::parse_str(expression).unwrap();
            assert!(source_path(&field).is_err(), "{expression}");
        }
    }
}

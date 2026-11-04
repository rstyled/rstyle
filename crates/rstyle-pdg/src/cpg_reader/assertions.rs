//! Standard assertions, collapsed to the record the data profile observes for them.

use super::Context;
use crate::assertions::Assertion;
use crate::frontend::{HirNode, PathRes};
use rstyle_core::cpg::model::{Node, Origin};

impl<'a> Context<'_, 'a> {
    /// An expansion of a standard library assertion macro, with its condition (or
    /// compared operands) and the locals its message captures.
    pub(super) fn assertion(&self, node: &'a Node) -> Result<Option<HirNode>, String> {
        if !self.reader.profile.supports("rust.scalar-assertions")
            || node.provenance.origin != Origin::MacroExpansion
        {
            return Ok(None);
        }
        let Some(name) = self.core_macro(node) else {
            return Ok(None);
        };
        let base = self.node(node, "expr_assert")?;
        let Some(expansion) = base.expansion.clone() else {
            return Ok(None);
        };
        let source = expansion.snippet.as_str();
        let Ok(assertion) = Assertion::parse(source) else {
            return Ok(None);
        };
        if assertion.name != name {
            return Ok(None);
        }
        let debug = self.reader.unit.semantics.debug_assertions.unwrap_or(false);
        let active = !assertion.name.starts_with("debug_") || debug;
        let Some(children) =
            self.assertion_operands(node, (source, base.span.start), &assertion)?
        else {
            return Ok(None);
        };
        Ok(Some(HirNode {
            ty: self.ty(node)?,
            res: Some(PathRes {
                kind: "standard_assertion".into(),
                target: None,
                description: format!("core::{}", assertion.name),
            }),
            lit: Some(active.to_string()),
            children,
            ..base
        }))
    }

    /// The name of the `core` macro a node's expansion comes from.
    fn core_macro(&self, node: &Node) -> Option<&'a str> {
        let definition = self
            .reader
            .tables
            .definition(node.provenance.macro_definition.as_deref()?)
            .ok()?;
        let crate_name = definition
            .extensions
            .get("rust:compiler-identity")?
            .get("crate_name")?
            .as_str()?;
        (crate_name == "core").then_some(())?;
        definition
            .stable_identity
            .trim_end_matches('!')
            .rsplit("::")
            .next()
    }

    fn assertion_operands(
        &self,
        node: &'a Node,
        (source, offset): (&str, u32),
        assertion: &Assertion,
    ) -> Result<Option<Vec<HirNode>>, String> {
        if let Some(right) = assertion.right_span(source, offset) {
            if !self.reader.profile.supports("rust.accelerator-packing") {
                return Ok(None);
            }
            let spans = [assertion.condition_span(source, offset), right];
            let mut operands = Vec::new();
            for span in spans {
                let found = self.within(node, &|candidate| {
                    candidate.provenance.origin == Origin::Source && self.written(candidate) == span
                })?;
                let [operand] = found.as_slice() else {
                    return Ok(None);
                };
                operands.push(self.expression(operand)?);
            }
            return Ok(Some(operands));
        }

        let debug = self.reader.unit.semantics.debug_assertions.unwrap_or(false);
        let condition = match assertion.name {
            "debug_assert" => self.debug_condition(node, debug)?,
            _ => self.assert_condition(node)?,
        };
        let Some(condition) = condition else {
            return Ok(None);
        };
        let mut children = vec![self.expression(condition)?];
        if let Some(diagnostic) = assertion.diagnostic.as_ref() {
            if !self.reader.profile.supports("rust.end-of-input") {
                return Ok(None);
            }
            for capture in &diagnostic.captures {
                let expected = crate::assertions::formatting::span(source, offset, capture);
                let found = self.within(node, &|candidate| {
                    candidate.original_kind == "expr_path"
                        && candidate
                            .span
                            .as_ref()
                            .is_some_and(|span| self.span(span) == expected)
                })?;
                let [operand] = found.as_slice() else {
                    return Ok(None);
                };
                children.push(self.expression(operand)?);
            }
        }
        Ok(Some(children))
    }

    /// `if !condition { .. }`, without an else branch.
    fn assert_condition(&self, node: &'a Node) -> Result<Option<&'a Node>, String> {
        if node.original_kind != "expr_if" {
            return Ok(None);
        }
        let children = self.body.children(node)?;
        let [test, _] = children.as_slice() else {
            return Ok(None);
        };
        if test.original_kind != "expr_unary" || test.operator.as_deref() != Some("not") {
            return Ok(None);
        }
        Ok(self.body.children(test)?.first().copied())
    }

    /// `if <debug assertions> { if !condition { .. }; }`.
    fn debug_condition(&self, node: &'a Node, active: bool) -> Result<Option<&'a Node>, String> {
        if node.original_kind != "expr_if" {
            return Ok(None);
        }
        let children = self.body.children(node)?;
        let [test, body] = children.as_slice() else {
            return Ok(None);
        };
        let enabled = test.original_kind == "expr_lit"
            && test.value.as_ref().is_some_and(|value| {
                value.kind == "bool" && value.value.as_deref() == Some(&active.to_string())
            });
        if !enabled || body.original_kind != "expr_block" {
            return Ok(None);
        }
        let statements = self.body.children(body)?;
        let [statement] = statements.as_slice() else {
            return Ok(None);
        };
        if statement.original_kind != "stmt_semi" {
            return Ok(None);
        }
        match self.body.children(statement)?.as_slice() {
            [inner] => self.assert_condition(inner),
            _ => Ok(None),
        }
    }

    /// The nodes of a subtree, its root included, that `matches` selects, in pre-order.
    fn within(
        &self,
        node: &'a Node,
        matches: &dyn Fn(&Node) -> bool,
    ) -> Result<Vec<&'a Node>, String> {
        let mut found = Vec::new();
        let mut pending = vec![node];
        while let Some(next) = pending.pop() {
            if matches(next) {
                found.push(next);
            }
            pending.extend(self.body.children(next)?.into_iter().rev());
        }
        Ok(found)
    }
}

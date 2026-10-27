//! Complete source/HIR correspondence and independently assigned value roles.
use super::{Result, Site, site, source};
use crate::{ByteSpan, HirFnFacts, HirNode, HirRef};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Role {
    Bytes,
    Usize,
    Text,
    Bool,
    Unit,
    Never,
    Returned,
    Result,
    Error,
}

pub(super) struct Roles {
    pub values: BTreeMap<Site, Role>,
    pub length: HirRef,
    pub condition: HirRef,
    pub error: HirRef,
    pub failure: HirRef,
    pub returned: HirRef,
    pub success: HirRef,
}

struct Shape<'a> {
    kind: &'a str,
    span: ByteSpan,
    role: Option<Role>,
}

impl<'a> Shape<'a> {
    fn value(kind: &'a str, span: ByteSpan, role: Role) -> Self {
        Self {
            kind,
            span,
            role: Some(role),
        }
    }

    fn statement(kind: &'a str, span: ByteSpan) -> Self {
        Self {
            kind,
            span,
            role: None,
        }
    }
}

struct Flow<'a> {
    facts: &'a HirFnFacts,
    values: BTreeMap<Site, Role>,
    seen: std::collections::BTreeSet<Site>,
}

pub(super) fn verify(facts: &HirFnFacts, source: &source::Checked) -> Result<Roles> {
    Flow {
        facts,
        values: BTreeMap::new(),
        seen: Default::default(),
    }
    .prove(source)
}

impl<'f> Flow<'f> {
    fn prove(mut self, source: &source::Checked) -> Result<Roles> {
        let (selection, success) = self.selection(source)?;
        let (condition, length, failure_block) = self.guard(selection, source)?;
        let (returned, failure, error) = self.branches(failure_block, success, source)?;

        Ok(Roles {
            values: self.values,
            length: length.hir.clone(),
            condition: condition.hir.clone(),
            error: error.hir.clone(),
            failure: failure.hir.clone(),
            returned: returned.hir.clone(),
            success: success.hir.clone(),
        })
    }

    fn selection(&mut self, source: &source::Checked) -> Result<(&'f HirNode, &'f HirNode)> {
        self.parameters(source)?;
        let facts = self.facts;

        self.node(
            &facts.body,
            Shape::value("expr_block", source.body, Role::Result),
        )?;
        let [statement, success] = self.children(&facts.body)?;

        self.node(statement, Shape::statement("stmt_expr", source.selection))?;
        let [selection] = self.children(statement)?;

        Ok((selection, success))
    }

    fn guard<'a>(
        &mut self,
        selection: &'a HirNode,
        source: &source::Checked,
    ) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode)> {
        self.node(
            selection,
            Shape::value("expr_if", source.selection, Role::Unit),
        )?;
        let [condition, failure] = self.children(selection)?;
        let length = self.condition(condition, source)?;
        self.comparison(condition)?;

        Ok((condition, length, failure))
    }

    fn condition<'a>(
        &mut self,
        condition: &'a HirNode,
        source: &source::Checked,
    ) -> Result<&'a HirNode> {
        self.node(
            condition,
            Shape::value("expr_binary", source.condition.expression, Role::Bool),
        )?;
        let [length, minimum] = self.children(condition)?;
        self.length(length, source)?;
        self.local(minimum, source.condition.minimum, Role::Usize)?;

        Ok(length)
    }

    fn branches<'a>(
        &mut self,
        failure: &'a HirNode,
        success: &HirNode,
        source: &source::Checked,
    ) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode)> {
        let result = self.failure(failure, source)?;
        self.success(success, source)?;

        Ok(result)
    }

    fn node(&mut self, node: &HirNode, shape: Shape<'_>) -> Result<()> {
        let Shape { kind, span, role } = shape;
        if node.kind != kind
            || node.span != span
            || node.expansion.is_some()
            || node.is_unsafe_block
            || node.hir.krate != 0
            || node.hir.owner != self.facts.body.hir.owner
            || !self.seen.insert(site(&node.hir))
            || self
                .facts
                .source
                .bytes
                .get(span.start as usize..span.end as usize)
                != Some(node.snippet.as_str())
            || node.ty.is_some() != role.is_some()
            || node.lit.is_some()
            || kind != "pat_binding"
                && (node.binding.is_some()
                    || node.binding_mut.is_some()
                    || node.param_index.is_some())
            || !matches!(kind, "expr_path" | "expr_call" | "expr_method_call") && node.res.is_some()
            || !matches!(kind, "expr_call" | "expr_binary" | "expr_method_call")
                && node.dispatch.is_some()
        {
            return Err(
                "Slice instance changes complete original source/HIR correspondence".into(),
            );
        }
        if let Some(role) = role {
            self.values.insert(site(&node.hir), role);
        }
        Ok(())
    }

    fn children<'a, const N: usize>(&self, node: &'a HirNode) -> Result<&'a [HirNode; N]> {
        if !self.seen.contains(&site(&node.hir)) {
            return Err("Slice child traversal requires its checked original parent".into());
        }

        node.children
            .as_slice()
            .try_into()
            .map_err(|_| "Slice instance original HIR child roster differs".into())
    }

    fn parameters(&mut self, source: &source::Checked) -> Result<()> {
        if self.facts.signature.params.len() != 3 || self.facts.bindings.len() != 3 {
            return Err("Slice instance requires exactly three original formal bindings".into());
        }
        for (index, role) in [Role::Bytes, Role::Usize, Role::Text]
            .into_iter()
            .enumerate()
        {
            let parameter = &self.facts.signature.params[index];
            let expected = &source.parameters[index];
            let binding = &self.facts.bindings[index];
            self.node(
                &parameter.pat,
                Shape::value("pat_binding", expected.name_span, role),
            )?;
            let _: &[HirNode; 0] = self.children(&parameter.pat)?;
            if parameter.position != index as u32
                || parameter.name_span != expected.name_span
                || parameter.ty_span != expected.ty_span
                || parameter.pat.param_index != Some(index as u32)
                || parameter.pat.binding.as_ref() != Some(&parameter.symbol)
                || parameter.pat.binding_mut != Some(false)
                || parameter.pat.res.is_some()
                || parameter.pat.dispatch.is_some()
                || parameter.pat.lit.is_some()
                || binding.hir != parameter.pat.hir
                || binding.symbol != parameter.symbol
                || binding.name != expected.name
                || binding.decl_span != expected.name_span
                || !binding.is_param
                || binding.is_mut
                || self
                    .facts
                    .source
                    .bytes
                    .get(expected.ty_span.start as usize..expected.ty_span.end as usize)
                    != Some(parameter.ty.spelling.as_str())
            {
                return Err("Slice instance original parameter/binding identity differs".into());
            }
        }
        Ok(())
    }

    fn local(&mut self, node: &HirNode, span: ByteSpan, role: Role) -> Result<()> {
        let index = match role {
            Role::Bytes => 0,
            Role::Usize => 1,
            Role::Text => 2,
            _ => return Err("Local slice proof has no formal role".into()),
        };

        self.node(node, Shape::value("expr_path", span, role))?;
        let _: &[HirNode; 0] = self.children(node)?;
        let parameter = &self.facts.signature.params[index];
        let resolution = node
            .res
            .as_ref()
            .ok_or("Slice formal use lacks resolution")?;

        if resolution.kind != "local"
            || resolution.target.as_ref() != Some(&parameter.pat.hir)
            || node.dispatch.is_some()
            || node.binding.is_some()
            || node.binding_mut.is_some()
            || node.param_index.is_some()
            || node.lit.is_some()
        {
            return Err("Slice instance formal use resolves to another value".into());
        }
        Ok(())
    }

    fn length(&mut self, node: &HirNode, source: &source::Checked) -> Result<()> {
        self.node(
            node,
            Shape::value("expr_method_call", source.condition.length, Role::Usize),
        )?;
        let [receiver] = self.children(node)?;
        self.local(receiver, source.condition.receiver, Role::Bytes)
    }

    fn comparison(&self, node: &HirNode) -> Result<()> {
        let dispatch = node
            .dispatch
            .as_ref()
            .ok_or("Slice guard lacks builtin comparison")?;
        if dispatch.kind != "builtin"
            || dispatch.op != "<"
            || dispatch.operand_tys != ["usize", "usize"]
            || dispatch.result_ty != "bool"
            || node.res.is_some()
        {
            return Err(
                "Slice guard must compare builtin usize values using strict less-than".into(),
            );
        }
        Ok(())
    }

    fn failure<'a>(
        &mut self,
        node: &'a HirNode,
        source: &source::Checked,
    ) -> Result<(&'a HirNode, &'a HirNode, &'a HirNode)> {
        let returned = self.returned(node, source)?;
        let (failure, error) = self.error(returned, source)?;

        Ok((returned, failure, error))
    }

    fn returned<'a>(&mut self, node: &'a HirNode, source: &source::Checked) -> Result<&'a HirNode> {
        self.node(
            node,
            Shape::value("expr_block", source.error.block, Role::Never),
        )?;
        let [statement] = self.children(node)?;
        self.node(
            statement,
            Shape::statement("stmt_semi", self.semicolon(source.error.returned)?),
        )?;

        let [returned] = self.children(statement)?;
        self.node(
            returned,
            Shape::value("expr_ret", source.error.returned, Role::Returned),
        )?;
        Ok(returned)
    }

    fn error<'a>(
        &mut self,
        returned: &'a HirNode,
        source: &source::Checked,
    ) -> Result<(&'a HirNode, &'a HirNode)> {
        let [failure] = self.children(returned)?;
        self.node(
            failure,
            Shape::value("expr_call", source.error.err, Role::Result),
        )?;
        let [error] = self.children(failure)?;
        self.node(
            error,
            Shape::value("expr_call", source.error.constructor, Role::Error),
        )?;

        let [what] = self.children(error)?;
        self.local(what, source.error.what, Role::Text)?;

        Ok((failure, error))
    }

    fn semicolon(&self, returned: ByteSpan) -> Result<ByteSpan> {
        let suffix = self
            .facts
            .source
            .bytes
            .get(returned.end as usize..)
            .ok_or("Missing return suffix")?;
        let gap = suffix.len() - suffix.trim_start().len();
        if suffix.as_bytes().get(gap) != Some(&b';') {
            return Err("Missing return semicolon".into());
        }
        Ok(ByteSpan {
            start: returned.start,
            end: returned.end + gap as u32 + 1,
        })
    }

    fn success(&mut self, node: &HirNode, source: &source::Checked) -> Result<()> {
        self.node(
            node,
            Shape::value("expr_call", source.success.expression, Role::Result),
        )?;
        let [unit] = self.children(node)?;
        self.node(
            unit,
            Shape::value("expr_tup", source.success.unit, Role::Unit),
        )?;

        let _: &[HirNode; 0] = self.children(unit)?;
        if unit.res.is_some() || unit.dispatch.is_some() {
            return Err("Unit success contains an operation".into());
        }
        Ok(())
    }
}

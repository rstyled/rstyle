//! Original loop bodies contribute syntactic sites once, including after transfers.
use super::{branches::condition_kind, *};

struct LoopPlan {
    effects: Effects,
    target: Option<crate::HirRef>,
}

impl LoopPlan {
    fn unavailable(effects: Effects) -> Self {
        Self {
            effects,
            target: None,
        }
    }
}

impl Scan<'_> {
    fn iteration_expression(&mut self, expression: &Expr) -> Result<bool, String> {
        match expression {
            Expr::Loop(value) if value.attrs.is_empty() && value.label.is_none() => {
                self.ordinary_loop(value)?;
            }
            Expr::While(value)
                if value.attrs.is_empty()
                    && value.label.is_none()
                    && plain_condition(&value.cond) =>
            {
                self.while_loop(value)?;
            }
            Expr::ForLoop(value) if value.attrs.is_empty() && value.label.is_none() => {
                self.for_loop(value)?;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    pub(super) fn loop_expression(&mut self, expression: &Expr) -> Result<bool, String> {
        if self.iteration_expression(expression)? {
            return Ok(true);
        }

        match expression {
            Expr::Break(value)
                if value.attrs.is_empty() && value.label.is_none() && value.expr.is_none() =>
            {
                self.apply(self.transfer_evidence(expression, "expr_break")?);
            }
            Expr::Continue(value) if value.attrs.is_empty() && value.label.is_none() => {
                self.apply(self.transfer_evidence(expression, "expr_continue")?);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn ordinary_loop(&mut self, syntax: &syn::ExprLoop) -> Result<(), String> {
        let span = self.span(syntax);
        let children = [("expr_block", self.span(&syntax.body))];
        let plan = match self.loop_node("expr_loop", span, &children)? {
            Some(node) => self.control_plan(node, &node.hir),
            None => LoopPlan::unavailable(Effects::unknown(
                span,
                "Loop lacks original native correspondence",
            )),
        };

        self.scan_loop(plan, &syntax.body, None)
    }

    fn while_loop(&mut self, syntax: &syn::ExprWhile) -> Result<(), String> {
        let condition = transparent(&syntax.cond);
        let span = self.span(syntax);
        let children = [
            (condition_kind(condition), self.span(condition)),
            ("expr_block", self.span(&syntax.body)),
        ];
        let plan = match self.loop_node("expr_while", span, &children)? {
            Some(node) => self.while_plan(syntax, node)?,
            None => LoopPlan::unavailable(Effects::unknown(
                span,
                "While lacks original native correspondence",
            )),
        };

        self.scan_loop(plan, &syntax.body, None)
    }

    fn while_plan(&self, syntax: &syn::ExprWhile, node: &HirNode) -> Result<LoopPlan, String> {
        let mut plan = self.control_plan(node, &node.hir);
        if plan.target.is_none() {
            return Ok(plan);
        }

        let condition = &node.children[0];
        match condition
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.as_deref())
        {
            Some("bool") => {}
            Some(_) => {
                return Err("While condition type contradicts original boolean syntax".into());
            }
            None => {
                return Ok(LoopPlan::unavailable(Effects::unknown(
                    self.span(syntax),
                    "While condition type is unavailable",
                )));
            }
        }
        plan.effects = self.condition_evidence(&syntax.cond, condition)?;
        Ok(plan)
    }

    fn for_loop(&mut self, syntax: &syn::ExprForLoop) -> Result<(), String> {
        let plan = self.for_plan(syntax)?;
        self.scan_loop(plan, &syntax.body, Some(&syntax.pat))
    }

    fn for_plan(&self, syntax: &syn::ExprForLoop) -> Result<LoopPlan, String> {
        let span = self.span(syntax);
        let Some((range, start, end)) = range_syntax(&syntax.expr) else {
            return Ok(LoopPlan::unavailable(Effects::unsupported(
                span,
                "For iterator is outside half-open u32 ranges",
            )));
        };
        let Some(pattern_kind) = binder_kind(&syntax.pat) else {
            return Ok(LoopPlan::unavailable(Effects::unsupported(
                span,
                "For binder requires an immutable u32 binding or wildcard",
            )));
        };
        let children = [
            ("expr_struct", self.span(range)),
            (pattern_kind, self.span(&syntax.pat)),
            ("expr_block", self.span(&syntax.body)),
        ];
        let Some(node) = self.loop_node("expr_for", span, &children)? else {
            return Ok(LoopPlan::unavailable(Effects::unknown(
                span,
                "For loop lacks original native correspondence",
            )));
        };
        let Some(target) = for_target(node)? else {
            return Ok(LoopPlan::unavailable(Effects::unknown(
                span,
                "For loop original destination is unavailable",
            )));
        };

        let mut plan = self.control_plan(node, target);
        if plan.target.is_some() {
            plan.effects = self.range_evidence(&node.children[0], [start, end])?;
            plan.effects
                .boundaries
                .extend(self.loop_type(&node.children[1], &["u32"]).boundaries);
            if !plan.effects.boundaries.is_empty() {
                plan.target = None;
            }
        }
        Ok(plan)
    }

    fn loop_node(
        &self,
        kind: &str,
        span: ByteSpan,
        children: &[(&str, ByteSpan)],
    ) -> Result<Option<&HirNode>, String> {
        let Some(node) = self.index.node(kind, span)? else {
            return Ok(None);
        };
        if node.hir.krate != self.facts.body.hir.krate
            || node.hir.owner != self.facts.body.hir.owner
        {
            return Err("Loop native identity belongs to another original owner".into());
        }
        if node.children.len() != children.len()
            || node
                .children
                .iter()
                .zip(children)
                .any(|(node, (kind, span))| {
                    node.kind != *kind
                        || !crate::source_evidence::corresponding_span(
                            self.source,
                            *span,
                            node.span,
                        )
                })
        {
            return Err("Loop native child roster contradicts original syntax".into());
        }
        Ok(Some(node))
    }

    fn control_plan(&self, node: &HirNode, target: &crate::HirRef) -> LoopPlan {
        let effects = self.loop_type(node, &["unit", "never"]);
        if !effects.boundaries.is_empty() {
            return LoopPlan::unavailable(effects);
        }
        let Some(body) = node.children.last() else {
            return LoopPlan::unavailable(Effects::unknown(
                node.span,
                "Loop body was not captured",
            ));
        };
        let effects = self.loop_type(body, &["unit", "never"]);
        if !effects.boundaries.is_empty() {
            return LoopPlan::unavailable(effects);
        }

        LoopPlan {
            effects,
            target: Some(target.clone()),
        }
    }

    fn scan_loop(
        &mut self,
        plan: LoopPlan,
        body: &syn::Block,
        pattern: Option<&Pat>,
    ) -> Result<(), String> {
        self.apply(plan.effects);
        let Some(target) = plan.target else {
            return Ok(());
        };
        if self.result.execution == Execution::BudgetExhausted {
            return Ok(());
        }
        self.scoped_loop(target, body, pattern)
    }

    fn scoped_loop(
        &mut self,
        target: crate::HirRef,
        body: &syn::Block,
        pattern: Option<&Pat>,
    ) -> Result<(), String> {
        self.enter_loop(target);
        let result = self.loop_statements(body, pattern);
        self.leave_loop();
        result
    }

    fn enter_loop(&mut self, target: crate::HirRef) {
        self.loop_targets.push(target);
        self.scopes.push(BTreeMap::new());
    }

    fn leave_loop(&mut self) {
        self.scopes.pop();
        self.loop_targets.pop();
    }

    fn loop_statements(&mut self, body: &syn::Block, pattern: Option<&Pat>) -> Result<(), String> {
        let scope = self.span(body);
        if let Some(pattern) = pattern
            && !matches!(pattern, Pat::Wild(_))
        {
            self.declare(
                pattern,
                DeclarationContext {
                    scope,
                    formal: None,
                    provenance: Provenance::LocalOwned,
                },
            )?;
        }
        self.statements(&body.stmts, scope)
    }

    fn transfer_evidence(&self, syntax: &Expr, kind: &str) -> Result<Effects, String> {
        let span = self.span(syntax);
        let Some(node) = self.index.node(kind, span)? else {
            return Ok(Effects::unknown(
                span,
                "Loop transfer lacks original native correspondence",
            ));
        };
        if !node.children.is_empty() {
            return Err("Valueless loop transfer retains unexpected native children".into());
        }
        let Some(resolution) = &node.res else {
            return Ok(Effects::unknown(
                span,
                "Loop transfer destination is unavailable",
            ));
        };
        if resolution.target.is_none() {
            return Ok(Effects::unknown(
                span,
                "Loop transfer target was not captured",
            ));
        }
        let Some(target) = self.loop_targets.last() else {
            return Ok(Effects::unsupported(
                span,
                "Loop transfer is outside a checked original loop",
            ));
        };
        if resolution.kind != "innermost_loop" || resolution.target.as_ref() != Some(target) {
            return Err("Loop transfer destination contradicts its original innermost loop".into());
        }

        Ok(self.loop_type(node, &["unit", "never"]))
    }

    fn range_evidence(&self, node: &HirNode, endpoints: [&Expr; 2]) -> Result<Effects, String> {
        if node.children.len() != endpoints.len() {
            return Err("For range endpoint roster contradicts original syntax".into());
        }
        let mut effects = self.range_authority(node)?;
        for (syntax, native) in endpoints.iter().zip(&node.children) {
            let operand = self.condition_evidence(syntax, native)?;
            effects.boundaries.extend(operand.boundaries);
            effects
                .boundaries
                .extend(self.loop_type(native, &["u32"]).boundaries);
        }
        Ok(effects)
    }

    fn range_authority(&self, node: &HirNode) -> Result<Effects, String> {
        if node.expansion.is_some() || node.ty.is_none() {
            return Ok(Effects::unknown(
                node.span,
                "For range original type authority is unavailable",
            ));
        }
        let Some(resolution) = &node.res else {
            return Ok(Effects::unknown(
                node.span,
                "For range core definition is unavailable",
            ));
        };
        if resolution.kind != "resolved_u32_range_type" {
            return Ok(Effects::unknown(
                node.span,
                "For range concrete u32 compiler observation is unavailable",
            ));
        }

        let callee: crate::data_library::Callee = serde_json::from_str(&resolution.description)
            .map_err(|error| format!("For range definition is malformed: {error}"))?;
        let Some(registry) = crate::data_library::registry_for(self.facts) else {
            return Ok(Effects::unknown(
                node.span,
                "For range selected compiler registry is unavailable",
            ));
        };
        if resolution.target.is_some()
            || callee.implementation.is_some()
            || node.ty.as_ref().is_some_and(|ty| ty.normalized.is_some())
            || !matches!(
                callee.path.as_str(),
                "std::ops::Range" | "core::ops::Range" | "core::ops::range::Range"
            )
            || !registry.exact_core_definition(&callee, "std::ops::Range")
        {
            return Err("For range does not name the exact original core Range definition".into());
        }
        Ok(Effects::default())
    }

    pub(super) fn control_type<'n>(&self, node: &'n HirNode) -> Option<&'n str> {
        let hir = &node.hir;
        if node.expansion.is_none()
            && self
                .never_controls
                .contains(&(hir.krate, hir.owner, hir.local))
        {
            Some("never")
        } else {
            node.ty.as_ref().and_then(|ty| ty.normalized.as_deref())
        }
    }

    fn loop_type(&self, node: &HirNode, allowed: &[&str]) -> Effects {
        if node.expansion.is_some() {
            return Effects::unknown(node.span, "Loop input has no original unexpanded evidence");
        }

        match self.control_type(node) {
            Some(ty) if allowed.contains(&ty) => Effects::default(),
            None => Effects::unknown(node.span, "Loop input type authority is unavailable"),
            Some(_) => Effects::unsupported(
                node.span,
                &format!("Loop input requires original {} type", allowed.join(" or ")),
            ),
        }
    }
}

fn for_target(node: &HirNode) -> Result<Option<&crate::HirRef>, String> {
    let Some(resolution) = &node.res else {
        return Ok(None);
    };
    let Some(target) = &resolution.target else {
        return Ok(None);
    };
    if resolution.kind != "innermost_loop"
        || target.krate != node.hir.krate
        || target.owner != node.hir.owner
    {
        return Err("For loop destination contradicts its original owner".into());
    }
    Ok(Some(target))
}

fn binder_kind(pattern: &Pat) -> Option<&'static str> {
    match pattern {
        Pat::Wild(value) if value.attrs.is_empty() => Some("pat_wild"),
        Pat::Ident(value)
            if value.attrs.is_empty()
                && value.mutability.is_none()
                && value.by_ref.is_none()
                && value.subpat.is_none() =>
        {
            Some("pat_binding")
        }
        _ => None,
    }
}

fn range_syntax(expression: &Expr) -> Option<(&syn::ExprRange, &Expr, &Expr)> {
    let Expr::Range(range) = transparent(expression) else {
        return None;
    };
    if !range.attrs.is_empty() || !matches!(range.limits, syn::RangeLimits::HalfOpen(_)) {
        return None;
    }

    let start = range.start.as_deref()?;
    let end = range.end.as_deref()?;
    (plain_condition(start) && plain_condition(end)).then_some((range, start, end))
}

pub(super) fn never_controls(
    facts: &HirFnFacts,
    index: &native::Index<'_>,
) -> Result<BTreeSet<(u32, u32, u32)>, String> {
    use crate::consumer_facts::control_types::{self, Observations};
    use serde::Deserialize;
    let mut identities = BTreeSet::new();
    let Some(value) = facts.build.frontend_options.get(control_types::KEY) else {
        return Ok(identities);
    };
    if value
        .get("never")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|entries| entries.len() > index.node_count())
    {
        return Err("Control type roster exceeds its native inventory".into());
    }
    let observations = Observations::deserialize(value).map_err(|error| error.to_string())?;
    if observations.identity != control_types::IDENTITY {
        return Err("Control type observations have another schema identity".into());
    }

    for control in observations.never {
        let node = index
            .captured_node(&control.kind, control.span)?
            .ok_or("Control type observation has no original native node")?;
        let hir = &node.hir;
        if !control_types::control_kind(&node.kind)
            || control.hir != *hir
            || hir.krate != facts.body.hir.krate
            || hir.owner != facts.body.hir.owner
            || node.ty.as_ref().is_none_or(|ty| ty.normalized.is_some())
            || identities.contains(&(hir.krate, hir.owner, hir.local))
        {
            return Err(
                "Control type observation contradicts original native identity or type".into(),
            );
        }
        identities.insert((hir.krate, hir.owner, hir.local));
    }
    Ok(identities)
}

use super::*;

struct ConditionalPlan {
    wrapper: Effects,
    condition: Option<Effects>,
}

impl ConditionalPlan {
    fn unavailable(wrapper: Effects) -> Self {
        Self {
            wrapper,
            condition: None,
        }
    }
}

impl Scan<'_> {
    pub(super) fn conditional_expression(&mut self, branch: &syn::ExprIf) -> Result<(), String> {
        if self.calls.is_some() {
            self.conditional_calls(branch)
        } else {
            self.direct_conditional(branch)
        }
    }

    fn direct_conditional(&mut self, branch: &syn::ExprIf) -> Result<(), String> {
        let plan = self.conditional_evidence(branch)?;
        self.apply(plan.wrapper);
        let Some(condition) = plan.condition else {
            return Ok(());
        };

        self.apply(condition);
        self.direct_conditional_branches(branch)
    }

    fn direct_conditional_branches(&mut self, branch: &syn::ExprIf) -> Result<(), String> {
        if self.result.execution != Execution::BudgetExhausted {
            self.block(&branch.then_branch)?;
        }
        if self.result.execution != Execution::BudgetExhausted
            && let Some((_, alternative)) = &branch.else_branch
        {
            self.expression(alternative)?;
        }
        Ok(())
    }

    fn conditional_evidence(&self, branch: &syn::ExprIf) -> Result<ConditionalPlan, String> {
        let span = self.span(branch);
        let Some(node) = self.index.node("expr_if", span)? else {
            return Ok(ConditionalPlan::unavailable(Effects::unknown(
                span,
                "Conditional lacks original native correspondence",
            )));
        };
        let children = self.conditional_children(branch);
        if node.children.len() != children.len()
            || node
                .children
                .iter()
                .zip(&children)
                .any(|(node, (kind, span))| {
                    !crate::source_evidence::corresponding_span(self.source, *span, node.span)
                        || node.kind != *kind
                })
        {
            return Err("Conditional native child roster contradicts original syntax".into());
        }

        let wrapper = self.conditional_types(node, span)?;
        if !wrapper.boundaries.is_empty() {
            return Ok(ConditionalPlan::unavailable(wrapper));
        }
        let condition = self.condition_evidence(&branch.cond, &node.children[0])?;
        Ok(ConditionalPlan {
            wrapper,
            condition: Some(condition),
        })
    }

    fn conditional_children(&self, branch: &syn::ExprIf) -> Vec<(&'static str, ByteSpan)> {
        let condition = transparent(&branch.cond);
        let kind = condition_kind(condition);
        let mut children = vec![
            (kind, self.span(condition)),
            ("expr_block", self.span(&branch.then_branch)),
        ];

        if let Some((_, alternative)) = &branch.else_branch {
            let alternative = transparent(alternative);
            let kind = if matches!(alternative, Expr::If(_)) {
                "expr_if"
            } else {
                "expr_block"
            };
            children.push((kind, self.span(alternative)));
        }
        children
    }

    fn condition_evidence(&self, expression: &Expr, node: &HirNode) -> Result<Effects, String> {
        let expression = transparent(expression);
        let span = self.span(expression);
        if node.kind != condition_kind(expression)
            || !crate::source_evidence::corresponding_span(self.source, span, node.span)
        {
            return Err("Conditional native operand identity contradicts original syntax".into());
        }

        if node.expansion.is_some() {
            return Ok(Effects::unknown(
                span,
                "Conditional operand has no original unexpanded evidence",
            ));
        }
        let children: Vec<&Expr> = match expression {
            Expr::Binary(binary) if compound(&binary.op) => {
                return Ok(Effects::unsupported(
                    span,
                    "Compound assignment in conditional operand is outside direct inventory",
                ));
            }
            Expr::Binary(binary) => vec![&binary.left, &binary.right],
            Expr::Unary(unary) => vec![&unary.expr],
            Expr::Path(_) | Expr::Lit(_) => vec![],
            Expr::Call(_) => return Ok(Effects::unknown(span, "Unknown call boundary")),
            _ => {
                return Ok(Effects::unsupported(
                    span,
                    "Unsupported conditional operand",
                ));
            }
        };
        if node.children.len() != children.len() {
            return Err("Conditional native operand roster contradicts original syntax".into());
        }

        let mut effects = if children.is_empty() {
            value_effects(node, span)
        } else {
            primitive_effects(node, span)
        };
        for (syntax, native) in children.iter().zip(&node.children) {
            // Condition syntax contributes only boundaries, never assignments or aliases.
            effects
                .boundaries
                .extend(self.condition_evidence(syntax, native)?.boundaries);
        }
        Ok(effects)
    }

    fn conditional_types(&self, node: &HirNode, span: ByteSpan) -> Result<Effects, String> {
        let Some(condition) = &node.children[0].ty else {
            return Ok(Effects::unknown(
                span,
                "Conditional native condition type was not captured",
            ));
        };
        if let Some(ty) = condition.normalized.as_deref()
            && ty != "bool"
        {
            return Err("Conditional native condition type contradicts ordinary if syntax".into());
        }
        if condition.normalized.as_deref() != Some("bool")
            || std::iter::once(node)
                .chain(node.children.iter().skip(1))
                .any(|node| {
                    node.ty
                        .as_ref()
                        .and_then(|ty| ty.normalized.as_ref())
                        .is_none()
                })
        {
            return Ok(Effects::unknown(
                span,
                "Conditional native type authority is unavailable",
            ));
        }

        Ok(
            if benign_node(node) && node.children.iter().skip(1).all(benign_node) {
                Effects::default()
            } else {
                Effects::unsupported(span, "Unsupported conditional value or branch result")
            },
        )
    }
}

fn condition_kind(expression: &Expr) -> &'static str {
    match expression {
        Expr::Path(_) => "expr_path",
        Expr::Lit(_) => "expr_lit",
        Expr::Binary(binary) if compound(&binary.op) => "expr_assign_op",
        Expr::Binary(_) => "expr_binary",
        Expr::Unary(_) => "expr_unary",
        Expr::Call(_) => "expr_call",
        _ => "unsupported_condition",
    }
}

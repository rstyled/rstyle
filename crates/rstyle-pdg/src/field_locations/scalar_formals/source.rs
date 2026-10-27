//! Source-first formal binding and guarded lexical access inventory.
use super::*;
use crate::{HirNode, source_evidence};
use syn::{spanned::Spanned, visit::Visit};

mod formals;

pub(super) fn verify(facts: &HirFnFacts, evidence: &ScalarFormalAccesses) -> Result<(), Failure> {
    if facts.source.bytes.len() > crate::concrete_types::MAX_SOURCE_BYTES
        || evidence.formals.len() > 64
    {
        return Err(Failure::Unknown(
            "Mutable scalar source/formal inventory exceeds its budget".into(),
        ));
    }
    let function: syn::ItemFn = syn::parse_str(&facts.source.bytes)
        .map_err(|error| Failure::Unsupported(error.to_string()))?;
    formals::verify(facts, evidence, &function)?;
    let mut inventory = Inventory {
        facts,
        evidence,
        nodes: source_evidence::Candidates::new(&facts.body),
        accesses: Vec::new(),
        error: None,
    };
    inventory.visit_block(&function.block);
    if let Some(error) = inventory.error {
        return Err(error);
    }
    if inventory.accesses != evidence.accesses {
        return Err("Mutable scalar source/HIR access order, roots or effects disagree".into());
    }

    Ok(())
}

struct Inventory<'a> {
    facts: &'a HirFnFacts,
    evidence: &'a ScalarFormalAccesses,
    nodes: source_evidence::Candidates<'a>,
    accesses: Vec<Access>,
    error: Option<Failure>,
}

impl Inventory<'_> {
    fn formal(&self, expression: &syn::Expr) -> Option<&Formal> {
        let syn::Expr::Path(path) = crate::data_aggregates::ungroup(expression) else {
            return None;
        };
        if path.qself.is_some() {
            return None;
        }
        self.evidence.formals.iter().find(|formal| {
            formal.mutable_usize()
                && path.path.is_ident(
                    self.facts.signature.params[formal.parameter as usize]
                        .pat
                        .snippet
                        .as_str(),
                )
        })
    }

    fn dereference<'a>(&self, expression: &'a syn::Expr) -> Option<(&'a syn::ExprUnary, u32)> {
        let syn::Expr::Unary(unary) = crate::data_aggregates::ungroup(expression) else {
            return None;
        };
        matches!(unary.op, syn::UnOp::Deref(_))
            .then(|| {
                self.formal(&unary.expr)
                    .map(|formal| (unary, formal.parameter))
            })
            .flatten()
    }

    fn node(&self, kind: &str, span: proc_macro2::Span) -> Result<&HirNode, Failure> {
        let span = source_evidence::original_span(&self.facts.source.bytes, span);
        let Some(node) = self.nodes.unique_node(&self.facts.source.bytes, kind, span) else {
            return Err(Failure::Unknown(
                "Mutable source access lacks one exact resolved HIR site".into(),
            ));
        };
        if self
            .facts
            .source
            .bytes
            .get(node.span.start as usize..node.span.end as usize)
            != Some(node.snippet.as_str())
        {
            return Err("Mutable HIR access snippet differs from its original source".into());
        }

        Ok(node)
    }

    fn load(&self, unary: &syn::ExprUnary, parameter: u32) -> Result<Access, Failure> {
        let node = self.node("expr_unary", unary.span())?;
        let [base] = node.children.as_slice() else {
            return Err(Failure::Unknown(
                "Mutable source dereference lacks one base".into(),
            ));
        };
        let formal = &self.evidence.formals[parameter as usize];
        let span = source_evidence::original_span(&self.facts.source.bytes, unary.expr.span());
        if !source_evidence::corresponding_span(&self.facts.source.bytes, span, base.span)
            || self
                .facts
                .source
                .bytes
                .get(base.span.start as usize..base.span.end as usize)
                != Some(base.snippet.as_str())
        {
            return Err(Failure::Unknown(
                "Mutable base source/HIR correspondence differs".into(),
            ));
        }
        if base
            .res
            .as_ref()
            .and_then(|resolution| resolution.target.as_ref())
            .is_none()
        {
            return Err(Failure::Unknown(
                "Mutable source base compiler resolution is missing".into(),
            ));
        }
        if base.kind != "expr_path"
            || !base.res.as_ref().is_some_and(|resolution| {
                resolution.kind == "local" && resolution.target.as_ref() == Some(&formal.binding)
            })
        {
            return Err("Mutable source dereference resolves to another formal or local".into());
        }

        Ok(Access {
            hir: node.hir.clone(),
            span: node.span,
            snippet: node.snippet.clone(),
            parameter,
            kind: AccessKind::Load,
            compound: None,
        })
    }

    fn compound(&mut self, binary: &syn::ExprBinary, parameter: u32) -> Result<(), Failure> {
        let operator =
            operator(&binary.op).ok_or("Unsupported mutable source assignment operator")?;
        let (unary, _) = self
            .dereference(&binary.left)
            .ok_or("Mutable source compound lacks direct target")?;
        self.visit_expr(&binary.right);
        let mut load = self.load(unary, parameter)?;
        let node = self.node("expr_assign_op", binary.span())?;
        let [_, rhs] = node.children.as_slice() else {
            return Err(Failure::Unknown(
                "Mutable source compound lacks exact operands".into(),
            ));
        };
        let span = source_evidence::original_span(&self.facts.source.bytes, binary.right.span());
        if !source_evidence::corresponding_span(&self.facts.source.bytes, span, rhs.span)
            || self
                .facts
                .source
                .bytes
                .get(rhs.span.start as usize..rhs.span.end as usize)
                != Some(rhs.snippet.as_str())
        {
            return Err("Mutable source compound changes its evaluated RHS".into());
        }
        load.compound = Some(Compound {
            hir: node.hir.clone(),
            operator: operator.into(),
            rhs: rhs.hir.clone(),
        });
        let store = Access {
            kind: AccessKind::Store,
            ..load.clone()
        };
        if self.accesses.len() > crate::field_locations::MAX_OPERATIONS - 2 {
            return Err(Failure::Unknown(
                "Mutable source access inventory exceeds its budget".into(),
            ));
        }
        self.accesses.extend([load, store]);

        Ok(())
    }

    fn expression(&mut self, expression: &syn::Expr) -> Result<bool, Failure> {
        if let syn::Expr::Binary(binary) = expression
            && operator(&binary.op).is_some()
            && let Some((_, parameter)) = self.dereference(&binary.left)
        {
            self.compound(binary, parameter)?;
            return Ok(true);
        }
        if let syn::Expr::Unary(unary) = expression
            && let Some((_, parameter)) = self.dereference(expression)
        {
            let access = self.load(unary, parameter)?;
            if self.accesses.len() >= crate::field_locations::MAX_OPERATIONS {
                return Err(Failure::Unknown(
                    "Mutable source access inventory exceeds its budget".into(),
                ));
            }
            self.accesses.push(access);
            return Ok(true);
        }
        if self.formal(expression).is_some() {
            return Err("Mutable source formal escapes direct scalar access".into());
        }
        if matches!(
            expression,
            syn::Expr::Assign(_)
                | syn::Expr::Reference(_)
                | syn::Expr::Unsafe(_)
                | syn::Expr::Closure(_)
                | syn::Expr::Async(_)
        ) {
            return Err(
                "Mutable scalar source store/reborrow/unsafe/callable form remains unsupported"
                    .into(),
            );
        }

        Ok(false)
    }
}

impl<'ast> Visit<'ast> for Inventory<'_> {
    fn visit_expr(&mut self, expression: &'ast syn::Expr) {
        if self.error.is_some() {
            return;
        }
        match self.expression(expression) {
            Ok(true) => {}
            Ok(false) => syn::visit::visit_expr(self, expression),
            Err(reason) => self.error = Some(reason),
        }
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        if self.evidence.formals.iter().any(|formal| {
            formal.mutable_usize()
                && pattern.ident
                    == self.facts.signature.params[formal.parameter as usize]
                        .pat
                        .snippet
                        .as_str()
        }) {
            self.error = Some("Mutable source formal is shadowed or rebound".into());
        }
        syn::visit::visit_pat_ident(self, pattern);
    }

    fn visit_macro(&mut self, _: &'ast syn::Macro) {
        self.error = Some("Mutable scalar source inventory cannot omit macro effects".into());
    }

    fn visit_item(&mut self, _: &'ast syn::Item) {
        self.error = Some("Mutable scalar source inventory cannot omit nested items".into());
    }
}

fn operator(operator: &syn::BinOp) -> Option<&'static str> {
    Some(match operator {
        syn::BinOp::AddAssign(_) => "+=",
        syn::BinOp::SubAssign(_) => "-=",
        syn::BinOp::MulAssign(_) => "*=",
        syn::BinOp::DivAssign(_) => "/=",
        syn::BinOp::RemAssign(_) => "%=",
        syn::BinOp::BitAndAssign(_) => "&=",
        syn::BinOp::BitOrAssign(_) => "|=",
        syn::BinOp::BitXorAssign(_) => "^=",
        syn::BinOp::ShlAssign(_) => "<<=",
        syn::BinOp::ShrAssign(_) => ">>=",
        _ => return None,
    })
}

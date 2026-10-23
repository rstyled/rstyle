//! HIR-side observational walk, separate from the source-side inventory.
use super::*;
use crate::HirNode;

pub(super) fn observe(facts: &HirFnFacts, formals: &[Formal]) -> Result<Vec<Access>, Failure> {
    if formals.len() > 64 {
        return Err(Failure::Unknown(
            "Mutable scalar formal inventory exceeds its parameter budget".into(),
        ));
    }
    check_node_identities(&facts.body, &mut std::collections::BTreeSet::new())?;
    if formals.iter().any(|formal| {
        matches!(formal.observation, FormalObservation::Unknown { .. })
            || matches!(
                formal.observation,
                FormalObservation::Mutable {
                    referent: ScalarObservation::Unknown { .. }
                } | FormalObservation::Shared {
                    referent: ScalarObservation::Unknown { .. }
                }
            )
    }) {
        return Err(Failure::Unknown(
            "Mutable scalar formal compiler type is missing".into(),
        ));
    }
    if formals.iter().any(|formal| {
        matches!(formal.observation, FormalObservation::Mutable { .. }) && !formal.mutable_usize()
    }) {
        return Err("Mutable referent is outside the observed usize location family".into());
    }
    let mut inventory = Inventory {
        formals,
        accesses: Vec::new(),
    };
    inventory.visit(&facts.body)?;

    Ok(inventory.accesses)
}

fn check_node_identities(
    node: &HirNode,
    identities: &mut std::collections::BTreeSet<(u32, u32, u32)>,
) -> Result<(), Failure> {
    if identities.len() >= super::super::MAX_OPERATIONS
        || !identities.insert((node.hir.krate, node.hir.owner, node.hir.local))
    {
        return Err(Failure::Unknown(
            "Mutable scalar HIR inventory has repeated identities or exceeds its node budget"
                .into(),
        ));
    }
    for child in &node.children {
        check_node_identities(child, identities)?;
    }

    Ok(())
}

struct Inventory<'a> {
    formals: &'a [Formal],
    accesses: Vec<Access>,
}

impl Inventory<'_> {
    fn visit(&mut self, node: &HirNode) -> Result<(), Failure> {
        if node.kind == "expr_assign_op" && self.target(node).is_some() {
            return self.compound(node);
        }
        if node.kind == "expr_assign" && self.target(node).is_some() {
            return Err("Plain mutable scalar stores remain unsupported observations".into());
        }
        if node.kind == "expr_unary" && self.referent(node).is_some() {
            let parameter = self.checked_deref(node)?;
            return self.push(access(node, parameter, AccessKind::Load, None));
        }
        if self.formal_path(node).is_some() {
            return Err("Mutable scalar reference escapes a direct observed dereference".into());
        }
        if node.kind == "expr_addr_of" && contains_formal(node, self.formals) {
            return Err("Mutable scalar reborrows remain unsupported observations".into());
        }
        for child in &node.children {
            self.visit(child)?;
        }

        Ok(())
    }

    fn formal_path(&self, node: &HirNode) -> Option<&Formal> {
        if node.kind != "expr_path" || node.res.as_ref()?.kind != "local" {
            return None;
        }
        let target = node.res.as_ref()?.target.as_ref()?;
        self.formals
            .iter()
            .find(|formal| formal.mutable_usize() && formal.binding == *target)
    }

    fn referent(&self, node: &HirNode) -> Option<&Formal> {
        let [base] = node.children.as_slice() else {
            return None;
        };
        self.formal_path(base)
    }

    fn target(&self, node: &HirNode) -> Option<&Formal> {
        let target = node.children.first()?;
        (target.kind == "expr_unary")
            .then(|| self.referent(target))
            .flatten()
    }

    fn checked_deref(&self, node: &HirNode) -> Result<u32, Failure> {
        let formal = self.referent(node).ok_or_else(|| {
            Failure::Unknown("Mutable dereference lacks its exact formal root".into())
        })?;
        let dispatch = node
            .dispatch
            .as_ref()
            .ok_or_else(|| Failure::Unknown("Mutable dereference lacks builtin dispatch".into()))?;
        let base = &node.children[0];
        let node_type = type_pair(node)?;
        let base_type = type_pair(base)?;
        if node.kind != "expr_unary"
            || dispatch.kind != "builtin"
            || dispatch.op != "*"
            || dispatch.operand_tys != ["&mut usize"]
            || dispatch.result_ty != "usize"
            || node_type != ("usize", Some("usize"))
            || node.expansion.is_some()
            || base_type != ("&mut usize", None)
            || base.expansion.is_some()
            || !base.children.is_empty()
        {
            return Err("Mutable dereference differs from builtin usize referent evidence".into());
        }

        Ok(formal.parameter)
    }

    fn compound(&mut self, node: &HirNode) -> Result<(), Failure> {
        let [target, rhs] = node.children.as_slice() else {
            return Err(Failure::Unknown(
                "Mutable compound assignment lacks its exact target/RHS".into(),
            ));
        };
        let parameter = self.checked_deref(target)?;
        let dispatch = node
            .dispatch
            .as_ref()
            .ok_or_else(|| Failure::Unknown("Mutable compound lacks builtin dispatch".into()))?;
        let node_type = type_pair(node)?;
        let rhs_type = type_pair(rhs)?;
        if dispatch.kind != "builtin"
            || compound_operator(&dispatch.op).is_none()
            || dispatch.operand_tys != ["usize", "usize"]
            || dispatch.result_ty != "unit"
            || node_type != ("()", Some("unit"))
            || rhs_type != ("usize", Some("usize"))
            || node.expansion.is_some()
        {
            return Err("Mutable compound lacks builtin usize operand/result evidence".into());
        }

        let compound = Compound {
            hir: node.hir.clone(),
            operator: compound_operator(&dispatch.op).unwrap().into(),
            rhs: rhs.hir.clone(),
        };
        // Primitive builtin compound assignment evaluates RHS before place/load.
        self.visit(rhs)?;
        self.push(access(
            target,
            parameter,
            AccessKind::Load,
            Some(compound.clone()),
        ))?;
        self.push(access(target, parameter, AccessKind::Store, Some(compound)))
    }

    fn push(&mut self, access: Access) -> Result<(), Failure> {
        if self.accesses.len() >= super::super::MAX_OPERATIONS {
            return Err(Failure::Unknown(
                "Mutable scalar access inventory exceeds its operation budget".into(),
            ));
        }
        self.accesses.push(access);

        Ok(())
    }
}

fn contains_formal(node: &HirNode, formals: &[Formal]) -> bool {
    node.res
        .as_ref()
        .and_then(|resolution| resolution.target.as_ref())
        .is_some_and(|target| {
            formals
                .iter()
                .any(|formal| formal.mutable_usize() && formal.binding == *target)
        })
        || node
            .children
            .iter()
            .any(|child| contains_formal(child, formals))
}

pub(super) fn access(
    node: &HirNode,
    parameter: u32,
    kind: AccessKind,
    compound: Option<Compound>,
) -> Access {
    Access {
        hir: node.hir.clone(),
        span: node.span,
        snippet: node.snippet.clone(),
        parameter,
        kind,
        compound,
    }
}

fn type_pair(node: &HirNode) -> Result<(&str, Option<&str>), Failure> {
    let ty = node
        .ty
        .as_ref()
        .ok_or_else(|| Failure::Unknown("Mutable scalar access compiler type is missing".into()))?;

    Ok((&ty.observed, ty.normalized.as_deref()))
}

pub(super) fn compound_operator(operator: &str) -> Option<&'static str> {
    Some(match operator {
        "+" | "+=" => "+=",
        "-" | "-=" => "-=",
        "*" | "*=" => "*=",
        "/" | "/=" => "/=",
        "%" | "%=" => "%=",
        "&" | "&=" => "&=",
        "|" | "|=" => "|=",
        "^" | "^=" => "^=",
        "<<" | "<<=" => "<<=",
        ">>" | ">>=" => ">>=",
        _ => return None,
    })
}

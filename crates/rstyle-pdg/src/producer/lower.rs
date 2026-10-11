//! Operation lowering preserves source evaluation order and snapshot values.

use super::graph::*;
use crate::{NodeCategory, frontend::*};

#[derive(Clone)]
struct End {
    node: String,
    kind: String,
    term: Vec<Atom>,
}

struct Flow {
    ends: Vec<End>,
    value: Option<String>,
    normal: bool,
}

struct Condition {
    flow: Flow,
    value: Option<String>,
}

struct Operands {
    flow: Flow,
    values: Vec<String>,
}

struct Arms {
    yes: Flow,
    no: Flow,
}

struct Builder<'a> {
    facts: &'a R0FnFacts,
    nodes: Vec<Operation>,
    transfers: Vec<Transfer>,
    sites: Vec<ValueSite>,
}

pub fn lower(facts: &R0FnFacts) -> Result<Graph, R0Eligibility> {
    let eligibility = crate::classify(facts);
    if eligibility.status != EligibilityVerdict::Eligible {
        return Err(eligibility);
    }
    if let Some(node) = incomplete_shape(&facts.body) {
        return Err(R0Eligibility {
            status: EligibilityVerdict::Unknown,
            diagnostics: vec![R0Diagnostic {
                capability: "rust.scalar-computation".to_string(),
                construct: node.kind.clone(),
                span: node.span,
                snippet: node.snippet.clone(),
                severity: "error".to_string(),
                reason: "Required resolved frontend child correspondence is incomplete; recapture compiler evidence before extraction.".to_string(),
            }],
        });
    }
    let mut builder = Builder {
        facts,
        nodes: Vec::new(),
        transfers: Vec::new(),
        sites: Vec::new(),
    };
    builder.inputs();
    let entry = Flow {
        ends: vec![End {
            node: "E".to_string(),
            kind: "normal".to_string(),
            term: Vec::new(),
        }],
        value: None,
        normal: true,
    };
    let flow = builder.expression(&facts.body, entry);
    if !flow.ends.is_empty() {
        builder.implicit_return(flow);
    }
    let mut transfers = builder.transfers;
    transfers.sort();
    transfers.dedup();

    Ok(Graph {
        nodes: builder.nodes,
        transfers,
        value_sites: builder.sites,
    })
}

fn incomplete_shape(node: &R0Node) -> Option<&R0Node> {
    let count = node.children.len();
    let complete = match node.kind.as_str() {
        "stmt_expr" | "stmt_semi" | "expr_unary" => count == 1,
        "expr_binary" | "expr_assign" | "expr_assign_op" => count == 2,
        "expr_if" => (2..=3).contains(&count),
        "expr_ret" => count <= 1,
        "expr_path" | "expr_lit" | "expr_tup" | "pat_binding" | "ty_annotation" => count == 0,
        _ => true,
    };
    if !complete {
        return Some(node);
    }

    node.children.iter().find_map(incomplete_shape)
}

impl Builder<'_> {
    fn inputs(&mut self) {
        for param in &self.facts.signature.params {
            let operation = Operation {
                id: format!("n{}", self.nodes.len()),
                ty: param
                    .ty
                    .normalized
                    .clone()
                    .expect("eligible parameter type"),
                symbol: Some(param.symbol.clone()),
                definition: Some(format!("d{}", self.nodes.len())),
                position: Some(u64::from(param.position)),
                ..template(&param.pat, NodeCategory::FormalInput)
            };
            self.nodes.push(operation);
        }
    }

    fn emit(&mut self, mut operation: Operation, flow: Flow) -> Flow {
        operation.id = format!("n{}", self.nodes.len());
        let id = operation.id.clone();
        if operation.symbol.is_some()
            && matches!(
                operation.category,
                NodeCategory::Declaration | NodeCategory::Assignment
            )
        {
            operation.definition = Some(format!("d{}", self.nodes.len()));
        }
        self.connect(&flow.ends, &id);
        self.nodes.push(operation);
        let terms = if flow.ends.is_empty() {
            vec![Vec::new()]
        } else {
            flow.ends.iter().map(|e| e.term.clone()).collect()
        };
        let ends = terms
            .into_iter()
            .map(|term| End {
                node: id.clone(),
                kind: "normal".to_string(),
                term,
            })
            .collect();

        Flow {
            ends,
            value: Some(id),
            normal: flow.normal,
        }
    }

    fn connect(&mut self, ends: &[End], id: &str) {
        for end in ends {
            self.transfers.push(Transfer {
                source: end.node.clone(),
                target: id.to_string(),
                kind: end.kind.clone(),
            });
        }
        for site in &mut self.sites {
            let frontier = site
                .alternatives
                .iter()
                .flat_map(|a| &a.completion_cfg_node_ids);
            if site.cfg_successor_node_id.is_none()
                && frontier
                    .into_iter()
                    .any(|node| ends.iter().any(|e| &e.node == node))
            {
                site.cfg_successor_node_id = Some(id.to_string());
            }
        }
    }

    fn expression(&mut self, node: &R0Node, flow: Flow) -> Flow {
        lowerer(node)(self, node, flow)
    }

    fn expression_statement(&mut self, node: &R0Node, flow: Flow) -> Flow {
        self.expression(&node.children[0], flow)
    }

    fn block(&mut self, node: &R0Node, mut flow: Flow) -> Flow {
        for child in &node.children {
            let continuing = flow.normal;
            flow = self.expression(
                child,
                Flow {
                    normal: true,
                    ..flow
                },
            );
            flow.normal &= continuing;
        }
        let tail = node
            .children
            .last()
            .is_some_and(|child| child.kind.starts_with("expr_"));
        if !tail {
            flow.value = None;
        }

        flow
    }

    fn literal(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let mut operation = template(node, NodeCategory::Computation);
        operation.literal = Some(node.lit.clone().unwrap_or_else(|| "()".to_string()));

        self.emit(operation, flow)
    }

    fn read(&mut self, node: &R0Node, flow: Flow) -> Flow {
        self.emit(self.read_operation(node), flow)
    }

    fn read_operation(&self, node: &R0Node) -> Operation {
        let target = node
            .res
            .as_ref()
            .and_then(|r| r.target.as_ref())
            .expect("eligible local resolution");
        Operation {
            symbol: self
                .facts
                .bindings
                .iter()
                .find(|b| &b.hir == target)
                .map(|b| b.symbol.clone()),
            ..template(node, NodeCategory::Read)
        }
    }

    fn declaration(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let initializer = node
            .children
            .iter()
            .find(|n| n.kind.starts_with("expr_"))
            .expect("eligible initializer");
        let flow = self.expression(initializer, flow);
        let pat = node
            .children
            .iter()
            .find(|n| n.kind == "pat_binding")
            .expect("eligible binding");
        let operation = Operation {
            ty: "unit".to_string(),
            symbol: pat.binding.clone(),
            write_form: Some("initialization".to_string()),
            operands: self.operand(&flow.value),
            ..template(node, NodeCategory::Declaration)
        };

        self.emit(operation, flow)
    }

    fn computation(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let evaluated = node.children.iter().fold(
            Operands {
                flow,
                values: Vec::new(),
            },
            |operands, child| {
                let flow = self.expression(child, operands.flow);
                let values = operands
                    .values
                    .into_iter()
                    .chain(self.operand(&flow.value))
                    .collect();
                Operands { flow, values }
            },
        );
        let operation = Operation {
            operator: node.dispatch.as_ref().map(|d| d.op.clone()),
            operands: evaluated.values,
            ..template(node, NodeCategory::Computation)
        };

        self.emit(operation, evaluated.flow)
    }

    fn assignment(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let flow = self.expression(&node.children[1], flow);
        let mut operation = self.write(node, "reassignment");
        operation.operator = Some("=".to_string());
        operation.operands = self.operand(&flow.value);

        self.emit(operation, flow)
    }

    fn write(&self, node: &R0Node, form: &str) -> Operation {
        let mut operation = template(node, NodeCategory::Assignment);
        let target = node.children[0]
            .res
            .as_ref()
            .and_then(|r| r.target.as_ref())
            .expect("eligible assignment target");
        operation.ty = "unit".to_string();
        operation.symbol = self
            .facts
            .bindings
            .iter()
            .find(|b| &b.hir == target)
            .map(|b| b.symbol.clone());
        operation.write_form = Some(form.to_string());

        operation
    }

    fn compound(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let evaluated = self.compound_values(node, flow);
        let dispatch = node.dispatch.as_ref().expect("builtin compound dispatch");
        let operation = Operation {
            ty: dispatch.operand_tys[0].clone(),
            operator: Some(dispatch.op.trim_end_matches('=').to_string()),
            role: "compound_computation".to_string(),
            operands: evaluated.values,
            ..template(node, NodeCategory::Computation)
        };
        let computed = self.emit(operation, evaluated.flow);
        let write = Operation {
            role: "compound_write".to_string(),
            operator: Some(dispatch.op.clone()),
            operands: self.operand(&computed.value),
            ..self.write(node, "compound_assignment")
        };

        self.emit(write, computed)
    }

    fn compound_values(&mut self, node: &R0Node, flow: Flow) -> Operands {
        let rhs = self.expression(&node.children[1], flow);
        let rhs_value = rhs.value.clone();
        let read = Operation {
            role: "compound_target_read".to_string(),
            construct_span: node.span,
            ..self.read_operation(&node.children[0])
        };
        let target = self.emit(read, rhs);
        let values = self
            .operand(&target.value)
            .into_iter()
            .chain(self.operand(&rhs_value))
            .collect();

        Operands {
            flow: target,
            values,
        }
    }

    fn operand(&self, value: &Option<String>) -> Vec<String> {
        value
            .iter()
            .filter(|id| self.nodes.iter().any(|n| &n.id == *id && n.ty != "unit"))
            .cloned()
            .collect()
    }

    fn predicate(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let (condition, construct) = if node.kind == "expr_if" {
            (&node.children[0], "if")
        } else {
            (
                node,
                node.dispatch.as_ref().expect("short circuit").op.as_str(),
            )
        };
        let predicate = Operation {
            construct_span: node.span,
            role: "predicate".to_string(),
            ty: "bool".to_string(),
            construct: Some(construct.to_string()),
            operands: self.operand(&flow.value),
            ..template(condition, NodeCategory::Control)
        };

        self.emit(predicate, flow)
    }

    fn branch(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let predicate = self.condition(node, flow).flow;
        let controller = predicate.value.clone().expect("predicate");
        let arms = self.branch_arms(node, &predicate);

        self.join(node, controller, arms)
    }

    fn branch_arms(&mut self, node: &R0Node, predicate: &Flow) -> Arms {
        let yes = self.expression(&node.children[1], arm(predicate, "true"));
        let no = if let Some(no) = node.children.get(2) {
            self.expression(no, arm(predicate, "false"))
        } else {
            arm(predicate, "false")
        };

        Arms { yes, no }
    }

    fn condition(&mut self, node: &R0Node, flow: Flow) -> Condition {
        let condition = self.expression(&node.children[0], flow);
        let value = condition.value.clone();
        let flow = self.predicate(node, condition);

        Condition { flow, value }
    }

    fn short_circuit(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let condition = self.condition(node, flow);
        let saved = condition.value;
        let op = node
            .dispatch
            .as_ref()
            .expect("short circuit dispatch")
            .op
            .as_str();
        let predicate = condition.flow;
        let controller = predicate.value.clone().expect("predicate");
        let selected = if op == "&&" { "true" } else { "false" };
        let rhs = self.expression(&node.children[1], arm(&predicate, selected));
        let shortcut = Flow {
            value: saved,
            ..arm(
                &predicate,
                if selected == "true" { "false" } else { "true" },
            )
        };
        let (yes, no) = if selected == "true" {
            (rhs, shortcut)
        } else {
            (shortcut, rhs)
        };

        self.join(node, controller, Arms { yes, no })
    }

    fn join(&mut self, node: &R0Node, controller: String, arms: Arms) -> Flow {
        let Arms { yes, no } = arms;
        let alternatives: Vec<_> = [("true", &yes), ("false", &no)]
            .into_iter()
            .filter(|(_, f)| f.normal && !f.ends.is_empty())
            .filter_map(|(outcome, f)| self.alternative(outcome, f))
            .collect();
        let value = if alternatives.len() > 1 {
            Some(self.merge(node, controller, alternatives))
        } else {
            alternatives.first().map(|a| a.value_pdg_node_id.clone())
        };
        let normal = yes.normal || no.normal;
        let ends = [yes, no]
            .into_iter()
            .filter(|f| f.normal)
            .flat_map(|f| f.ends)
            .collect();

        Flow {
            ends,
            value,
            normal,
        }
    }

    fn alternative(&self, outcome: &str, flow: &Flow) -> Option<Alternative> {
        let value = self.operand(&flow.value).into_iter().next()?;
        let frontier: std::collections::BTreeSet<_> =
            flow.ends.iter().map(|e| e.node.clone()).collect();

        Some(Alternative {
            value_pdg_node_id: value,
            outcome: outcome.to_string(),
            completion_cfg_node_ids: frontier.into_iter().collect(),
            guards: normalize(flow.ends.iter().map(|e| e.term.clone()).collect()),
        })
    }

    fn merge(
        &mut self,
        node: &R0Node,
        controller: String,
        alternatives: Vec<Alternative>,
    ) -> String {
        let id = format!("n{}", self.nodes.len());
        let guards = normalize(alternatives.iter().flat_map(|a| a.guards.clone()).collect());
        let mut merge = template(node, NodeCategory::Merge);
        merge.id = id.clone();
        merge.original_kind = "compat.rust.hir-r0.expression_merge".to_string();
        self.nodes.push(merge);
        self.sites.push(ValueSite {
            pdg_node_id: id.clone(),
            construct_span: node.span,
            controller_pdg_node_id: controller,
            cfg_successor_node_id: None,
            guards,
            alternatives,
        });

        id
    }

    fn explicit_return(&mut self, node: &R0Node, flow: Flow) -> Flow {
        let flow = if let Some(value) = node.children.first() {
            self.expression(value, flow)
        } else {
            flow
        };

        self.return_operation(node, "explicit", flow)
    }

    fn implicit_return(&mut self, flow: Flow) {
        let body = &self.facts.body;
        let tail = body
            .children
            .last()
            .filter(|node| node.kind.starts_with("expr_"));
        let anchor = tail.cloned().unwrap_or_else(|| R0Node {
            span: ByteSpan {
                start: body.span.end - 1,
                end: body.span.end,
            },
            snippet: "}".to_string(),
            ..body.clone()
        });

        self.return_operation(&anchor, "implicit", flow);
    }

    fn return_operation(&mut self, node: &R0Node, form: &str, flow: Flow) -> Flow {
        let mut operation = template(node, NodeCategory::Return);
        operation.ty = self
            .facts
            .signature
            .return_ty
            .normalized
            .clone()
            .expect("eligible result");
        operation.return_form = Some(form.to_string());
        operation.operands = if operation.ty == "unit" {
            Vec::new()
        } else {
            self.operand(&flow.value)
        };
        let returned = self.emit(operation, flow);
        let id = returned.value.expect("return operation");
        self.transfers.push(Transfer {
            source: id,
            target: "X".to_string(),
            kind: "return".to_string(),
        });

        Flow {
            ends: Vec::new(),
            value: None,
            normal: false,
        }
    }
}

fn arm(predicate: &Flow, outcome: &str) -> Flow {
    let controller = predicate.value.clone().expect("predicate value");
    let ends = predicate
        .ends
        .iter()
        .map(|end| {
            let term = end
                .term
                .iter()
                .cloned()
                .chain(std::iter::once(Atom {
                    controller_pdg_node_id: controller.clone(),
                    outcome: outcome.to_string(),
                }))
                .collect();
            End {
                node: end.node.clone(),
                kind: outcome.to_string(),
                term,
            }
        })
        .collect();

    Flow {
        ends,
        value: None,
        normal: predicate.normal,
    }
}

fn lowerer<'a>(node: &R0Node) -> fn(&mut Builder<'a>, &R0Node, Flow) -> Flow {
    match node.kind.as_str() {
        "expr_block" => Builder::block,
        "stmt_expr" | "stmt_semi" => Builder::expression_statement,
        "stmt_let" => Builder::declaration,
        "expr_path" => Builder::read,
        "expr_lit" | "expr_tup" => Builder::literal,
        "expr_binary"
            if node
                .dispatch
                .as_ref()
                .is_some_and(|d| d.op == "&&" || d.op == "||") =>
        {
            Builder::short_circuit
        }
        "expr_binary" | "expr_unary" => Builder::computation,
        "expr_assign" => Builder::assignment,
        "expr_assign_op" => Builder::compound,
        "expr_if" => Builder::branch,
        "expr_ret" => Builder::explicit_return,
        _ => unreachable!("eligibility rejects unsupported constructs"),
    }
}

fn template(node: &R0Node, category: NodeCategory) -> Operation {
    Operation {
        id: String::new(),
        category,
        span: node.span,
        construct_span: node.span,
        original_kind: node.kind.clone(),
        role: "source_operation".to_string(),
        ty: node
            .ty
            .as_ref()
            .and_then(|ty| ty.normalized.clone())
            .unwrap_or_else(|| "unit".to_string()),
        symbol: None,
        definition: None,
        operator: None,
        literal: None,
        construct: None,
        write_form: None,
        return_form: None,
        position: None,
        operands: Vec::new(),
    }
}

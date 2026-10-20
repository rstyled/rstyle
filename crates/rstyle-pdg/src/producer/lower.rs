//! Operation lowering preserves source evaluation order and snapshot values.

mod loops;

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
    loops: Vec<loops::LoopFrame>,
    facts: &'a dyn crate::unit_store::EvidenceFacts,
    nodes: Vec<Operation>,
    transfers: Vec<Transfer>,
    sites: Vec<ValueSite>,
}

pub fn lower(facts: &dyn crate::unit_store::EvidenceFacts) -> Result<Graph, HirEligibility> {
    let eligibility = crate::classify(facts);
    if eligibility.status != EligibilityVerdict::Eligible {
        return Err(eligibility);
    }
    if let Some(node) = incomplete_shape(&facts.body) {
        return Err(HirEligibility {
            status: EligibilityVerdict::Unknown,
            diagnostics: vec![HirDiagnostic {
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
        loops: Vec::new(),
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

fn incomplete_shape(node: &HirNode) -> Option<&HirNode> {
    let count = node.children.len();
    let complete = match node.kind.as_str() {
        "stmt_expr" | "stmt_semi" | "expr_unary" | "expr_cast" => count == 1,
        "expr_binary" | "expr_assign" | "expr_assign_op" => count == 2,
        "expr_if" => (2..=3).contains(&count),
        "expr_ret" => count <= 1,
        "expr_path" | "expr_lit" | "pat_binding" | "ty_annotation" => count == 0,
        "expr_tup" | "expr_array" | "expr_repeat" | "expr_struct" => true,
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
                loop_role: if end.kind == "break"
                    || (end.kind == "false"
                        && self.nodes.iter().any(|node| {
                            node.id == end.node && node.construct.as_deref() == Some("while")
                        })) {
                    Some("loop_exit".into())
                } else {
                    None
                },
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

    fn expression(&mut self, node: &HirNode, flow: Flow) -> Flow {
        lowerer(node)(self, node, flow)
    }

    fn expression_statement(&mut self, node: &HirNode, flow: Flow) -> Flow {
        self.expression(&node.children[0], flow)
    }

    fn block(&mut self, node: &HirNode, mut flow: Flow) -> Flow {
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

    fn literal(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let mut operation = template(node, NodeCategory::Computation);
        operation.literal = Some(node.lit.clone().unwrap_or_else(|| "()".to_string()));

        self.emit(operation, flow)
    }

    fn constant(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let constant = crate::scalar_constants::ScalarConstant::decode(
            node.res.as_ref().expect("constant resolution"),
        )
        .expect("verified scalar constant");
        let operation = Operation {
            literal: Some(constant.value),
            construct: Some("constant.value".into()),
            ..template(node, NodeCategory::Computation)
        };

        self.emit(operation, flow)
    }

    fn assertion(&mut self, node: &HirNode, flow: Flow) -> Flow {
        if node.lit.as_deref() == Some("false") {
            return Flow {
                value: None,
                ..flow
            };
        }
        let flow = self.expression(&node.children[0], flow);
        let operation = Operation {
            operator: Some("assert".into()),
            construct: Some(
                node.res
                    .as_ref()
                    .expect("verified assertion")
                    .description
                    .clone(),
            ),
            operands: self.operand(&flow.value),
            ..template(node, NodeCategory::Computation)
        };

        self.emit(operation, flow)
    }

    fn read(&mut self, node: &HirNode, flow: Flow) -> Flow {
        self.emit(self.read_operation(node), flow)
    }

    fn field_read(&mut self, node: &HirNode, flow: Flow) -> Flow {
        if crate::data_value_fields::candidate(node, self.facts) {
            return self.value_field(node, flow);
        }
        if crate::field_locations::derived::candidate(node, self.facts) {
            return self.indirect_field(node, flow);
        }
        self.captured_field(node, flow)
    }

    fn captured_field(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let flow = self.field_base(node, flow);
        let fields = self
            .facts
            .build
            .frontend_options
            .data::<crate::field_locations::FieldObservations>()
            .expect("verified data locations");
        let field = fields
            .operations
            .iter()
            .find(|field| field.hir == node.hir)
            .expect("verified field operation");
        let crate::field_locations::Location::Captured { parameter, .. } = field.location else {
            unreachable!("verified data origin")
        };

        let operation = Operation {
            symbol: Some(
                self.facts.signature.params[parameter as usize]
                    .symbol
                    .clone(),
            ),
            operands: self.operand(&flow.value),
            construct: Some("data.field".into()),
            ..template(node, NodeCategory::Read)
        };
        self.emit(operation, flow)
    }

    fn indirect_field(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let flow = self.field_base(node, flow);
        let operation = Operation {
            operands: self.operand(&flow.value),
            construct: Some("data.indirect-field".into()),
            ..template(node, NodeCategory::Read)
        };
        self.emit(operation, flow)
    }

    fn value_field(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let flow = self.expression(&node.children[0], flow);
        let operation = Operation {
            construct: Some("data.value-field".into()),
            literal: Some(
                crate::data_value_fields::index(node, self.facts)
                    .expect("verified aggregate component")
                    .to_string(),
            ),
            operands: self.operand(&flow.value),
            ..template(node, NodeCategory::Computation)
        };

        self.emit(operation, flow)
    }

    fn borrow(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let flow = self.field_place(&node.children[0], flow);
        self.address_operation(node, flow, "data.borrow")
    }

    fn field_place(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let flow = self.field_base(node, flow);
        let construct = if crate::field_locations::derived::candidate(node, self.facts) {
            "data.indirect-place"
        } else {
            "data.place"
        };
        self.address_operation(node, flow, construct)
    }

    fn address_operation(&mut self, node: &HirNode, flow: Flow, construct: &str) -> Flow {
        let operation = Operation {
            operands: self.operand(&flow.value),
            construct: Some(construct.into()),
            ..template(node, NodeCategory::Computation)
        };
        self.emit(operation, flow)
    }

    fn field_base(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let base = &node.children[0];
        if crate::data_places::is_place_base(base).expect("verified field base type") {
            self.field_place(base, flow)
        } else {
            self.expression(base, flow)
        }
    }

    fn read_operation(&self, node: &HirNode) -> Operation {
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

    fn declaration(&mut self, node: &HirNode, flow: Flow) -> Flow {
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

    fn evaluate_operands(&mut self, node: &HirNode, flow: Flow) -> Operands {
        let mut evaluated = Operands {
            flow,
            values: Vec::new(),
        };
        for child in &node.children {
            evaluated.flow = self.expression(child, evaluated.flow);
            if crate::data_aggregates::constructor(node)
                && evaluated.flow.normal
                && evaluated.flow.value.is_none()
            {
                let unit = Operation {
                    literal: Some("()".into()),
                    construct: Some("unit.value".into()),
                    ..template(child, NodeCategory::Computation)
                };
                evaluated.flow = self.emit(unit, evaluated.flow);
            }
            evaluated.values.extend(
                evaluated
                    .flow
                    .value
                    .iter()
                    .filter(|id| crate::data_aggregates::constructor(node) || self.is_value(id))
                    .cloned(),
            );
        }
        evaluated
    }

    fn computation(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let evaluated = self.evaluate_operands(node, flow);
        let category = if matches!(node.kind.as_str(), "expr_method_call" | "expr_call") {
            NodeCategory::Call
        } else {
            NodeCategory::Computation
        };
        let operation = Operation {
            operator: node.dispatch.as_ref().map(|d| d.op.clone()),
            operands: evaluated.values,
            ..template(node, category)
        };

        self.emit(operation, evaluated.flow)
    }

    fn structure(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let evaluated = self.evaluate_operands(node, flow);
        let operation = Operation {
            construct: Some("data.struct".into()),
            literal: Some(
                crate::data_structs::descriptor(node, self.facts)
                    .expect("verified nominal field slots"),
            ),
            operands: evaluated.values,
            ..template(node, NodeCategory::Computation)
        };

        self.emit(operation, evaluated.flow)
    }

    fn assignment(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let flow = self.expression(&node.children[1], flow);
        let mut operation = self.write(node, "reassignment");
        operation.operator = Some("=".to_string());
        operation.operands = self.operand(&flow.value);

        self.emit(operation, flow)
    }

    fn write(&self, node: &HirNode, form: &str) -> Operation {
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

    fn compound(&mut self, node: &HirNode, flow: Flow) -> Flow {
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

    fn compound_values(&mut self, node: &HirNode, flow: Flow) -> Operands {
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
            .filter(|id| self.is_value(id))
            .cloned()
            .collect()
    }

    fn is_value(&self, id: &str) -> bool {
        self.nodes
            .iter()
            .any(|node| node.id == id && node.ty != "unit")
    }

    fn predicate(&mut self, node: &HirNode, flow: Flow) -> Flow {
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

    fn branch(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let predicate = self.condition(node, flow).flow;
        let controller = predicate.value.clone().expect("predicate");
        let arms = self.branch_arms(node, &predicate);

        self.join(node, controller, arms)
    }

    fn branch_arms(&mut self, node: &HirNode, predicate: &Flow) -> Arms {
        let yes = self.expression(&node.children[1], arm(predicate, "true"));
        let no = if let Some(no) = node.children.get(2) {
            self.expression(no, arm(predicate, "false"))
        } else {
            arm(predicate, "false")
        };

        Arms { yes, no }
    }

    fn condition(&mut self, node: &HirNode, flow: Flow) -> Condition {
        let condition = self.expression(&node.children[0], flow);
        let value = condition.value.clone();
        let flow = self.predicate(node, condition);

        Condition { flow, value }
    }

    fn short_circuit(&mut self, node: &HirNode, flow: Flow) -> Flow {
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

    fn join(&mut self, node: &HirNode, controller: String, arms: Arms) -> Flow {
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
        node: &HirNode,
        controller: String,
        alternatives: Vec<Alternative>,
    ) -> String {
        self.merge_selected(
            node,
            MergeSelection {
                controller,
                alternatives,
                selection_kind: None,
            },
        )
    }

    fn merge_selected(&mut self, node: &HirNode, selection: MergeSelection) -> String {
        let MergeSelection {
            controller,
            alternatives,
            selection_kind,
        } = selection;
        let id = format!("n{}", self.nodes.len());
        let guards = normalize(alternatives.iter().flat_map(|a| a.guards.clone()).collect());
        let mut merge = template(node, NodeCategory::Merge);
        merge.id = id.clone();
        merge.original_kind = if selection_kind.is_some() {
            "rstyle.hir-coverage.match_merge"
        } else {
            "compat.rust.hir-r0.expression_merge"
        }
        .to_string();
        self.nodes.push(merge);
        self.sites.push(ValueSite {
            selection_kind,
            pdg_node_id: id.clone(),
            construct_span: node.span,
            controller_pdg_node_id: controller,
            cfg_successor_node_id: None,
            guards,
            alternatives,
        });

        id
    }

    fn scalar_match(&mut self, node: &HirNode, flow: Flow) -> Flow {
        let scrutinee = self.expression(&node.children[0], flow);
        let saved = scrutinee.value.clone().expect("eligible scalar scrutinee");
        let mut state = MatchState {
            remaining: scrutinee,
            completed: Vec::new(),
            controller: None,
        };
        let closing =
            crate::scalar_exhaustiveness::closing_arm(node, self.facts.build.pointer_width.value);
        for (index, arm_node) in node.children[1..].iter().enumerate() {
            state = self.match_arm(
                MatchArm {
                    node,
                    arm: arm_node,
                    saved: &saved,
                    index,
                    exhaustive: closing == Some(index),
                },
                state,
            );
        }

        self.match_join(
            node,
            state.controller.expect("match selector"),
            state.completed,
        )
    }

    fn match_arm(&mut self, input: MatchArm<'_>, state: MatchState) -> MatchState {
        let selection = self.match_selection(&input, state.remaining, state.controller);
        let selection = self.guarded_selection(&input, selection);
        let body = self.expression(
            input.arm.children.last().expect("arm body"),
            selection.matched,
        );

        MatchState {
            remaining: selection.remaining,
            controller: selection.controller,
            completed: state
                .completed
                .into_iter()
                .chain(std::iter::once((input.index, body)))
                .collect(),
        }
    }

    fn match_selection(
        &mut self,
        input: &MatchArm<'_>,
        remaining: Flow,
        controller: Option<String>,
    ) -> ArmSelection {
        let pattern = &input.arm.children[0];
        let operation = self.match_selector(input);
        let selected = self.emit(operation, remaining);
        let controller = controller.or_else(|| selected.value.clone());
        let remaining = if !input.exhaustive && !crate::scalar_exhaustiveness::irrefutable(pattern)
        {
            arm(&selected, "false")
        } else {
            empty_flow()
        };
        let matched = if pattern.kind != "pat_wild" {
            arm(&selected, "true")
        } else {
            selected
        };

        ArmSelection {
            remaining,
            matched,
            controller,
        }
    }

    fn guarded_selection(&mut self, input: &MatchArm<'_>, selection: ArmSelection) -> ArmSelection {
        if input.arm.children.len() != 3 {
            return selection;
        }
        let (matched, failed, predicate) = self.match_guard(input, selection.matched);
        let controller = if input.index == 0 && input.arm.children[0].kind == "pat_wild" {
            Some(predicate)
        } else {
            selection.controller
        };

        ArmSelection {
            remaining: combine_frontiers(selection.remaining, failed),
            matched,
            controller,
        }
    }

    fn match_guard(&mut self, input: &MatchArm<'_>, matched: Flow) -> (Flow, Flow, String) {
        let MatchArm {
            node,
            arm: arm_node,
            index,
            ..
        } = input;
        let guard = &arm_node.children[1].children[0];
        let evaluated = self.expression(guard, matched);
        let operation = Operation {
            construct_span: node.span,
            role: format!("match_guard:{index}"),
            ty: "bool".into(),
            construct: Some("match.guard".into()),
            operands: self.operand(&evaluated.value),
            ..template(guard, NodeCategory::Control)
        };
        let predicate = self.emit(operation, evaluated);

        (
            arm(&predicate, "true"),
            arm(&predicate, "false"),
            predicate.value.expect("guard predicate"),
        )
    }

    fn match_selector(&self, input: &MatchArm<'_>) -> Operation {
        let MatchArm {
            node, saved, index, ..
        } = input;
        let pattern = &input.arm.children[0];

        let (construct, literal) = match_predicate(pattern);

        Operation {
            construct_span: node.span,
            role: format!("match_pattern:{index}"),
            ty: "bool".into(),
            construct: Some(construct.into()),
            literal,
            operands: if pattern.kind != "pat_wild" {
                vec![(*saved).into()]
            } else {
                Vec::new()
            },
            ..template(pattern, NodeCategory::Control)
        }
    }

    fn match_join(
        &mut self,
        node: &HirNode,
        controller: String,
        completed: Vec<(usize, Flow)>,
    ) -> Flow {
        let alternatives: Vec<_> = completed
            .iter()
            .filter(|(_, f)| f.normal && !f.ends.is_empty())
            .filter_map(|(index, flow)| self.alternative(&format!("arm:{index}"), flow))
            .collect();
        let value = if alternatives.len() > 1 {
            Some(self.merge_selected(
                node,
                MergeSelection {
                    controller,
                    alternatives,
                    selection_kind: Some("ordered-match/1".into()),
                },
            ))
        } else {
            alternatives.first().map(|a| a.value_pdg_node_id.clone())
        };
        let normal = completed.iter().any(|(_, f)| f.normal);
        let ends = completed
            .into_iter()
            .filter(|(_, f)| f.normal)
            .flat_map(|(_, f)| f.ends)
            .collect();

        Flow {
            ends,
            value,
            normal,
        }
    }

    fn explicit_return(&mut self, node: &HirNode, flow: Flow) -> Flow {
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
        let anchor = tail.cloned().unwrap_or_else(|| HirNode {
            span: ByteSpan {
                start: body.span.end - 1,
                end: body.span.end,
            },
            snippet: "}".to_string(),
            ..body.clone()
        });

        self.return_operation(&anchor, "implicit", flow);
    }

    fn return_operation(&mut self, node: &HirNode, form: &str, flow: Flow) -> Flow {
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
            loop_role: None,
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

fn lowerer<'a>(node: &HirNode) -> fn(&mut Builder<'a>, &HirNode, Flow) -> Flow {
    match node.kind.as_str() {
        "expr_block" => Builder::block,
        "expr_assert" => Builder::assertion,
        "stmt_expr" | "stmt_semi" => Builder::expression_statement,
        "stmt_let" => Builder::declaration,
        "expr_path"
            if node
                .res
                .as_ref()
                .is_some_and(|res| res.kind == crate::scalar_constants::RESOLUTION) =>
        {
            Builder::constant
        }
        "expr_path" => Builder::read,
        "expr_field" => Builder::field_read,
        "expr_addr_of" => Builder::borrow,
        "expr_tup" if !node.children.is_empty() => Builder::computation,
        "expr_struct" => Builder::structure,
        "expr_array" | "expr_repeat" => Builder::computation,
        "expr_lit" | "expr_tup" => Builder::literal,
        "expr_binary"
            if node
                .dispatch
                .as_ref()
                .is_some_and(|d| d.op == "&&" || d.op == "||") =>
        {
            Builder::short_circuit
        }
        "expr_binary" | "expr_unary" | "expr_cast" | "expr_method_call" | "expr_call" => {
            Builder::computation
        }
        "expr_assign" => Builder::assignment,
        "expr_assign_op" => Builder::compound,
        "expr_if" => Builder::branch,
        "expr_match" => Builder::scalar_match,
        "expr_while" => Builder::while_loop,
        "expr_loop" => Builder::infinite_loop,
        "expr_break" | "expr_continue" => Builder::loop_exit,
        "expr_ret" => Builder::explicit_return,
        _ => unreachable!("eligibility rejects unsupported constructs"),
    }
}

fn match_predicate(pattern: &HirNode) -> (&'static str, Option<String>) {
    match pattern.kind.as_str() {
        "pat_literal" => ("match.literal", pattern.lit.clone()),
        "pat_constant" => ("match.constant", pattern.lit.clone()),
        "pat_range" => ("match.range", pattern.lit.clone()),
        "pat_or" => match_alternatives(pattern),
        _ => ("match.wildcard", None),
    }
}

fn match_alternatives(pattern: &HirNode) -> (&'static str, Option<String>) {
    if !pattern.children.iter().any(|child| child.kind == "pat_or") {
        return encode_alternatives(pattern.children.iter());
    }
    let mut children = Vec::new();
    flatten_alternatives(pattern, &mut children);
    encode_alternatives(children.iter().copied())
}

fn encode_alternatives<'a>(
    children: impl Iterator<Item = &'a HirNode> + Clone,
) -> (&'static str, Option<String>) {
    if children.clone().all(|child| child.kind == "pat_literal") {
        let values: Vec<_> = children.map(|child| &child.lit).collect();
        return (
            "match.or",
            Some(serde_json::to_string(&values).expect("literal alternatives")),
        );
    }
    let values: Vec<_> = children.map(|child| (&child.kind, &child.lit)).collect();

    (
        "match.or-patterns",
        Some(serde_json::to_string(&values).expect("pattern alternatives")),
    )
}

fn flatten_alternatives<'a>(pattern: &'a HirNode, leaves: &mut Vec<&'a HirNode>) {
    if pattern.kind == "pat_or" {
        for child in &pattern.children {
            flatten_alternatives(child, leaves);
        }
    } else {
        leaves.push(pattern);
    }
}

fn template(node: &HirNode, category: NodeCategory) -> Operation {
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

fn empty_flow() -> Flow {
    Flow {
        ends: Vec::new(),
        value: None,
        normal: false,
    }
}

fn combine_frontiers(mut remaining: Flow, failed: Flow) -> Flow {
    remaining.ends.extend(failed.ends);
    remaining.normal |= failed.normal;

    remaining
}

struct MatchState {
    remaining: Flow,
    completed: Vec<(usize, Flow)>,
    controller: Option<String>,
}

struct MatchArm<'a> {
    node: &'a HirNode,
    arm: &'a HirNode,
    saved: &'a str,
    index: usize,
    exhaustive: bool,
}

struct ArmSelection {
    remaining: Flow,
    matched: Flow,
    controller: Option<String>,
}

struct MergeSelection {
    controller: String,
    alternatives: Vec<Alternative>,
    selection_kind: Option<String>,
}

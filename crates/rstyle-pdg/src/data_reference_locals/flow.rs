//! Source kill/gen over existing binding identities. Values name original RHS
//! operations, so copying a reference retains its earlier read snapshot.

use crate::{HirFnFacts, HirNode};
use std::collections::{BTreeMap, BTreeSet};

type State = BTreeMap<u32, BTreeSet<u32>>;

#[derive(Default)]
struct Transfer {
    normal: Option<State>,
    breaks: Option<State>,
    continues: Option<State>,
}

impl Transfer {
    fn normal(state: State) -> Self {
        Self {
            normal: Some(state),
            ..Self::default()
        }
    }

    fn merge(&mut self, other: Self) {
        join(&mut self.normal, other.normal);
        join(&mut self.breaks, other.breaks);
        join(&mut self.continues, other.continues);
    }
}

fn join(target: &mut Option<State>, source: Option<State>) {
    if let Some(source) = source {
        let target = target.get_or_insert_with(State::new);
        for (binding, definitions) in source {
            target.entry(binding).or_default().extend(definitions);
        }
    }
}

pub(super) enum Value<'a> {
    Parameter(&'a crate::ParamFact),
    Expression(&'a HirNode),
}

pub(super) struct Index<'a> {
    pub values: BTreeMap<u32, Value<'a>>,
    pub reads: BTreeMap<u32, BTreeSet<u32>>,
    width: Option<u64>,
}

impl<'a> Index<'a> {
    pub fn new(facts: &'a HirFnFacts) -> Result<Self, String> {
        let mut index = Self {
            values: BTreeMap::new(),
            reads: BTreeMap::new(),
            width: facts.build.pointer_width.value,
        };
        let mut entry = State::new();
        for parameter in &facts.signature.params {
            let id = parameter.pat.hir.local;
            if reference(&parameter.pat) {
                index.values.insert(id, Value::Parameter(parameter));
                entry.insert(id, BTreeSet::from([id]));
            }
        }

        index.walk(&facts.body, entry)?;
        Ok(index)
    }

    fn walk(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        if node.kind == "expr_path" {
            return self.read(node, state);
        }
        if node.kind == "expr_assert" && node.lit.as_deref() == Some("false") {
            self.children(node, state.clone())?;
            return Ok(Transfer::normal(state));
        }

        dispatch(node)(self, node, state)
    }

    fn children(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        self.sequence(&node.children, state)
    }

    fn indexed(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        let [base, index] = node.children.as_slice() else {
            return Err("Index flow lacks two operands".into());
        };
        let builtin = node
            .dispatch
            .as_ref()
            .is_some_and(|dispatch| dispatch.kind == "builtin");
        if builtin && crate::data_index_reads::delayed_base(base) {
            self.sequence([index, base], state)
        } else {
            self.children(node, state)
        }
    }

    fn returned(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        self.sequence(&node.children, state)?;
        Ok(Transfer::default())
    }

    fn interrupted(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        Ok(if node.kind == "expr_break" {
            Transfer {
                breaks: Some(state),
                ..Transfer::default()
            }
        } else {
            Transfer {
                continues: Some(state),
                ..Transfer::default()
            }
        })
    }

    fn read(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        if reference(node)
            && let Some(target) = node
                .res
                .as_ref()
                .and_then(|resolution| resolution.target.as_ref())
        {
            self.reads
                .entry(node.hir.local)
                .or_default()
                .extend(state.get(&target.local).into_iter().flatten());
        }
        Ok(Transfer::normal(state))
    }

    fn matched(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        let (scrutinee, arms) = node
            .children
            .split_first()
            .ok_or("Missing match scrutinee")?;
        let closing = crate::scalar_exhaustiveness::closing_arm(node, self.width);
        let mut flow = MatchFlow::new(self.walk(scrutinee, state)?);

        for (position, arm) in arms.iter().enumerate() {
            let Some(state) = flow.remaining.take() else {
                break;
            };
            let (rest, selected) = self.match_arm(arm, state, closing == Some(position))?;
            flow.advance(rest, selected);
        }
        Ok(flow.completed)
    }

    fn match_arm(
        &mut self,
        arm: &'a HirNode,
        state: State,
        closing: bool,
    ) -> Result<(Option<State>, Transfer), String> {
        let pattern = arm.children.first().ok_or("Missing source match pattern")?;
        let body = arm.children.last().ok_or("Missing source match body")?;
        let mut remaining = (!closing && !crate::scalar_exhaustiveness::irrefutable(pattern))
            .then(|| state.clone());

        let mut selected = if arm.children.len() == 3 {
            self.walk(&arm.children[1], state)?
        } else {
            Transfer::normal(state)
        };
        if arm.children.len() == 3 {
            join(&mut remaining, selected.normal.clone());
        }
        if let Some(state) = selected.normal.take() {
            selected.merge(self.walk(body, state)?);
        }
        Ok((remaining, selected))
    }

    fn sequence(
        &mut self,
        nodes: impl IntoIterator<Item = &'a HirNode>,
        state: State,
    ) -> Result<Transfer, String> {
        let mut result = Transfer::normal(state);
        for node in nodes {
            let Some(state) = result.normal.take() else {
                break;
            };
            result.merge(self.walk(node, state)?);
        }
        Ok(result)
    }

    fn define(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        let Some(definition) = definition(node)? else {
            return self.children(node, state);
        };
        if !reference(definition.value) {
            return self.walk(definition.value, state);
        }
        self.reference_definition(definition, state)
    }

    fn reference_definition(
        &mut self,
        definition: Definition<'a>,
        state: State,
    ) -> Result<Transfer, String> {
        if definition.target.kind == "expr_path" {
            self.walk(definition.target, state.clone())?;
        }
        self.values
            .insert(definition.id, Value::Expression(definition.value));

        let mut result = self.walk(definition.value, state)?;
        if let Some(state) = &mut result.normal {
            state.insert(definition.binding, BTreeSet::from([definition.id]));
        }
        Ok(result)
    }

    fn branch(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        let (predicate, branches) = node
            .children
            .split_first()
            .ok_or("Missing flow predicate")?;
        let mut result = self.walk(predicate, state)?;
        if let Some(state) = result.normal.take() {
            for branch in branches {
                result.merge(self.walk(branch, state.clone())?);
            }
            if branches.len() == 1 {
                join(&mut result.normal, Some(state));
            }
        }
        Ok(result)
    }

    fn short_circuit(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        let [left, right] = node.children.as_slice() else {
            return Err("Missing short-circuit flow operands".into());
        };
        let mut result = self.walk(left, state)?;
        if let Some(state) = result.normal.clone() {
            result.merge(self.walk(right, state)?);
        }
        Ok(result)
    }

    fn looped(&mut self, node: &'a HirNode, entry: State) -> Result<Transfer, String> {
        let mut header = entry.clone();
        loop {
            let iteration = self.iteration(node, header.clone())?;
            let next = loop_header(&entry, &iteration);
            if next == header {
                return self.loop_exits(node, header, iteration.breaks);
            }
            header = next;
        }
    }

    fn loop_exits(
        &mut self,
        node: &'a HirNode,
        header: State,
        mut exits: Option<State>,
    ) -> Result<Transfer, String> {
        if node.kind == "expr_while" && node.children[0].lit.as_deref() != Some("true") {
            let predicate = self.walk(&node.children[0], header)?;
            join(&mut exits, predicate.normal);
        }
        Ok(Transfer {
            normal: exits,
            ..Transfer::default()
        })
    }

    fn iteration(&mut self, node: &'a HirNode, state: State) -> Result<Transfer, String> {
        if node.kind == "expr_loop" {
            let body = node.children.first().ok_or("Missing loop body")?;
            return self.walk(body, state);
        }
        let [predicate, body] = node.children.as_slice() else {
            return Err("Missing while predicate/body".into());
        };
        let mut result = self.walk(predicate, state)?;

        if predicate.lit.as_deref() == Some("false") {
            return Ok(Transfer::default());
        }
        if let Some(state) = result.normal.take() {
            result.merge(self.walk(body, state)?);
        }
        Ok(result)
    }
}

fn reference(node: &HirNode) -> bool {
    node.ty
        .as_ref()
        .and_then(|ty| ty.normalized.as_deref())
        .and_then(|token| crate::data_types::parse(token).ok())
        .is_some_and(|ty| crate::data_origins::shared(&ty))
}

fn loop_header(entry: &State, iteration: &Transfer) -> State {
    let mut next = Some(entry.clone());
    join(&mut next, iteration.normal.clone());
    join(&mut next, iteration.continues.clone());
    next.unwrap()
}

type Handler<'a> = fn(&mut Index<'a>, &'a HirNode, State) -> Result<Transfer, String>;

fn dispatch<'a>(node: &HirNode) -> Handler<'a> {
    match node.kind.as_str() {
        "expr_if" => Index::branch,
        "expr_index" => Index::indexed,
        "expr_while" | "expr_loop" => Index::looped,
        "expr_break" | "expr_continue" => Index::interrupted,
        "expr_ret" => Index::returned,
        "stmt_let" | "expr_assign" => Index::define,
        "expr_match" => Index::matched,
        "expr_binary"
            if node
                .dispatch
                .as_ref()
                .is_some_and(|dispatch| matches!(dispatch.op.as_str(), "&&" | "||")) =>
        {
            Index::short_circuit
        }
        _ => Index::children,
    }
}

struct MatchFlow {
    remaining: Option<State>,
    completed: Transfer,
}

impl MatchFlow {
    fn new(mut completed: Transfer) -> Self {
        Self {
            remaining: completed.normal.take(),
            completed,
        }
    }

    fn advance(&mut self, remaining: Option<State>, selected: Transfer) {
        self.remaining = remaining;
        self.completed.merge(selected);
    }
}

struct Definition<'a> {
    id: u32,
    binding: u32,
    target: &'a HirNode,
    value: &'a HirNode,
}

fn definition(node: &HirNode) -> Result<Option<Definition<'_>>, String> {
    let (target, value) = if node.kind == "stmt_let" {
        (
            node.children
                .iter()
                .find(|child| child.kind == "pat_binding"),
            node.children
                .iter()
                .find(|child| child.kind.starts_with("expr_")),
        )
    } else {
        (node.children.first(), node.children.get(1))
    };
    let (Some(target), Some(value)) = (target, value) else {
        return Ok(None);
    };

    let binding = if node.kind == "stmt_let" {
        Some(&target.hir)
    } else {
        target
            .res
            .as_ref()
            .and_then(|resolution| resolution.target.as_ref())
    }
    .ok_or("Reference flow assignment lacks a resolved local target")?;
    Ok(Some(Definition {
        id: node.hir.local,
        binding: binding.local,
        target,
        value,
    }))
}

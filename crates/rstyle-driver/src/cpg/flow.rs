//! A function's control flow over its emitted tree, in evaluation order. A node runs after
//! the operands it evaluates, so a construct's vertex is where its paths rejoin; a `for`,
//! which has nothing to rejoin at, tests for another item at its own. A closure's body is
//! opaque, a coroutine's runs in place, and a panic ends its path without reaching an exit.

use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

const ENTRY: &str = "entry";
const EXIT: &str = "exit";
const BACK_EDGE: &str = "back_edge";
const LOOP_EXIT: &str = "loop_exit";

/// An execution transfer waiting for the vertex it reaches.
#[derive(Clone, Copy)]
struct Transfer<'a> {
    source: &'a str,
    kind: &'static str,
    role: Option<&'static str>,
}

type Transfers<'a> = Vec<Transfer<'a>>;

fn retagged<'a>(transfers: &[Transfer<'a>], kind: &'static str) -> Transfers<'a> {
    transfers
        .iter()
        .map(|transfer| Transfer { kind, ..*transfer })
        .collect()
}

fn leaving<'a>(mut transfers: Transfers<'a>) -> Transfers<'a> {
    for transfer in &mut transfers {
        transfer.role = Some(LOOP_EXIT);
    }
    transfers
}

fn repeating_transfers<'a>(mut transfers: Transfers<'a>) -> Transfers<'a> {
    for transfer in &mut transfers {
        transfer.role = Some(BACK_EDGE);
    }
    transfers
}

fn joined<'a>(mut first: Transfers<'a>, second: Transfers<'a>) -> Transfers<'a> {
    first.extend(second);
    first
}

fn children_of(node: &Value) -> Vec<&str> {
    node["children"]
        .as_array()
        .map(|children| children.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// What a `break` or `continue` can leave or repeat.
struct Scope<'a> {
    label: Option<&'a str>,
    is_loop: bool,
    breaks: Transfers<'a>,
    continues: Transfers<'a>,
}

impl<'a> Scope<'a> {
    fn new(label: Option<&'a str>, is_loop: bool) -> Self {
        Self {
            label,
            is_loop,
            breaks: Vec::new(),
            continues: Vec::new(),
        }
    }
}

/// A recorded transfer.
struct Edge<'a> {
    source: &'a str,
    target: &'a str,
    kind: &'static str,
    role: Option<&'static str>,
}

/// A coroutine being walked: what returns from it, and the scopes around it, which what
/// runs inside it does not leave.
struct Coroutine<'a> {
    returned: Transfers<'a>,
    outer_scopes: usize,
}

/// What the walk reads of a function besides its tree: which calls never return, and which
/// operators are calls, whose operands are evaluated in the order they are written.
pub struct Inputs<'a> {
    pub root: &'a str,
    pub diverging: &'a HashSet<String>,
    pub refs: &'a [Value],
}

struct Walk<'a> {
    nodes: HashMap<&'a str, &'a Value>,
    diverging: &'a HashSet<String>,
    overloaded: HashSet<&'a str>,
    scopes: Vec<Scope<'a>>,
    /// Each coroutine being walked, innermost last.
    coroutines: Vec<Coroutine<'a>>,
    edges: Vec<Edge<'a>>,
}

/// The control flow of a function with these nodes.
pub fn cfg(nodes: &[Value], inputs: Inputs<'_>) -> Value {
    let mut walk = Walk {
        nodes: nodes
            .iter()
            .filter_map(|node| Some((node["id"].as_str()?, node)))
            .collect(),
        diverging: inputs.diverging,
        overloaded: inputs
            .refs
            .iter()
            .filter_map(|reference| reference["node"].as_str())
            .collect(),
        scopes: Vec::new(),
        coroutines: Vec::new(),
        edges: Vec::new(),
    };
    let entry = vec![Transfer {
        source: ENTRY,
        kind: "normal",
        role: None,
    }];
    let finished = walk.visit(inputs.root, entry);
    walk.connect(&finished, EXIT);

    let edges: Vec<Value> = walk.edges.iter().enumerate().map(edge_record).collect();

    json!({"entry": ENTRY, "exits": [{"id": EXIT, "kind": "normal"}], "edges": edges})
}

fn edge_record((index, edge): (usize, &Edge<'_>)) -> Value {
    let mut record = json!({"id": format!("e{index}"), "source": edge.source,
                            "target": edge.target, "kind": edge.kind});
    if let Some(role) = edge.role {
        record["loop_role"] = json!(role);
    }
    record
}

impl<'a> Walk<'a> {
    fn connect(&mut self, transfers: &[Transfer<'a>], target: &'a str) {
        for transfer in transfers {
            self.edges.push(Edge {
                source: transfer.source,
                target,
                kind: transfer.kind,
                role: transfer.role,
            });
        }
    }

    /// Records the node as a vertex these transfers reach, unless none reach it.
    fn vertex(&mut self, id: &'a str, incoming: Transfers<'a>) -> Transfers<'a> {
        if incoming.is_empty() {
            return incoming;
        }
        self.connect(&incoming, id);
        vec![Transfer {
            source: id,
            kind: "normal",
            role: None,
        }]
    }

    fn sequence(&mut self, ids: &[&'a str], incoming: Transfers<'a>) -> Transfers<'a> {
        ids.iter()
            .fold(incoming, |flowing, id| self.visit(id, flowing))
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    /// Evaluates the node, then whatever it is written after, then what it does.
    fn visit(&mut self, id: &'a str, incoming: Transfers<'a>) -> Transfers<'a> {
        if incoming.is_empty() {
            return incoming;
        }
        let node = self.nodes[id];
        let children = children_of(node);
        match node["kind"].as_str().unwrap_or_default() {
            "type_annotation" | "item_declaration" => incoming,
            "closure" => self.closure((id, node), &children, incoming),
            "block" => self.block((id, node), &children, incoming),
            "if" => self.branch(id, &children, incoming),
            "pattern_condition" => {
                let (yes, no) = self.condition(id, incoming);
                joined(yes, no)
            }
            "operator" => self.operator((id, node), &children, incoming),
            "match" => self.selection(id, &children, incoming),
            "while" | "loop" | "for" => self.repetition((id, node), &children, incoming),
            "local_declaration" => self.declaration(id, &children, incoming),
            "assignment" => self.assignment(id, &children, incoming),
            "compound_assignment" => self.compound(id, &children, incoming),
            "propagate" | "return" | "break" | "continue" => {
                self.jump((id, node), &children, incoming)
            }
            _ => {
                let evaluated = self.sequence(&children, incoming);
                let done = self.vertex(id, evaluated);
                match self.diverging.contains(id) && node["kind"] == "call" {
                    true => Vec::new(),
                    false => done,
                }
            }
        }
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    fn closure(
        &mut self,
        (id, node): (&'a str, &'a Value),
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let coroutine = node["modifiers"]
            .as_array()
            .is_some_and(|modifiers| modifiers.iter().any(|modifier| modifier == "coroutine"));
        if !coroutine {
            return self.vertex(id, incoming);
        }
        self.coroutines.push(Coroutine {
            returned: Vec::new(),
            outer_scopes: self.scopes.len(),
        });
        let finished = self.sequence(children, incoming);
        let returned = self
            .coroutines
            .pop()
            .map(|coroutine| coroutine.returned)
            .unwrap_or_default();

        self.vertex(id, joined(finished, returned))
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    fn block(
        &mut self,
        (id, node): (&'a str, &'a Value),
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let label = node["label"].as_str();
        if label.is_some() {
            self.scopes.push(Scope::new(label, false));
        }
        let mut finished = self.sequence(children, incoming);
        if label.is_some()
            && let Some(scope) = self.scopes.pop()
        {
            finished.extend(scope.breaks);
        }

        self.vertex(id, finished)
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    fn branch(
        &mut self,
        id: &'a str,
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let (condition, then, otherwise) = (children[0], children[1], children.get(2));
        let (yes, no) = self.condition(condition, incoming);
        let done = self.visit(then, yes);
        let other = match otherwise {
            Some(otherwise) => self.visit(otherwise, no),
            None => no,
        };

        self.vertex(id, joined(done, other))
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    /// Where the condition leads when it holds and when it does not. An operator that only
    /// steers a branch has no vertex of its own: its operands branch.
    fn condition(
        &mut self,
        id: &'a str,
        incoming: Transfers<'a>,
    ) -> (Transfers<'a>, Transfers<'a>) {
        let node = self.nodes[id];
        let children = children_of(node);
        match (node["kind"].as_str(), node["operator"].as_str()) {
            (Some("operator"), Some("and")) => {
                let (first, failed) = self.condition(children[0], incoming);
                let (both, second_failed) = self.condition(children[1], first);
                (both, joined(failed, second_failed))
            }
            (Some("operator"), Some("or")) => {
                let (held, failed) = self.condition(children[0], incoming);
                let (second_held, neither) = self.condition(children[1], failed);
                (joined(held, second_held), neither)
            }
            (Some("pattern_condition"), _) => {
                let scrutinee = self.visit(children[1], incoming);
                let test = self.vertex(id, scrutinee);
                let bound = self.visit(children[0], retagged(&test, "true"));
                (bound, retagged(&test, "false"))
            }
            _ => {
                let done = self.visit(id, incoming);
                (retagged(&done, "true"), retagged(&done, "false"))
            }
        }
    }

    fn operator(
        &mut self,
        (id, node): (&'a str, &'a Value),
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let flowing = match node["operator"].as_str() {
            Some("and" | "or") => {
                let (yes, no) = self.condition(id, incoming);
                joined(yes, no)
            }
            _ => self.sequence(children, incoming),
        };

        self.vertex(id, flowing)
    }

    /// A match's arms leave what it tests, and rejoin at its own vertex. A guard that fails
    /// falls to the next arm.
    fn selection(
        &mut self,
        id: &'a str,
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let tested = self.visit(children[0], incoming);
        let arms = &children[1..];
        let mut finished = Vec::new();
        let mut failed = Vec::new();
        for (position, arm) in arms.iter().enumerate() {
            let kind = match position + 1 == arms.len() && self.catches_all(arm) {
                true => "default",
                false => "case",
            };
            let into = joined(retagged(&tested, kind), std::mem::take(&mut failed));
            let (done, guard_failed) = self.arm(arm, into);
            finished.extend(done);
            failed = guard_failed;
        }

        self.vertex(id, finished)
    }

    fn catches_all(&self, arm: &str) -> bool {
        let children = children_of(self.nodes[arm]);
        let unguarded = children
            .iter()
            .all(|child| self.nodes[child]["kind"] != "match_guard");
        let pattern = children.first().map(|pattern| self.nodes[pattern]);
        unguarded
            && pattern.is_some_and(|pattern| {
                pattern["kind"] == "pattern_wildcard"
                    || pattern["kind"] == "pattern_binding" && children_of(pattern).is_empty()
            })
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    fn arm(&mut self, id: &'a str, incoming: Transfers<'a>) -> (Transfers<'a>, Transfers<'a>) {
        let children = children_of(self.nodes[id]);
        let bound = self.visit(children[0], incoming);
        let (body, guard) = (children[children.len() - 1], children.get(1));
        let guard = guard.filter(|guard| self.nodes[*guard]["kind"] == "match_guard");
        let Some(guard) = guard else {
            return (self.visit(body, bound), Vec::new());
        };
        let condition = children_of(self.nodes[*guard])[0];
        let (yes, no) = self.condition(condition, bound);

        (self.visit(body, yes), no)
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    /// A `while` repeats what it tests and a `loop` its body, from the first thing each
    /// evaluates; both rejoin at their own vertex. A `for` evaluates what it iterates once, and
    /// its vertex tests for another item each pass.
    fn repetition(
        &mut self,
        (id, node): (&'a str, &'a Value),
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let kind = node["kind"].as_str();
        let before = match kind {
            Some("for") => self.visit(children[1], incoming),
            _ => incoming,
        };
        self.scopes.push(Scope::new(node["label"].as_str(), true));

        let start = self.edges.len();
        let (repeated, finished, head) = match kind {
            Some("while") => {
                let (yes, no) = self.condition(children[0], before);
                let head = self.edges.get(start).map(|edge| edge.target);
                (self.visit(children[1], yes), no, head)
            }
            Some("for") => {
                let tested = self.vertex(id, before);
                let bound = self.visit(children[0], retagged(&tested, "true"));
                let body = self.visit(children[2], bound);
                (body, retagged(&tested, "false"), Some(id))
            }
            _ => {
                let body = self.visit(children[0], before);
                let head = self.edges.get(start).map(|edge| edge.target);
                (body, Vec::new(), head)
            }
        };
        let scope = self.scopes.pop().unwrap_or_else(|| Scope::new(None, true));

        if let Some(head) = head {
            let repeating = joined(repeated, scope.continues);
            self.connect(&repeating_transfers(repeating), head);
        }
        let left = joined(leaving(finished), scope.breaks);
        match kind {
            Some("for") => left,
            _ => self.vertex(id, left),
        }
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    /// An initializer runs first; the pattern then binds, or a `let .. else` diverges.
    fn declaration(
        &mut self,
        id: &'a str,
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let (pattern, rest) = (children[0], &children[1..]);
        let rest: Vec<&'a str> = rest
            .iter()
            .copied()
            .filter(|child| self.nodes[child]["kind"] != "type_annotation")
            .collect();
        let evaluated = self.sequence(&rest[..rest.len().min(1)], incoming);
        let declared = self.vertex(id, evaluated);
        let Some(otherwise) = rest.get(1) else {
            return self.visit(pattern, declared);
        };
        let bound = self.visit(pattern, retagged(&declared, "true"));
        let failed = self.visit(otherwise, retagged(&declared, "false"));

        joined(bound, failed)
    }

    /// The value of an assignment is evaluated before the place it is written to.
    fn assignment(
        &mut self,
        id: &'a str,
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let order: Vec<&'a str> = children.iter().take(2).rev().copied().collect();
        let evaluated = self.sequence(&order, incoming);

        self.vertex(id, evaluated)
    }

    /// A built-in compound assignment evaluates its value first; an overloaded one is a
    /// call, whose operands run in the order they are written.
    fn compound(
        &mut self,
        id: &'a str,
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        if self.overloaded.contains(id) {
            let evaluated = self.sequence(children, incoming);
            return self.vertex(id, evaluated);
        }
        self.assignment(id, children, incoming)
    }

    // rstyle-allow(mutation_per_target): a walk accumulates its transfers and scopes in self by design
    fn jump(
        &mut self,
        (id, node): (&'a str, &'a Value),
        children: &[&'a str],
        incoming: Transfers<'a>,
    ) -> Transfers<'a> {
        let kind = node["kind"].as_str().unwrap_or_default();
        let operands = match kind {
            "propagate" => &children[..1],
            _ => children,
        };
        let evaluated = self.sequence(operands, incoming);

        let at = self.vertex(id, evaluated);
        match kind {
            "propagate" => {
                self.returned(&at);
                at
            }
            "return" => {
                self.returned(&at);
                Vec::new()
            }
            "break" => {
                self.broke(&at, node["label"].as_str());
                Vec::new()
            }
            _ => {
                self.continued(&at, node["label"].as_str());
                Vec::new()
            }
        }
    }

    /// A return leaves every loop around it, within what it returns from.
    fn returned(&mut self, at: &[Transfer<'a>]) {
        let floor = self.coroutines.last().map_or(0, |inner| inner.outer_scopes);
        let mut transfers = retagged(at, "return");
        if self.scopes[floor..].iter().any(|scope| scope.is_loop) {
            transfers = leaving(transfers);
        }
        match self.coroutines.last_mut() {
            Some(coroutine) => coroutine.returned.extend(transfers),
            None => self.connect(&transfers, EXIT),
        }
    }

    /// A break leaves its target and whatever is inside it, loops among them or not.
    fn broke(&mut self, at: &[Transfer<'a>], label: Option<&str>) {
        let found = self.scopes.iter().rposition(|scope| match label {
            Some(label) => scope.label == Some(label),
            None => scope.is_loop,
        });
        let Some(found) = found else {
            return;
        };
        let mut transfers = retagged(at, "break");

        if self.scopes[found..].iter().any(|scope| scope.is_loop) {
            transfers = leaving(transfers);
        }
        self.scopes[found].breaks.extend(transfers);
    }

    fn continued(&mut self, at: &[Transfer<'a>], label: Option<&str>) {
        let found = self.scopes.iter_mut().rev().find(|scope| match label {
            Some(label) => scope.label == Some(label),
            None => scope.is_loop,
        });
        if let Some(scope) = found {
            scope.continues.extend(retagged(at, "continue"));
        }
    }
}

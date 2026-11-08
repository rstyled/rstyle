//! Canonicalizing desugar pass over dependence graphs.
//!
//! Two functions that do the same thing but spell control flow or value construction differently
//! have different graphs, and no scorer can fix a different shape. Each rule here rewrites one
//! spelling into a shared core form before hashing and aligning. Rules are pure
//! `Pdg -> Pdg` functions, applied in a fixed order, and switched on one by one by `Rules`.
//!
//! Invariants (tested): idempotent, node count never grows, original `detail` text survives on
//! the rewritten node so holes can still be reported.

use crate::facts::{EdgeKind, NodeKind, Pdg, PdgEdge, PdgNode};
use std::borrow::Cow;
use std::collections::BTreeSet;

/// Which rules run. Explicit value, no globals: callers pass it down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// R1: value construction collapses into one `Ctor` node.
    pub ctor: bool,
    /// R2: `&&`, `||`, `!` over predicate calls disappear; the calls feed the consumer.
    pub pred: bool,
    /// R3: `if`, `if let`, `match`, `== literal` conditions and else-if chains become one `Case`.
    pub case: bool,
    /// R4: `unreachable!()` after a loop is dropped.
    pub exit: bool,
    /// R5/R6: range `for` loops, `while`/`loop` with a manual counter and `while i < N` become one `Loop`
    /// whose node is the counter.
    pub counter: bool,
    /// R7: comparing the counter with a literal is one `cmp` operator whatever the operator.
    pub cmp: bool,
    /// R8: `match r { Ok(..) => .., Err(e) => return Err(e) }` is the `?` operator.
    pub propagate: bool,
}

impl Rules {
    pub const ALL: Rules = Rules {
        ctor: true,
        pred: true,
        case: true,
        exit: true,
        counter: true,
        cmp: true,
        propagate: true,
    };

    pub fn is_none(&self) -> bool {
        *self == Rules::default()
    }

    /// `"ctor,pred"`, `"all"` or `""`.
    pub fn parse(text: &str) -> Result<Rules, String> {
        text.split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .try_fold(Rules::default(), |rules, name| match name {
                "all" => Ok(Rules::ALL),
                "ctor" => Ok(Rules {
                    ctor: true,
                    ..rules
                }),
                "pred" => Ok(Rules {
                    pred: true,
                    ..rules
                }),
                "case" => Ok(Rules {
                    case: true,
                    ..rules
                }),
                "exit" => Ok(Rules {
                    exit: true,
                    ..rules
                }),
                "counter" => Ok(Rules {
                    counter: true,
                    ..rules
                }),
                "cmp" => Ok(Rules { cmp: true, ..rules }),
                "try" => Ok(Rules {
                    propagate: true,
                    ..rules
                }),
                other => Err(format!(
                    "unknown normalize rule `{other}` (ctor, pred, case, exit, counter, cmp, try, all)"
                )),
            })
    }
}

/// One rewrite step.
type Rule = fn(Pdg) -> Pdg;

/// Applies the enabled rules in a fixed order (R1, R2, ...).
pub fn canonicalize(pdg: &Pdg, rules: &Rules) -> Pdg {
    canonicalize_or_borrow(pdg, rules).into_owned()
}

/// Default proposal policy leaves graphs unchanged, so retain their original owner.
pub(crate) fn canonicalize_or_borrow<'a>(pdg: &'a Pdg, rules: &Rules) -> Cow<'a, Pdg> {
    if rules.is_none() {
        return Cow::Borrowed(pdg);
    }
    let steps: [(bool, Rule); 7] = [
        (rules.propagate, propagate_errors),
        (rules.ctor, fold_ctors),
        (rules.pred, collapse_preds),
        (rules.case, unify_case),
        (rules.exit, drop_unreachable),
        (rules.counter, counter_loops),
        (rules.cmp, compare_class),
    ];
    Cow::Owned(
        steps
            .iter()
            .filter(|(on, _)| *on)
            .fold(pdg.clone(), |graph, (_, step)| step(graph)),
    )
}

// ---- graph helpers -------------------------------------------------------------------------

fn data_producers(pdg: &Pdg, node: usize) -> Vec<usize> {
    let ins = pdg
        .edges
        .iter()
        .filter(|e| e.to == node && e.kind == EdgeKind::Data);
    ins.map(|e| e.from).collect()
}

fn data_consumers(pdg: &Pdg, node: usize) -> BTreeSet<usize> {
    let outs = pdg
        .edges
        .iter()
        .filter(|e| e.from == node && e.kind == EdgeKind::Data);
    outs.map(|e| e.to).collect()
}

/// Removes `dead` nodes and every edge touching them; indices are compacted in order.
fn without(pdg: Pdg, dead: &BTreeSet<usize>) -> Pdg {
    let new_index: Vec<Option<usize>> = (0..pdg.nodes.len())
        .scan(0, |next, old| {
            Some((!dead.contains(&old)).then(|| {
                *next += 1;
                *next - 1
            }))
        })
        .collect();
    let nodes = pdg
        .nodes
        .into_iter()
        .enumerate()
        .filter(|(old, _)| new_index[*old].is_some())
        .map(|(_, node)| node);
    let edges = pdg.edges.iter().filter_map(|e| {
        Some(PdgEdge {
            from: new_index[e.from]?,
            to: new_index[e.to]?,
            ..*e
        })
    });
    Pdg {
        nodes: nodes.collect(),
        edges: edges.collect(),
    }
}

/// First occurrence of each edge wins, so adding an edge that exists is a no-op.
fn dedup(edges: Vec<PdgEdge>) -> Vec<PdgEdge> {
    let mut seen = BTreeSet::new();
    edges
        .into_iter()
        .filter(|e| seen.insert((e.from, e.to, e.kind, e.arg_pos)))
        .collect()
}

// ---- R1: value construction ------------------------------------------------------------------

fn is_ctor_op(node: &PdgNode) -> bool {
    node.kind == NodeKind::Op && matches!(node.detail.as_str(), "ctor" | "tuple")
}

/// `None`, `Mode::Fast`: a path literal naming a variant (last segment capitalized, not a CONST).
fn is_unit_variant(node: &PdgNode) -> bool {
    let last = node.detail.rsplit("::").next().unwrap_or("");
    node.kind == NodeKind::Lit
        && node.lit_kind.as_deref() == Some("path")
        && node.callee_id.is_none()
        && last.chars().next().is_some_and(char::is_uppercase)
        && last.chars().any(char::is_lowercase)
}

/// Literal operands and ctor nodes, the only things a constant value is built from.
fn is_value_leaf(node: &PdgNode) -> bool {
    node.kind == NodeKind::Lit && node.callee_id.is_none()
}

/// True if `node` is a ctor/tuple whose whole input tree is literals and ctors owned by it.
fn constant_tree(pdg: &Pdg, node: usize) -> bool {
    is_ctor_op(&pdg.nodes[node])
        && data_producers(pdg, node).into_iter().all(|p| {
            let owned = data_consumers(pdg, p) == BTreeSet::from([node]);
            owned && (is_value_leaf(&pdg.nodes[p]) || constant_tree(pdg, p))
        })
}

/// `node` and everything it was built from.
fn built_from(pdg: &Pdg, node: usize) -> BTreeSet<usize> {
    data_producers(pdg, node)
        .into_iter()
        .flat_map(|p| built_from(pdg, p))
        .chain([node])
        .collect()
}

/// A constant tree is a root when no constant ctor consumes it: `Err(E::Bad("x"))` is one value.
fn constant_root(pdg: &Pdg, node: usize) -> bool {
    constant_tree(pdg, node)
        && !data_consumers(pdg, node)
            .into_iter()
            .any(|c| constant_tree(pdg, c))
}

/// Folds each constant value (ctor trees over literals, unit tuples) into one `Ctor` leaf and
/// turns unit-variant literals into `Ctor` too. Non-constant ctor chains merge into the
/// outermost ctor: `Some(Wrap(x))` and `Some(x)` become one node over `x`.
fn fold_ctors(pdg: Pdg) -> Pdg {
    let roots: Vec<usize> = (0..pdg.nodes.len())
        .filter(|&n| constant_root(&pdg, n))
        .collect();
    let owned: Vec<(usize, usize)> = roots
        .iter()
        .flat_map(|&r| {
            built_from(&pdg, r)
                .into_iter()
                .filter(move |&n| n != r)
                .map(move |n| (n, r))
        })
        .collect();
    let dead: BTreeSet<usize> = owned.iter().map(|(n, _)| *n).collect();

    // Control that reached an absorbed node now reaches the folded value.
    let moved = pdg.edges.iter().filter_map(|e| {
        let (_, root) = owned.iter().find(|(n, _)| *n == e.to)?;
        (e.kind == EdgeKind::Ctrl).then_some(PdgEdge { to: *root, ..*e })
    });
    let edges = dedup(pdg.edges.iter().copied().chain(moved).collect());
    let nodes: Vec<PdgNode> = pdg
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            if roots.contains(&i) || is_unit_variant(n) {
                as_ctor(n)
            } else {
                n.clone()
            }
        })
        .collect();
    let folded = without(Pdg { nodes, edges }, &dead);
    splice_nested(folded)
}

fn as_ctor(node: &PdgNode) -> PdgNode {
    PdgNode {
        kind: NodeKind::Ctor,
        detail: node.detail.clone(),
        ..node.clone()
    }
}

/// A ctor feeding only another ctor merges into it: its inputs become the parent's inputs.
fn splice_nested(pdg: Pdg) -> Pdg {
    let is_ctor = |n: usize| is_ctor_op(&pdg.nodes[n]);
    let inner: BTreeSet<usize> = (0..pdg.nodes.len())
        .filter(|&n| {
            let parents = data_consumers(&pdg, n);
            is_ctor(n) && parents.len() == 1 && parents.iter().all(|&p| is_ctor(p))
        })
        .collect();

    // Outermost ctor above `n` (chains of nested ctors merge all the way up).
    let parent_of = |n: usize| {
        let mut at = n;
        while inner.contains(&at) {
            at = data_consumers(&pdg, at).into_iter().next().unwrap_or(at);
        }
        at
    };
    let moved = pdg.edges.iter().filter_map(|e| {
        let into_inner = inner.contains(&e.to) && e.kind == EdgeKind::Data;
        let from_inner = inner.contains(&e.from);
        match (into_inner, from_inner) {
            (true, false) => Some(PdgEdge {
                to: parent_of(e.to),
                ..*e
            }),
            (false, false) => Some(*e),
            _ => None,
        }
    });
    let edges = dedup(moved.collect());
    without(
        Pdg {
            edges,
            ..pdg.clone()
        },
        &inner,
    )
}

// ---- R2: predicate collapse ----------------------------------------------------------------

fn is_combinator(node: &PdgNode) -> bool {
    node.kind == NodeKind::Op && matches!(node.detail.as_str(), "&&" | "||" | "!")
}

/// Combinator whose operands are calls or (recursively) such combinators.
fn predicate_tree(pdg: &Pdg, node: usize) -> bool {
    is_combinator(&pdg.nodes[node])
        && data_producers(pdg, node)
            .into_iter()
            .all(|p| pdg.nodes[p].kind == NodeKind::Call || predicate_tree(pdg, p))
}

/// Call leaves under `node`, each with whether an odd number of `!` sits above it.
fn leaves(pdg: &Pdg, node: usize, negated: bool) -> Vec<(usize, bool)> {
    let flip = negated ^ (pdg.nodes[node].detail == "!");
    data_producers(pdg, node)
        .into_iter()
        .flat_map(|p| {
            if is_combinator(&pdg.nodes[p]) {
                leaves(pdg, p, flip)
            } else {
                vec![(p, flip)]
            }
        })
        .collect()
}

/// `if !a.ok() && !b.ok()` and `if a.ok()` differ only by calls: the combinator nodes go, the
/// calls feed the consumer directly (negation survives as `detail: "!"` on the call).
fn collapse_preds(pdg: Pdg) -> Pdg {
    let trees: BTreeSet<usize> = (0..pdg.nodes.len())
        .filter(|&n| predicate_tree(&pdg, n))
        .collect();
    let roots: Vec<usize> = trees
        .iter()
        .copied()
        .filter(|&n| data_consumers(&pdg, n).is_disjoint(&trees))
        .collect();
    let found: Vec<(usize, bool)> = roots.iter().flat_map(|&r| leaves(&pdg, r, false)).collect();
    let negated: BTreeSet<usize> = found
        .iter()
        .filter(|(_, neg)| *neg)
        .map(|(n, _)| *n)
        .collect();
    let nodes: Vec<PdgNode> = pdg
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| match negated.contains(&i) {
            true => PdgNode {
                detail: "!".into(),
                ..n.clone()
            },
            false => n.clone(),
        })
        .collect();
    let rewired = roots.iter().flat_map(|&root| {
        let outs: Vec<PdgEdge> = pdg
            .edges
            .iter()
            .filter(|e| e.from == root && e.kind == EdgeKind::Data)
            .copied()
            .collect();
        leaves(&pdg, root, false)
            .into_iter()
            .flat_map(move |(leaf, _)| {
                outs.clone().into_iter().map(move |o| PdgEdge {
                    from: leaf,
                    arg_pos: 0,
                    ..o
                })
            })
            .collect::<Vec<_>>()
    });
    let edges = dedup(pdg.edges.iter().copied().chain(rewired).collect());

    without(Pdg { nodes, edges }, &trees)
}

// ---- R3: one multiway decision ---------------------------------------------------------------

fn is_branching(node: &PdgNode) -> bool {
    matches!(node.kind, NodeKind::Branch | NodeKind::Match)
}

/// `if let P = e {..}`: the destructuring `Let` feeding a `Branch` is the scrutinee of a match.
/// The `Let` goes away; its input and its bindings attach to the branch.
fn contract_lets(pdg: Pdg) -> Pdg {
    let pairs: Vec<(usize, usize)> = pdg
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Data)
        .filter(|e| {
            pdg.nodes[e.from].kind == NodeKind::Let && pdg.nodes[e.to].kind == NodeKind::Branch
        })
        .map(|e| (e.from, e.to))
        .collect();
    let branch_of = |l: usize| pairs.iter().find(|(x, _)| *x == l).map(|(_, b)| *b);
    let moved = pdg
        .edges
        .iter()
        .filter_map(|e| match (branch_of(e.from), branch_of(e.to)) {
            (Some(b), _) if e.to == b => None,
            (Some(b), _) => Some(PdgEdge { from: b, ..*e }),
            (None, Some(b)) if e.kind == EdgeKind::Data => Some(PdgEdge { to: b, ..*e }),
            (None, Some(_)) => None,
            (None, None) => Some(*e),
        });
    let dead = pairs.iter().map(|(l, _)| *l).collect();
    let edges = dedup(moved.collect());

    without(Pdg { edges, ..pdg }, &dead)
}

/// `if x == 3`: the comparison with a literal is the pattern of a match arm, not structure.
/// The branch is then a decision on `x` alone.
fn literal_scrutinees(pdg: Pdg) -> Pdg {
    let found: Vec<(usize, usize, usize, usize)> = (0..pdg.nodes.len())
        .filter(|&b| pdg.nodes[b].kind == NodeKind::Branch)
        .filter_map(|b| {
            let cond = data_producers(&pdg, b);
            let [op] = cond.as_slice() else { return None };
            let inputs = data_producers(&pdg, *op);
            let is_eq = pdg.nodes[*op].kind == NodeKind::Op && pdg.nodes[*op].detail == "==";
            let (lit, x) = match inputs.as_slice() {
                [p, q] if is_value_leaf(&pdg.nodes[*p]) => (*p, *q),
                [p, q] if is_value_leaf(&pdg.nodes[*q]) => (*q, *p),
                _ => return None,
            };
            let owned = data_consumers(&pdg, lit) == BTreeSet::from([*op]);
            let single_use = data_consumers(&pdg, *op) == BTreeSet::from([b]);
            (is_eq && owned && single_use && !is_value_leaf(&pdg.nodes[x]))
                .then_some((b, *op, lit, x))
        })
        .collect();
    let dead: BTreeSet<usize> = found
        .iter()
        .flat_map(|&(_, op, lit, _)| [op, lit])
        .collect();
    let scrutinee = found.iter().map(|&(b, _, _, x)| PdgEdge {
        from: x,
        to: b,
        kind: EdgeKind::Data,
        arg_pos: 0,
    });
    let edges = dedup(pdg.edges.iter().copied().chain(scrutinee).collect());

    without(Pdg { edges, ..pdg }, &dead)
}

fn arms_of(pdg: &Pdg, case: usize) -> Vec<&PdgEdge> {
    let outs = pdg.edges.iter();
    outs.filter(|e| e.from == case && e.kind == EdgeKind::Ctrl)
        .collect()
}

/// A case sitting alone in the last arm of another case on the same scrutinee: `(parent, child, arm)`.
fn else_if(pdg: &Pdg) -> Option<(usize, usize, usize)> {
    let is_case = |n: usize| pdg.nodes[n].kind == NodeKind::Case;
    (0..pdg.nodes.len()).filter(|&p| is_case(p)).find_map(|p| {
        let arms = arms_of(pdg, p);
        let last = arms.iter().map(|e| e.arg_pos).max()?;
        let tail: Vec<usize> = arms
            .iter()
            .filter(|e| e.arg_pos == last)
            .map(|e| e.to)
            .collect();
        let [c] = tail.as_slice() else { return None };
        let same_scrutinee =
            !data_producers(pdg, p).is_empty() && data_producers(pdg, p) == data_producers(pdg, *c);
        (is_case(*c) && same_scrutinee && data_consumers(pdg, *c).is_empty())
            .then_some((p, *c, last))
    })
}

/// `if x == 1 {..} else if x == 2 {..} else {..}` is one case with three arms.
fn flatten_else_if(pdg: Pdg) -> Pdg {
    let Some((parent, child, arm)) = else_if(&pdg) else {
        return pdg;
    };
    let kept = pdg
        .edges
        .iter()
        .filter(|e| e.from != child && e.to != child)
        .copied();
    let lifted = pdg
        .edges
        .iter()
        .filter(|e| e.from == child && e.kind == EdgeKind::Ctrl)
        .map(|e| PdgEdge {
            from: parent,
            arg_pos: arm + e.arg_pos,
            ..*e
        });
    let edges = dedup(kept.chain(lifted).collect());
    let merged = without(Pdg { edges, ..pdg }, &BTreeSet::from([child]));

    flatten_else_if(merged)
}

fn relabel_cases(pdg: Pdg) -> Pdg {
    let nodes = pdg.nodes.into_iter().map(|n| match is_branching(&n) {
        true => PdgNode {
            kind: NodeKind::Case,
            ..n
        },
        false => n,
    });
    Pdg {
        nodes: nodes.collect(),
        ..pdg
    }
}

/// `if`, `else if`, `if let` and `match` all become `Case`; patterns spelled as `== literal` and
/// else-if chains over one scrutinee disappear into the arms.
fn unify_case(pdg: Pdg) -> Pdg {
    let lets = contract_lets(pdg);
    let cases = relabel_cases(literal_scrutinees(lets));
    flatten_else_if(cases)
}

// ---- R4: exits -------------------------------------------------------------------------------

/// `unreachable!()` closes a `for` that always returns inside; the loop form has no such tail.
fn drop_unreachable(pdg: Pdg) -> Pdg {
    let dead = pdg
        .nodes
        .iter()
        .enumerate()
        .filter(|(i, n)| {
            n.kind == NodeKind::Macro
                && n.macro_name
                    .as_deref()
                    .is_some_and(|m| m.contains("unreachable"))
                && data_producers(&pdg, *i).is_empty()
        })
        .map(|(i, _)| i)
        .collect();
    without(pdg, &dead)
}

// ---- R8: error propagation -------------------------------------------------------------------

/// `(match, err ctor, return)` of every `match` whose only job besides yielding the `Ok` value is
/// `Err(e) => return Err(e)`: the arm holds exactly one ctor fed by the match, feeding one return.
fn propagations(pdg: &Pdg) -> Vec<(usize, usize, usize)> {
    let arms = |m: usize| -> BTreeSet<usize> {
        let outs = pdg.edges.iter();
        outs.filter(|e| e.from == m && e.kind == EdgeKind::Ctrl)
            .map(|e| e.to)
            .collect()
    };
    let kind = |n: usize| pdg.nodes[n].kind;
    let found = (0..pdg.nodes.len()).filter(|&m| kind(m) == NodeKind::Match);
    found
        .filter_map(|m| {
            let [c, r] = arms(m).into_iter().collect::<Vec<_>>()[..] else {
                return None;
            };
            let (c, r) = if kind(c) == NodeKind::Return {
                (r, c)
            } else {
                (c, r)
            };
            let shaped = is_ctor_op(&pdg.nodes[c]) && kind(r) == NodeKind::Return;
            let wired = data_producers(pdg, c) == [m]
                && data_consumers(pdg, c) == BTreeSet::from([r])
                && data_producers(pdg, r) == [c];
            (shaped && wired).then_some((m, c, r))
        })
        .collect()
}

/// `match r { Ok(v) => v, Err(e) => return Err(e) }` becomes the `Try` node (`r?`). The graph
/// shrinks by the ctor and the return; the match becomes the `Try`.
fn propagate_errors(pdg: Pdg) -> Pdg {
    let found = propagations(&pdg);
    let dead = found.iter().flat_map(|&(_, c, r)| [c, r]).collect();
    let nodes = pdg.nodes.iter().enumerate().map(|(i, n)| {
        let is_try = found.iter().any(|&(m, _, _)| m == i);
        match is_try {
            true => PdgNode {
                kind: NodeKind::Try,
                ..n.clone()
            },
            false => n.clone(),
        }
    });
    without(
        Pdg {
            nodes: nodes.collect(),
            ..pdg
        },
        &dead,
    )
}

// ---- R5-R7: loops and counters ---------------------------------------------------------------

const COMPARES: [&str; 6] = ["<", "<=", "==", ">=", ">", "!="];

fn is_int_lit(node: &PdgNode) -> bool {
    is_value_leaf(node) && node.lit_kind.as_deref() == Some("int")
}

fn is_compare(node: &PdgNode) -> bool {
    node.kind == NodeKind::Op && COMPARES.contains(&node.detail.as_str())
}

/// `n` is consumed by `consumer` only.
fn owned_by(pdg: &Pdg, n: usize, consumer: usize) -> bool {
    data_consumers(pdg, n) == BTreeSet::from([consumer])
}

/// `for i in 1..=3`: the iterate over a literal range is a `Loop`; the range call and its bounds go
/// (the bounds stay in `detail`, e.g. `1..3`).
fn range_loops(pdg: Pdg) -> Pdg {
    let found: Vec<(usize, usize, Vec<usize>)> = (0..pdg.nodes.len())
        .filter(|&i| pdg.nodes[i].kind == NodeKind::Iterate)
        .filter_map(|i| {
            let producers = data_producers(&pdg, i);
            let [call] = producers.as_slice() else {
                return None;
            };
            let call = *call;
            let bounds = data_producers(&pdg, call);
            let range = pdg.nodes[call]
                .callee_id
                .as_deref()
                .is_some_and(|id| id.contains("ops::range::"));
            let literal = bounds.len() == 2
                && bounds
                    .iter()
                    .all(|&b| is_int_lit(&pdg.nodes[b]) && owned_by(&pdg, b, call));
            (range && literal && owned_by(&pdg, call, i)).then_some((i, call, bounds))
        })
        .collect();
    let dead: BTreeSet<usize> = found
        .iter()
        .flat_map(|(_, call, bounds)| bounds.iter().chain([call]).copied())
        .collect();
    let nodes: Vec<PdgNode> = pdg
        .nodes
        .iter()
        .enumerate()
        .map(|(n, node)| match found.iter().find(|(i, ..)| *i == n) {
            Some((_, _, bounds)) => PdgNode {
                kind: NodeKind::Loop,
                detail: bounds
                    .iter()
                    .map(|&b| pdg.nodes[b].detail.clone())
                    .collect::<Vec<_>>()
                    .join(".."),
                ..node.clone()
            },
            None => node.clone(),
        })
        .collect();
    without(Pdg { nodes, ..pdg }, &dead)
}

/// The nearest `Loop` above `node` in control dependence (a `while` guard sits between).
fn enclosing_loop(pdg: &Pdg, node: usize) -> Option<usize> {
    let mut at = node;
    for _ in 0..4 {
        at = pdg
            .edges
            .iter()
            .find(|e| e.to == at && e.kind == EdgeKind::Ctrl)?
            .from;
        if pdg.nodes[at].kind == NodeKind::Loop {
            return Some(at);
        }
    }
    None
}

/// A manual counter in a loop: `(init literal, step op, step literal, loop)` for
/// `let mut n = 0; loop { n += 1; .. }`.
fn manual_counter(pdg: &Pdg, step: usize) -> Option<(usize, usize, usize)> {
    let is_step_op = pdg.nodes[step].kind == NodeKind::Op
        && matches!(pdg.nodes[step].detail.as_str(), "+=" | "+==");
    let inputs = pdg
        .edges
        .iter()
        .filter(|e| e.to == step && e.kind == EdgeKind::Data);
    let by_pos = |pos: usize| inputs.clone().find(|e| e.arg_pos == pos).map(|e| e.from);
    let (amount, init) = (by_pos(0)?, by_pos(1)?);
    let looped = enclosing_loop(pdg, step)?;
    let ok = is_step_op
        && is_int_lit(&pdg.nodes[amount])
        && is_int_lit(&pdg.nodes[init])
        && owned_by(pdg, amount, step)
        && data_consumers(pdg, step).is_empty();

    ok.then_some((init, amount, looped))
}

/// Adds `extra` edges, then removes `dead` nodes with every edge touching them.
fn rewire(pdg: Pdg, extra: Vec<PdgEdge>, dead: &BTreeSet<usize>) -> Pdg {
    let edges = dedup(pdg.edges.iter().copied().chain(extra).collect());
    without(Pdg { edges, ..pdg }, dead)
}

/// The loop node stands for the counter: whatever read `n` reads the loop, and the
/// initialisation and the `n += 1` step disappear.
fn manual_counters(pdg: Pdg) -> Pdg {
    let found: Vec<(usize, usize, usize, usize)> = (0..pdg.nodes.len())
        .filter_map(|step| {
            let (init, amount, looped) = manual_counter(&pdg, step)?;
            Some((step, init, amount, looped))
        })
        .collect();
    let counter_of = |n: usize| found.iter().find(|f| f.1 == n).map(|f| f.3);
    let reads = pdg
        .edges
        .iter()
        .filter_map(|e| {
            let looped = counter_of(e.from)?;
            let step = found.iter().any(|f| f.0 == e.to);
            (e.kind == EdgeKind::Data && !step).then_some(PdgEdge { from: looped, ..*e })
        })
        .collect();
    let dead = found.iter().flat_map(|&(s, i, a, _)| [s, i, a]).collect();

    rewire(pdg, reads, &dead)
}

/// `while n < 3 { body }` where `n` is a counter loop: the guard is the loop bound, so the body
/// hangs directly under the loop, as in `for n in 1..=3`.
fn loop_guards(pdg: Pdg) -> Pdg {
    let guard = |g: usize| -> Option<(usize, usize, usize)> {
        let is_case = matches!(pdg.nodes[g].kind, NodeKind::Case | NodeKind::Branch);
        let owner = pdg
            .edges
            .iter()
            .find(|e| e.to == g && e.kind == EdgeKind::Ctrl)?
            .from;
        let cond = data_producers(&pdg, g);
        let [op] = cond.as_slice() else { return None };
        let inputs = data_producers(&pdg, *op);
        let bound = inputs
            .iter()
            .copied()
            .find(|&n| is_int_lit(&pdg.nodes[n]))?;
        let counts = inputs.contains(&owner) && inputs.len() == 2;
        let ok = is_case
            && pdg.nodes[owner].kind == NodeKind::Loop
            && counts
            && is_compare(&pdg.nodes[*op])
            && owned_by(&pdg, bound, *op)
            && owned_by(&pdg, *op, g)
            && arms_of(&pdg, g).iter().all(|e| e.arg_pos == 0);
        ok.then_some((owner, *op, bound))
    };
    let found: Vec<(usize, usize, usize, usize)> = (0..pdg.nodes.len())
        .filter_map(|g| guard(g).map(|(owner, op, bound)| (g, owner, op, bound)))
        .collect();
    let body = pdg
        .edges
        .iter()
        .filter_map(|e| {
            let (_, owner, ..) = found.iter().find(|f| f.0 == e.from)?;
            (e.kind == EdgeKind::Ctrl).then_some(PdgEdge { from: *owner, ..*e })
        })
        .collect();
    let dead = found.iter().flat_map(|&(g, _, op, b)| [g, op, b]).collect();

    rewire(pdg, body, &dead)
}

fn counter_loops(pdg: Pdg) -> Pdg {
    loop_guards(manual_counters(range_loops(pdg)))
}

/// `i < 3` and `i == 3` on a counter compare it with a literal bound: one operator class.
/// Only when the other operand is a literal (otherwise the operators mean different things).
fn compare_class(pdg: Pdg) -> Pdg {
    let counted = |n: usize| {
        let inputs = data_producers(&pdg, n);
        let counter = inputs.iter().any(|&p| pdg.nodes[p].kind == NodeKind::Loop);
        let bound = inputs.iter().any(|&p| is_int_lit(&pdg.nodes[p]));
        is_compare(&pdg.nodes[n]) && inputs.len() == 2 && counter && bound
    };
    let nodes: Vec<PdgNode> = (0..pdg.nodes.len())
        .map(|n| match counted(n) {
            true => PdgNode {
                detail: format!("cmp:{}", pdg.nodes[n].detail),
                ..pdg.nodes[n].clone()
            },
            false => pdg.nodes[n].clone(),
        })
        .collect();
    Pdg { nodes, ..pdg }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pdgtest::*;

    fn op(detail: &str) -> PdgNode {
        PdgNode {
            detail: detail.into(),
            ..node(NodeKind::Op)
        }
    }

    fn lit(kind: &str, text: &str) -> PdgNode {
        PdgNode {
            lit_kind: Some(kind.into()),
            detail: text.into(),
            ..node(NodeKind::Lit)
        }
    }

    fn kinds(pdg: &Pdg) -> Vec<NodeKind> {
        pdg.nodes.iter().map(|n| n.kind).collect()
    }

    /// Value rules only, so these tests do not depend on the control-flow rules.
    fn all() -> Rules {
        Rules::parse("ctor,pred").unwrap()
    }

    /// `return Err(Bad("x"))`
    fn nested_constant() -> Pdg {
        Pdg {
            nodes: vec![
                lit("str", "\"x\""),
                op("ctor"),
                op("ctor"),
                node(NodeKind::Return),
            ],
            edges: vec![data(0, 1, 0), data(1, 2, 0), data(2, 3, 0)],
        }
    }

    #[test]
    fn constant_ctor_tree_becomes_one_ctor() {
        let out = canonicalize(&nested_constant(), &all());
        assert_eq!(kinds(&out), vec![NodeKind::Ctor, NodeKind::Return]);
        assert_eq!(out.edges, vec![data(0, 1, 0)]);
    }

    #[test]
    fn control_edges_into_absorbed_nodes_move_to_the_folded_ctor() {
        let mut pdg = nested_constant();
        pdg.nodes.insert(0, node(NodeKind::Branch));
        pdg.edges = vec![ctrl(0, 2, 0), data(1, 2, 0), data(2, 3, 0), data(3, 4, 0)];
        let out = canonicalize(&pdg, &all());
        assert_eq!(
            kinds(&out),
            vec![NodeKind::Branch, NodeKind::Ctor, NodeKind::Return]
        );
        assert!(out.edges.contains(&ctrl(0, 1, 0)));
        assert!(out.edges.contains(&data(1, 2, 0)));
        assert_eq!(out.edges.len(), 2);
    }

    #[test]
    fn unit_variant_literal_and_ok_unit_are_ctors() {
        let pdg = Pdg {
            nodes: vec![
                lit("path", "Mode::Fast"),
                node(NodeKind::Return),
                op("tuple"),
                op("ctor"),
            ],
            edges: vec![data(0, 1, 0), data(2, 3, 0), data(3, 1, 1)],
        };
        let out = canonicalize(&pdg, &all());
        assert_eq!(
            kinds(&out),
            vec![NodeKind::Ctor, NodeKind::Return, NodeKind::Ctor]
        );
        assert_eq!(out.nodes[0].detail, "Mode::Fast", "hole text survives");
    }

    #[test]
    fn plain_path_literals_stay_literals() {
        let pdg = Pdg {
            nodes: vec![lit("path", "LIMIT"), node(NodeKind::Return)],
            edges: vec![data(0, 1, 0)],
        };
        assert_eq!(canonicalize(&pdg, &all()), pdg);
    }

    #[test]
    fn non_constant_ctor_chain_merges_into_the_outer_ctor() {
        let pdg = Pdg {
            nodes: vec![push_call(), op("ctor"), op("ctor"), node(NodeKind::Return)],
            edges: vec![data(0, 1, 0), data(1, 2, 0), data(2, 3, 0)],
        };
        let out = canonicalize(&pdg, &all());
        assert_eq!(
            kinds(&out),
            vec![NodeKind::Call, NodeKind::Op, NodeKind::Return]
        );
        assert_eq!(out.edges, vec![data(0, 1, 0), data(1, 2, 0)]);
    }

    #[test]
    fn literal_shared_with_a_call_is_not_folded_into_the_ctor() {
        let pdg = Pdg {
            nodes: vec![lit("str", "\"x\""), op("ctor"), push_call()],
            edges: vec![data(0, 1, 0), data(0, 2, 1)],
        };
        assert_eq!(canonicalize(&pdg, &all()).nodes.len(), 3);
    }

    /// `if !a.ok() && !b.ok() {..}`
    fn negated_pair() -> Pdg {
        Pdg {
            nodes: vec![
                getter_call("A", "ok"),
                getter_call("B", "ok"),
                op("!"),
                op("!"),
                op("&&"),
                node(NodeKind::Branch),
            ],
            edges: vec![
                data(0, 2, 0),
                data(1, 3, 0),
                data(2, 4, 0),
                data(3, 4, 1),
                data(4, 5, 0),
            ],
        }
    }

    #[test]
    fn negated_conjunction_leaves_two_calls_feeding_the_branch() {
        let out = canonicalize(&negated_pair(), &all());
        assert_eq!(
            kinds(&out),
            vec![NodeKind::Call, NodeKind::Call, NodeKind::Branch]
        );
        assert_eq!(out.edges, vec![data(0, 2, 0), data(1, 2, 0)]);
        assert!(out.nodes[..2].iter().all(|n| n.detail == "!"));
    }

    #[test]
    fn single_call_predicate_matches_its_negated_form_but_for_the_flag() {
        let plain = Pdg {
            nodes: vec![getter_call("A", "ok"), node(NodeKind::Branch)],
            edges: vec![data(0, 1, 0)],
        };
        let negated = Pdg {
            nodes: vec![getter_call("A", "ok"), op("!"), node(NodeKind::Branch)],
            edges: vec![data(0, 1, 0), data(1, 2, 0)],
        };
        let out = canonicalize(&negated, &all());
        assert_eq!(kinds(&out), kinds(&plain));
        assert_eq!(out.edges, plain.edges);
    }

    #[test]
    fn combinators_over_non_calls_are_left_alone() {
        let pdg = Pdg {
            nodes: vec![lit("bool", "true"), op("!"), node(NodeKind::Branch)],
            edges: vec![data(0, 1, 0), data(1, 2, 0)],
        };
        assert_eq!(canonicalize(&pdg, &all()), pdg);
    }

    #[test]
    fn rules_are_idempotent_and_never_grow_the_graph() {
        let fixtures = [
            nested_constant(),
            negated_pair(),
            emit_loop("Dog", "bark"),
            emit_loop("Cat", "meow"),
        ];
        for pdg in &fixtures {
            let once = canonicalize(pdg, &all());
            assert_eq!(canonicalize(&once, &all()), once);
            assert!(once.nodes.len() <= pdg.nodes.len());
            assert!(once.edges.len() <= pdg.edges.len() + 2);
        }
    }

    #[test]
    fn graphs_without_matching_shapes_are_unchanged() {
        let pdg = emit_loop("Dog", "bark");
        assert_eq!(canonicalize(&pdg, &all()), pdg);
    }

    #[test]
    fn no_rules_is_the_identity() {
        let pdg = negated_pair();
        assert_eq!(canonicalize(&pdg, &Rules::default()), pdg);
    }

    #[test]
    fn default_normalization_borrows_the_original_graph() {
        let graph = negated_pair();
        let normalized = canonicalize_or_borrow(&graph, &Rules::default());
        assert!(matches!(normalized, Cow::Borrowed(_)));
        assert!(std::ptr::eq(normalized.as_ref(), &graph));
    }

    #[test]
    fn enabled_normalization_owns_rewrites_without_changing_the_input() {
        let original = nested_constant();
        let graph = original.clone();
        let normalized = canonicalize_or_borrow(&graph, &all());
        assert!(matches!(normalized, Cow::Owned(_)));
        assert!(normalized.nodes.len() < graph.nodes.len());
        assert_eq!(normalized.as_ref(), &canonicalize(&graph, &all()));
        assert_eq!(graph, original);
    }

    #[test]
    fn rules_parse_names_and_reject_unknown_ones() {
        assert_eq!(
            Rules::parse("ctor").unwrap(),
            Rules {
                ctor: true,
                ..Rules::default()
            }
        );
        assert_eq!(
            Rules::parse("ctor, pred,case,exit,counter,cmp,try").unwrap(),
            Rules::ALL
        );
        assert_eq!(Rules::parse("all").unwrap(), Rules::ALL);
        assert!(Rules::parse("").unwrap().is_none());
        assert!(Rules::parse("bogus").is_err());
    }

    #[test]
    fn rewritten_graphs_keep_valid_indices() {
        let out = canonicalize(&negated_pair(), &all());
        let n = out.nodes.len();
        assert!(out.edges.iter().all(|e| e.from < n && e.to < n));
    }

    fn sorted_edges(pdg: &Pdg) -> Vec<PdgEdge> {
        let mut edges = pdg.edges.clone();
        edges.sort_by_key(|e| (e.from, e.to, e.kind, e.arg_pos));
        edges
    }

    fn same_graph(a: &Pdg, b: &Pdg) -> bool {
        a.nodes == b.nodes && sorted_edges(a) == sorted_edges(b)
    }

    fn case_rules() -> Rules {
        Rules::parse("case").unwrap()
    }

    /// `if let Some(v) = p.find() { hit(v) } else { miss() }`
    fn if_let() -> Pdg {
        Pdg {
            nodes: vec![
                node(NodeKind::Param),
                getter_call("R", "find"),
                node(NodeKind::Let),
                node(NodeKind::Branch),
                getter_call("R", "hit"),
                getter_call("R", "miss"),
                node(NodeKind::Return),
            ],
            edges: vec![
                data(0, 1, 0),
                data(1, 2, 0),
                data(2, 3, 0),
                data(2, 4, 1),
                ctrl(3, 4, 0),
                ctrl(3, 5, 1),
                data(3, 6, 0),
            ],
        }
    }

    /// `match p.find() { Some(v) => hit(v), None => miss() }`
    fn two_arm_match() -> Pdg {
        Pdg {
            nodes: vec![
                node(NodeKind::Param),
                getter_call("R", "find"),
                node(NodeKind::Match),
                getter_call("R", "hit"),
                getter_call("R", "miss"),
                node(NodeKind::Return),
            ],
            edges: vec![
                data(0, 1, 0),
                data(1, 2, 0),
                data(2, 3, 1),
                ctrl(2, 3, 0),
                ctrl(2, 4, 1),
                data(2, 5, 0),
            ],
        }
    }

    #[test]
    fn if_let_and_match_become_the_same_case() {
        let (a, b) = (
            canonicalize(&if_let(), &case_rules()),
            canonicalize(&two_arm_match(), &case_rules()),
        );
        assert!(same_graph(&a, &b), "{a:?}\n{b:?}");
        assert_eq!(a.nodes[2].kind, NodeKind::Case);
    }

    /// `if code == 200 { a } else if code == 404 { b } else { c }` (three arms)
    fn literal_chain() -> Pdg {
        let eq = op("==");
        Pdg {
            nodes: vec![
                node(NodeKind::Param),
                node(NodeKind::Branch),
                eq.clone(),
                lit("int", "200"),
                getter_call("R", "a"),
                node(NodeKind::Branch),
                eq,
                lit("int", "404"),
                getter_call("R", "b"),
                getter_call("R", "c"),
                node(NodeKind::Return),
            ],
            edges: vec![
                data(0, 2, 0),
                data(3, 2, 1),
                data(2, 1, 0),
                ctrl(1, 4, 0),
                ctrl(1, 5, 1),
                ctrl(1, 6, 1),
                ctrl(1, 7, 1),
                data(0, 6, 0),
                data(7, 6, 1),
                data(6, 5, 0),
                ctrl(5, 8, 0),
                ctrl(5, 9, 1),
                data(1, 10, 0),
            ],
        }
    }

    fn match_on_code() -> Pdg {
        Pdg {
            nodes: vec![
                node(NodeKind::Param),
                node(NodeKind::Match),
                getter_call("R", "a"),
                getter_call("R", "b"),
                getter_call("R", "c"),
                node(NodeKind::Return),
            ],
            edges: vec![
                data(0, 1, 0),
                ctrl(1, 2, 0),
                ctrl(1, 3, 1),
                ctrl(1, 4, 2),
                data(1, 5, 0),
            ],
        }
    }

    #[test]
    fn literal_else_if_chain_equals_match_on_the_scrutinee() {
        let (a, b) = (
            canonicalize(&literal_chain(), &case_rules()),
            canonicalize(&match_on_code(), &case_rules()),
        );
        assert!(same_graph(&a, &b), "{a:?}\n{b:?}");
    }

    #[test]
    fn else_if_on_a_different_scrutinee_stays_nested() {
        let mut pdg = literal_chain();
        pdg.nodes.push(node(NodeKind::Param));
        let other = pdg.nodes.len() - 1;
        pdg.edges.retain(|e| !(e.from == 0 && e.to == 6));
        pdg.edges.push(data(other, 6, 0));
        let out = canonicalize(&pdg, &case_rules());
        let cases = out.nodes.iter().filter(|n| n.kind == NodeKind::Case);
        assert_eq!(cases.count(), 2);
    }

    #[test]
    fn plain_boolean_branch_becomes_a_case_without_losing_its_condition() {
        let pdg = Pdg {
            nodes: vec![getter_call("R", "ok"), node(NodeKind::Branch)],
            edges: vec![data(0, 1, 0)],
        };
        let out = canonicalize(&pdg, &case_rules());
        assert_eq!(kinds(&out), vec![NodeKind::Call, NodeKind::Case]);
        assert_eq!(out.edges, pdg.edges);
    }

    #[test]
    fn unreachable_tail_is_dropped_but_a_macro_with_inputs_is_kept() {
        let macro_node = |name: &str| PdgNode {
            macro_name: Some(name.into()),
            ..node(NodeKind::Macro)
        };
        let pdg = Pdg {
            nodes: vec![
                node(NodeKind::Param),
                macro_node("$crate::panic::unreachable_2021"),
            ],
            edges: vec![],
        };
        let rules = Rules::parse("exit").unwrap();
        assert_eq!(canonicalize(&pdg, &rules).nodes.len(), 1);
        let used = Pdg {
            edges: vec![data(0, 1, 0)],
            ..pdg
        };
        assert_eq!(canonicalize(&used, &rules), used);
    }

    #[test]
    fn case_rules_are_idempotent() {
        for pdg in [if_let(), two_arm_match(), literal_chain(), match_on_code()] {
            let once = canonicalize(&pdg, &Rules::ALL);
            assert_eq!(canonicalize(&once, &Rules::ALL), once);
            assert!(once.nodes.len() <= pdg.nodes.len());
        }
    }

    fn counter_rules() -> Rules {
        Rules::parse("counter,cmp").unwrap()
    }

    fn range_call() -> PdgNode {
        PdgNode {
            callee_id: Some("core::ops::range::{impl#7}::new".into()),
            ..node(NodeKind::Call)
        }
    }

    /// `for n in 1..=3 { body(n) }`
    fn range_form() -> Pdg {
        Pdg {
            nodes: vec![
                node(NodeKind::Iterate),
                range_call(),
                lit("int", "1"),
                lit("int", "3"),
                getter_call("R", "body"),
            ],
            edges: vec![
                data(2, 1, 0),
                data(3, 1, 1),
                data(1, 0, 0),
                data(0, 4, 1),
                ctrl(0, 4, 0),
            ],
        }
    }

    /// `let mut n = 0; while n < 3 { n += 1; body(n) }`
    fn while_form() -> Pdg {
        let loop_node = PdgNode {
            detail: "while".into(),
            ..node(NodeKind::Loop)
        };
        Pdg {
            nodes: vec![
                lit("int", "0"),
                loop_node,
                node(NodeKind::Branch),
                op("<"),
                lit("int", "3"),
                op("+=="),
                lit("int", "1"),
                getter_call("R", "body"),
            ],
            edges: vec![
                data(0, 3, 0),
                data(4, 3, 1),
                data(3, 2, 0),
                data(0, 5, 1),
                data(6, 5, 0),
                data(0, 7, 1),
                ctrl(1, 2, 0),
                ctrl(1, 3, 0),
                ctrl(1, 4, 0),
                ctrl(2, 5, 0),
                ctrl(2, 6, 0),
                ctrl(2, 7, 0),
            ],
        }
    }

    #[test]
    fn range_for_loop_becomes_a_loop_and_keeps_its_bounds_in_detail() {
        let out = canonicalize(&range_form(), &counter_rules());
        assert_eq!(kinds(&out), vec![NodeKind::Loop, NodeKind::Call]);
        assert_eq!(out.nodes[0].detail, "1..3");
    }

    #[test]
    fn while_with_manual_counter_equals_the_range_loop() {
        let (w, r) = (
            canonicalize(&while_form(), &counter_rules()),
            canonicalize(&range_form(), &counter_rules()),
        );
        assert_eq!(kinds(&w), kinds(&r));
        assert_eq!(sorted_edges(&w), sorted_edges(&r));
        assert_eq!(sorted_edges(&w), vec![data(0, 1, 1), ctrl(0, 1, 0)]);
    }

    #[test]
    fn loop_over_a_collection_is_not_a_counter_loop() {
        let pdg = emit_loop("Dog", "bark");
        assert_eq!(canonicalize(&pdg, &counter_rules()), pdg);
    }

    #[test]
    fn range_with_a_non_literal_bound_stays_an_iterate() {
        let mut pdg = range_form();
        pdg.nodes[3] = node(NodeKind::Param);
        let out = canonicalize(&pdg, &counter_rules());
        assert_eq!(out.nodes[0].kind, NodeKind::Iterate);
    }

    #[test]
    fn counter_comparison_has_one_class_whatever_the_operator() {
        let with = |operator: &str| {
            let pdg = Pdg {
                nodes: vec![
                    node(NodeKind::Loop),
                    lit("int", "3"),
                    op(operator),
                    node(NodeKind::Branch),
                ],
                edges: vec![data(0, 2, 0), data(1, 2, 1), data(2, 3, 0)],
            };
            canonicalize(&pdg, &counter_rules())
        };
        let (lt, eq) = (with("<"), with("=="));
        assert_eq!(lt.edges, eq.edges);
        assert_eq!(
            crate::wl::label(&lt.nodes[2]),
            crate::wl::label(&eq.nodes[2])
        );
        // the original operator survives as the hole value
        assert_eq!(lt.nodes[2].detail, "cmp:<");
        assert_eq!(eq.nodes[2].detail, "cmp:==");
    }

    #[test]
    fn comparison_of_a_non_counter_is_left_alone() {
        let pdg = Pdg {
            nodes: vec![
                node(NodeKind::Param),
                lit("int", "3"),
                op("<"),
                node(NodeKind::Branch),
            ],
            edges: vec![data(0, 2, 0), data(1, 2, 1), data(2, 3, 0)],
        };
        assert_eq!(canonicalize(&pdg, &counter_rules()), pdg);
    }

    #[test]
    fn counter_rules_are_idempotent_and_shrink() {
        for pdg in [range_form(), while_form()] {
            let once = canonicalize(&pdg, &Rules::ALL);
            assert_eq!(canonicalize(&once, &Rules::ALL), once);
            assert!(once.nodes.len() < pdg.nodes.len());
        }
    }

    /// `match call(p) { Ok(v) => v, Err(e) => return Err(e) }` then `use(v)`.
    fn explicit_propagation() -> Pdg {
        Pdg {
            nodes: vec![
                node(NodeKind::Param),
                node(NodeKind::Match),
                node(NodeKind::Return),
                op("ctor"),
                node(NodeKind::Macro),
            ],
            edges: vec![
                data(0, 1, 0),
                data(1, 3, 0),
                ctrl(1, 3, 1),
                ctrl(1, 2, 1),
                data(3, 2, 0),
                data(1, 4, 0),
            ],
        }
    }

    fn try_rule() -> Rules {
        Rules::parse("try").unwrap()
    }

    #[test]
    fn explicit_error_propagation_becomes_try() {
        let out = canonicalize(&explicit_propagation(), &try_rule());
        assert_eq!(
            kinds(&out),
            vec![NodeKind::Param, NodeKind::Try, NodeKind::Macro]
        );
        assert_eq!(out.edges, vec![data(0, 1, 0), data(1, 2, 0)]);
    }

    #[test]
    fn match_with_other_arm_work_is_not_a_try() {
        let mut g = explicit_propagation();
        g.nodes.push(node(NodeKind::Call));
        g.edges.push(ctrl(1, 5, 0));
        assert_eq!(canonicalize(&g, &try_rule()), g);
    }

    #[test]
    fn returning_something_else_is_not_a_try() {
        let mut g = explicit_propagation();
        g.edges
            .retain(|e| !(e.from == 1 && e.to == 3 && e.kind == EdgeKind::Data));
        g.edges.push(data(0, 3, 0));
        assert_eq!(canonicalize(&g, &try_rule()), g);
    }

    #[test]
    fn propagation_rule_is_idempotent_and_shrinks() {
        let once = canonicalize(&explicit_propagation(), &try_rule());
        assert_eq!(canonicalize(&once, &try_rule()), once);
        assert!(once.nodes.len() < explicit_propagation().nodes.len());
    }
}
